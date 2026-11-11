//! `StoreEnv` — backend واقعی orchestrator بات روی `V3Store` (فاز ۵).
//!
//! معناشناسی عیناً همان [`MemoryDomain`] بات است، اما روی SQLite واقعی با
//! الگوی load → دامنهٔ خالص → persist فاز ۲:
//! * replay: `V3Store::apply` خودش پیش‌چک کلید یکسان‌سازی را انجام می‌دهد —
//!   retry با همان `message_id` هرگز دوبار پول جابه‌جا نمی‌کند؛
//! * مادهٔ ۷: خطای `REQUIRES_HUMAN_APPROVAL` از دامنه به صف HITL سرور می‌رسد
//!   (`enqueue_hitl` — شناسهٔ دترمیینیستیک، replay-safe مثل هندلر فاز ۴)؛
//! * تأیید HITL از مسیر `approve_hitl_and_settle` فاز ۲ — تسویهٔ اتمیک ۹۰/۱۰
//!   در همان تراکنش با کلید قطعی `hitl:<id>:approve`؛ تصمیم دوم → تایپ‌شده.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use dropagentx_bot::{
    user_account, BotError, EconomyItem, Env, EscrowItem, HistoryItem, HitlItem, HitlOutcome,
    Locale, ProductItem, Transition, TransitionOutcome,
};
use dropagentx_db::{
    now_iso, ApplyOptions, ApplyResult, CommandOutcome, HitlRow, ProductFilter, V3Store,
};
use dropagentx_domain::{
    cancel_escrow, create_escrow, deliver_escrow, dispute_escrow, fund_escrow, release_escrow,
    split_seller_tax, CreateEscrowCommand, DeliverEscrowCommand, DisputeEscrowCommand,
    DomainErrorCode, EscrowRefCommand, EscrowStatus, ReleaseEscrowCommand,
};

/// backend بات روی فروشگاه واقعی — ارزان برای ساخت به ازای هر درخواست.
#[derive(Debug, Clone)]
pub struct StoreEnv {
    /// فروشگاه v3 (Clone ارزان — pool مشترک)
    store: V3Store,
    /// انتخاب صریح زبان کاربران — بین درخواست‌ها می‌ماند
    sessions: Arc<Mutex<BTreeMap<i64, Locale>>>,
    /// شناسه‌های ادمین — از `BOT_ADMIN_IDS` بوت
    admins: Arc<Vec<i64>>,
}

impl StoreEnv {
    /// ساخت از وضعیت سرور — sessions و ادمین‌ها از AppState.
    #[must_use]
    pub fn from_state(state: &crate::state::AppState) -> Self {
        Self {
            store: state.store.clone(),
            sessions: state.bot_sessions.clone(),
            admins: Arc::new(state.config.bot_admin_ids.clone()),
        }
    }

    /// نگاشت خطای فروشگاه → خطای بات (کد دامنه حفظ می‌شود).
    fn map_db(e: dropagentx_db::DbError) -> BotError {
        match e {
            dropagentx_db::DbError::Domain(err) => BotError::Domain(err.code),
            other => {
                tracing::error!(error = %other, "bot backend failure");
                BotError::Backend
            }
        }
    }

    async fn load_escrow(&self, escrow_id: &str) -> Result<dropagentx_domain::Escrow, BotError> {
        self.store
            .get_escrow(escrow_id)
            .await
            .map_err(Self::map_db)?
            .ok_or(BotError::EscrowNotFound)
    }

    fn item_of(e: &dropagentx_domain::Escrow) -> EscrowItem {
        EscrowItem {
            id: e.id.clone(),
            buyer_id: e.buyer_id.clone(),
            seller_id: e.seller_id.clone(),
            amount: e.amount.as_i64(),
            status: e.status.as_str().to_owned(),
            product_ref: e.product_ref.clone(),
        }
    }

    /// خروجی apply → خروجی گذار بات (زیر آستانه: تسویهٔ مستقیم).
    fn settled_outcome(res: &ApplyResult) -> TransitionOutcome {
        let escrow = res.escrow.as_ref().map(Self::item_of);
        let mut out = TransitionOutcome {
            escrow: escrow.unwrap_or_else(|| EscrowItem {
                id: String::new(),
                buyer_id: String::new(),
                seller_id: String::new(),
                amount: 0,
                status: "UNKNOWN".into(),
                product_ref: None,
            }),
            replayed: res.replayed,
            hitl_queued: None,
            tx_id: res.tx.as_ref().map(|t| t.id.clone()),
            seller_net: None,
            tax: None,
        };
        if !res.replayed {
            if let Ok(split) = split_seller_tax(e_amount(res)) {
                out.seller_net = Some(split.seller.as_i64());
                out.tax = Some(split.tax.as_i64());
            }
        }
        out
    }
}

fn e_amount(res: &ApplyResult) -> dropagentx_domain::Din {
    res.escrow.as_ref().map(|e| e.amount).unwrap_or(dropagentx_domain::Din::ZERO)
}

impl Env for StoreEnv {
    fn locale(&self, user: i64, hint: Option<&str>) -> Locale {
        if let Ok(map) = self.sessions.lock() {
            if let Some(l) = map.get(&user) {
                return *l;
            }
        }
        Locale::from_language_code(hint)
    }

    fn set_locale(&self, user: i64, locale: Locale) {
        if let Ok(mut map) = self.sessions.lock() {
            map.insert(user, locale);
        }
    }

    fn is_admin(&self, user: i64) -> bool {
        self.admins.contains(&user)
    }

    async fn balance(&self, user: i64) -> Result<i64, BotError> {
        let account = user_account(user);
        match self.store.balance_of(&account).await.map_err(Self::map_db)? {
            Some(b) => Ok(b),
            None => Err(BotError::Domain(DomainErrorCode::UnknownAccount)),
        }
    }

    async fn history(&self, user: i64, limit: usize) -> Result<Vec<HistoryItem>, BotError> {
        let account = user_account(user);
        if self.store.balance_of(&account).await.map_err(Self::map_db)?.is_none() {
            return Err(BotError::Domain(DomainErrorCode::UnknownAccount));
        }
        let rows = self
            .store
            .recent_postings_for(&account, i64::try_from(limit).unwrap_or(10))
            .await
            .map_err(Self::map_db)?;
        Ok(rows
            .into_iter()
            .map(|p| HistoryItem { tx_id: p.tx_id, amount: p.amount, memo: p.memo, at: p.at })
            .collect())
    }

    async fn products(&self, limit: usize) -> Result<Vec<ProductItem>, BotError> {
        let rows = self
            .store
            .list_products(&ProductFilter {
                q: None,
                limit: i64::try_from(limit).unwrap_or(10),
                offset: 0,
                active_only: true,
            })
            .await
            .map_err(Self::map_db)?;
        Ok(rows
            .into_iter()
            .map(|p| ProductItem {
                id: p.id,
                seller_id: p.seller_id,
                title: p.title,
                price_din: p.price_din,
            })
            .collect())
    }

    async fn escrow(&self, escrow_id: &str) -> Result<Option<EscrowItem>, BotError> {
        Ok(self
            .store
            .get_escrow(escrow_id)
            .await
            .map_err(Self::map_db)?
            .as_ref()
            .map(Self::item_of))
    }

    async fn open_escrows_of(&self, user: i64) -> Result<Vec<EscrowItem>, BotError> {
        let account = user_account(user);
        let open = [
            EscrowStatus::Created,
            EscrowStatus::Funded,
            EscrowStatus::Delivered,
            EscrowStatus::Disputed,
        ];
        Ok(self
            .store
            .list_escrows(None)
            .await
            .map_err(Self::map_db)?
            .iter()
            .filter(|e| {
                (e.buyer_id == account || e.seller_id == account) && open.contains(&e.status)
            })
            .map(Self::item_of)
            .collect())
    }

    async fn buy(
        &mut self,
        user: i64,
        chat_id: i64,
        message_id: i64,
        seller: &str,
        amount: i64,
        product_ref: Option<&str>,
    ) -> Result<(EscrowItem, bool), BotError> {
        let escrow_id = format!("esc:c{chat_id}m{message_id}");
        // replay-safe create — retry همان پیام همان امانت را برمی‌گرداند
        if let Some(existing) = self.store.get_escrow(&escrow_id).await.map_err(Self::map_db)? {
            return Ok((Self::item_of(&existing), true));
        }
        let buyer = user_account(user);
        let seller_account = resolve_seller(seller, self).await?;
        let amount_din =
            dropagentx_domain::Din::new(amount).map_err(|e| BotError::Domain(e.code))?;
        let at = now_iso();
        let apply_id = escrow_id.clone();
        let res = self
            .store
            .apply(
                ApplyOptions {
                    actor: buyer.clone(),
                    action: "escrow.create".into(),
                    target: Some(apply_id.clone()),
                    escrow_id: Some(apply_id.clone()),
                    idempotency_key: None,
                },
                |ledger, registry| {
                    create_escrow(
                        ledger,
                        registry,
                        &CreateEscrowCommand {
                            id: apply_id.clone(),
                            buyer_id: buyer.clone(),
                            seller_id: seller_account.clone(),
                            amount: amount_din,
                            product_ref: product_ref.map(str::to_owned),
                            expires_at: None,
                            at: Some(at.clone()),
                        },
                    )
                    .map(|o| CommandOutcome {
                        ledger: o.ledger,
                        tx: o.tx,
                        escrow: Some(o.escrow),
                        events: o
                            .events
                            .into_iter()
                            .map(dropagentx_domain::DomainEvent::Escrow)
                            .collect(),
                    })
                },
            )
            .await
            .map_err(Self::map_db)?;
        let item = res.escrow.as_ref().map(Self::item_of).ok_or(BotError::Backend)?;
        self.store
            .audit_ext(&buyer, "bot.escrow.create", Some(&item.id), None)
            .await
            .map_err(Self::map_db)?;
        Ok((item, res.replayed))
    }

    async fn transition(
        &mut self,
        user: i64,
        chat_id: i64,
        message_id: i64,
        escrow_id: &str,
        verb: Transition,
        reason: Option<&str>,
    ) -> Result<TransitionOutcome, BotError> {
        let account = user_account(user);
        let key = format!("tg:{chat_id}:{message_id}");

        // گارد نقش fail-closed — قبل از هر apply (apply گارد وضعیت را داخل tx
        // اتمیک اجرا می‌کند؛ اینجا فقط پیام دقیقِ نقش را می‌سازیم)
        let escrow = self.load_escrow(escrow_id).await?;
        let is_buyer = escrow.buyer_id == account;
        let is_seller = escrow.seller_id == account;
        if !is_buyer && !is_seller {
            return Err(BotError::NotParty);
        }
        if verb.buyer_only() && !is_buyer {
            return Err(BotError::BuyerOnly);
        }
        if !verb.buyer_only() && !is_seller {
            return Err(BotError::SellerOnly);
        }

        let at = now_iso();
        let apply_id = escrow_id.to_owned();
        let actor = account.clone();
        let res = match verb {
            Transition::Fund => {
                self.store
                    .apply(
                        opts(&actor, "escrow.fund", &apply_id, Some(key.clone())),
                        |ledger, registry| {
                            fund_escrow(
                                ledger,
                                registry,
                                &EscrowRefCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
            }
            Transition::Deliver => {
                self.store
                    .apply(
                        opts(&actor, "escrow.deliver", &apply_id, Some(key.clone())),
                        |ledger, registry| {
                            deliver_escrow(
                                ledger,
                                registry,
                                &DeliverEscrowCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                    delivery_ref: reason.map(str::to_owned),
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
            }
            Transition::Release => {
                self.store
                    .apply(
                        opts(&actor, "escrow.release", &apply_id, Some(key.clone())),
                        |ledger, registry| {
                            release_escrow(
                                ledger,
                                registry,
                                &ReleaseEscrowCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                    approved_by_admin: false,
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
            }
            Transition::Dispute => {
                self.store
                    .apply(
                        opts(&actor, "escrow.dispute", &apply_id, Some(key.clone())),
                        |ledger, registry| {
                            dispute_escrow(
                                ledger,
                                registry,
                                &DisputeEscrowCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                    reason: reason.unwrap_or("buyer dispute").to_owned(),
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
            }
            Transition::Cancel => {
                self.store
                    .apply(
                        opts(&actor, "escrow.cancel", &apply_id, Some(key.clone())),
                        |ledger, registry| {
                            cancel_escrow(
                                ledger,
                                registry,
                                &EscrowRefCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
            }
        };

        match res {
            Ok(r) => {
                let out = Self::settled_outcome(&r);
                self.store
                    .audit_ext(
                        &actor,
                        format!("bot.{}", verb.action()).as_str(),
                        Some(escrow_id),
                        None,
                    )
                    .await
                    .map_err(Self::map_db)?;
                Ok(out)
            }
            Err(dropagentx_db::DbError::Domain(err))
                if verb == Transition::Release
                    && err.code == DomainErrorCode::RequiresHumanApproval =>
            {
                // مادهٔ ۷ — صف HITL سرور (شناسهٔ دترمیینیستیک؛ replay-safe)
                let hitl = self.enqueue_hitl_row(escrow_id, user).await?;
                let escrow = self.load_escrow(escrow_id).await?;
                Ok(TransitionOutcome {
                    escrow: Self::item_of(&escrow),
                    replayed: false,
                    hitl_queued: Some(hitl),
                    tx_id: None,
                    seller_net: None,
                    tax: None,
                })
            }
            Err(e) => Err(Self::map_db(e)),
        }
    }

    async fn hitl_queue(&self) -> Result<Vec<HitlItem>, BotError> {
        Ok(self
            .store
            .list_hitl(Some("PENDING"))
            .await
            .map_err(Self::map_db)?
            .iter()
            .map(hitl_item_of)
            .collect())
    }

    async fn hitl_approve(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> Result<HitlOutcome, BotError> {
        let decision = self
            .store
            .approve_hitl_and_settle(hitl_id, &user_account(admin), note)
            .await
            .map_err(Self::map_db)?;
        let (net, tax) = match dropagentx_domain::Din::new(decision.request.amount)
            .ok()
            .and_then(|a| split_seller_tax(a).ok())
        {
            Some(s) => (Some(s.seller.as_i64()), Some(s.tax.as_i64())),
            None => (None, None),
        };
        self.store
            .audit_ext(&user_account(admin), "bot.hitl.approve", Some(hitl_id), None)
            .await
            .map_err(Self::map_db)?;
        Ok(HitlOutcome {
            id: decision.request.id,
            status: decision.request.status,
            seller_net: net,
            tax,
            replayed: decision.replayed,
        })
    }

    async fn hitl_reject(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> Result<HitlOutcome, BotError> {
        let row = self
            .store
            .reject_hitl(hitl_id, &user_account(admin), note)
            .await
            .map_err(Self::map_db)?;
        self.store
            .audit_ext(&user_account(admin), "bot.hitl.reject", Some(hitl_id), None)
            .await
            .map_err(Self::map_db)?;
        Ok(HitlOutcome {
            id: row.id,
            status: row.status,
            seller_net: None,
            tax: None,
            replayed: false,
        })
    }

    async fn economy(&self) -> Result<EconomyItem, BotError> {
        let e = self.store.economy_summary().await.map_err(Self::map_db)?;
        Ok(EconomyItem {
            circulating_supply: e.circulating_supply,
            treasury_balance: e.treasury_balance,
            escrow_locked: e.escrow_locked,
            users_sum: e.users_sum,
            open_escrows: e.open_escrows,
            balanced: e.balanced,
        })
    }

    async fn audit_tail(&self, limit: usize) -> Result<Vec<String>, BotError> {
        let rows = self
            .store
            .list_audit(i64::try_from(limit).unwrap_or(10))
            .await
            .map_err(Self::map_db)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                format!(
                    "{} | {} | {} | {}",
                    r.at,
                    r.actor,
                    r.action,
                    r.target.as_deref().unwrap_or("-")
                )
            })
            .collect())
    }
}

/// ApplyOptions استاندارد بات — actor = حساب شهروند (audit صادقانه).
fn opts(actor: &str, action: &str, escrow_id: &str, key: Option<String>) -> ApplyOptions {
    ApplyOptions {
        actor: actor.to_owned(),
        action: action.to_owned(),
        target: Some(escrow_id.to_owned()),
        escrow_id: Some(escrow_id.to_owned()),
        idempotency_key: key,
    }
}

/// خروجی دامنه → خروجی فرمان فروشگاه.
fn to_outcome(o: dropagentx_domain::EscrowOutcome) -> CommandOutcome {
    CommandOutcome {
        ledger: o.ledger,
        tx: o.tx,
        escrow: Some(o.escrow),
        events: o.events.into_iter().map(dropagentx_domain::DomainEvent::Escrow).collect(),
    }
}

/// ردیف HITL سرور → نمای بات.
fn hitl_item_of(h: &HitlRow) -> HitlItem {
    HitlItem {
        id: h.id.clone(),
        escrow_id: h.escrow_id.clone(),
        amount: h.amount,
        requester_id: h.requester_id.clone(),
        status: h.status.clone(),
        expires_at: h.expires_at.clone(),
    }
}

impl StoreEnv {
    /// صف‌کردن درخواست HITL — همان شکل هندلر فاز ۴ (`enqueue_hitl`).
    async fn enqueue_hitl_row(
        &self,
        escrow_id: &str,
        requester: i64,
    ) -> Result<HitlItem, BotError> {
        let escrow = self.load_escrow(escrow_id).await?;
        let row = HitlRow {
            id: format!("hitl:{escrow_id}"),
            kind: "ESCROW_RELEASE".into(),
            escrow_id: escrow_id.to_owned(),
            amount: escrow.amount.as_i64(),
            requester_id: user_account(requester),
            note: None,
            status: "PENDING".into(),
            created_at: now_iso(),
            expires_at: (chrono::Utc::now() + chrono::Duration::hours(24))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            decided_by: None,
            decided_at: None,
            decision_note: None,
        };
        let queued = self.store.enqueue_hitl(row, escrow_id).await.map_err(Self::map_db)?;
        Ok(hitl_item_of(&queued))
    }
}

/// resolve فروشنده — مثل [`dropagentx_bot::memory`]: عدد تلگرام یا حساب یا محصول.
async fn resolve_seller(seller: &str, env: &StoreEnv) -> Result<String, BotError> {
    let raw = seller.trim();
    if raw.is_empty() {
        return Err(BotError::Domain(DomainErrorCode::UnknownAccount));
    }
    if let Ok(tg) = raw.parse::<i64>() {
        return Ok(user_account(tg));
    }
    if raw.starts_with("user:tg:") {
        return Ok(raw.to_owned());
    }
    // شناسهٔ محصول → فروشندهٔ آن
    let products = env.products(100).await?;
    products
        .iter()
        .find(|p| p.id == raw)
        .map(|p| p.seller_id.clone())
        .ok_or(BotError::Domain(DomainErrorCode::UnknownAccount))
}
