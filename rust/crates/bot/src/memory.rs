//! Backend درون‌حافظه‌ای — معادل آفلاین `V3Store` برای تست‌های orchestrator.
//!
//! معناشناسی عیناً همان الگوی `V3Store::apply` است (فاز ۲/۴):
//! ۱) پیش‌چک replay با کلید یکسان‌سازی — retry هرگز دوبار پول جابه‌جا نمی‌کند؛
//! ۲) گارد نقش fail-closed (خریدار fund/release/dispute/cancel، فروشنده deliver)؛
//! ۳) اجرای دامنهٔ خالص فاز ۱ (بدون IO)؛ ۴) به‌روزرسانی وضعیت + audit.
//!
//! مادهٔ ۷ هم اینجا پیاده شده: آزادسازی ≥ ۵۰٬۰۰۰ Đ خطای
//! `REQUIRES_HUMAN_APPROVAL` دامنه را می‌گیرد و به‌جای آن درخواست HITL با
//! شناسهٔ دترمیینیستیک `hitl:<escrow_id>` صف می‌کند (TTL ۲۴ ساعت)؛ تأیید
//! تسویهٔ اتمیک را با کلید قطعی `hitl:<id>:approve` اجرا می‌کند و تصمیم دوم
//! `HITL_ALREADY_DECIDED` تایپ‌شده می‌دهد با بازپرداخت صفر.
//!
//! صرفاً برای تست/توسعه — در تولید `StoreEnv` سرور روی SQLite واقعی می‌نشیند.
//! `std::sync::Mutex` (نه RefCell) تا futureهای متدها `Send` بمانند و در
//! هندلرهای async هم قابل استفاده باشد؛ هیچ قفلی روی `await` نگه داشته نمی‌شود
//! (بدنهٔ متدها کامل sync است).

use std::collections::BTreeMap;
use std::sync::Mutex;

use dropagentx_domain::{
    add_account, cancel_escrow, create_escrow, deliver_escrow, dispute_escrow, fund_escrow,
    open_ledger, open_registry, post, release_escrow, split_seller_tax, Account, AccountKind,
    Charter, CreateEscrowCommand, DeliverEscrowCommand, DisputeEscrowCommand, DomainError,
    DomainErrorCode, Escrow, EscrowRefCommand, EscrowRegistry, EscrowStatus, LedgerState,
    PostCommand, Posting, ReleaseEscrowCommand, BURN_ACCOUNT_ID, MINT_ACCOUNT_ID,
    TREASURY_ACCOUNT_ID,
};

use crate::env::{
    user_account, BotError, EconomyItem, Env, EscrowItem, HistoryItem, HitlItem, HitlOutcome,
    ProductItem, Transition, TransitionOutcome,
};
use crate::i18n::Locale;

/// ردیف HITL درون‌حافظه‌ای.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryHitl {
    id: String,
    escrow_id: String,
    amount: i64,
    requester_id: String,
    status: String,
    expires_at: String,
    decided_by: Option<String>,
}

/// وضعیت کشور درون‌حافظه‌ای — همان چیزی که `V3Store` روی SQLite نگه می‌دارد.
#[derive(Debug, Default)]
pub struct MemoryDomain {
    ledger: LedgerState,
    registry: EscrowRegistry,
    /// کلید یکسان‌سازی → شناسهٔ تراکنش — آینهٔ `LedgerState.idempotency`؛
    /// جدا نگه داشته می‌شود برای پیش‌چک replay قبل از گارد وضعیت (مثل apply).
    idem: BTreeMap<String, String>,
    hitl: Vec<MemoryHitl>,
    /// انتخاب صریح زبان — Mutex برای Sync بودن `&self`
    locales: Mutex<BTreeMap<i64, Locale>>,
    admins: Vec<i64>,
    products: Vec<ProductItem>,
    audit: Vec<String>,
    /// ساعت قطعی برای تست — ISO RFC3339.
    now: String,
}

impl MemoryDomain {
    /// کشور خالی با حساب‌های سیستمی (mint/treasury/burn).
    #[must_use]
    pub fn new(now_iso: &str) -> Self {
        let mut ledger = open_ledger();
        for (id, kind, label) in [
            (MINT_ACCOUNT_ID, AccountKind::Mint, "mint"),
            (TREASURY_ACCOUNT_ID, AccountKind::Treasury, "treasury"),
            (BURN_ACCOUNT_ID, AccountKind::Burn, "burn"),
        ] {
            ledger = add_account(
                &ledger,
                Account { id: id.to_owned(), kind, label: Some(label.to_owned()) },
            )
            .unwrap_or_else(|e| panic!("memory seed: system account must be unique: {e}"));
        }
        Self {
            ledger,
            registry: open_registry(),
            idem: BTreeMap::new(),
            hitl: Vec::new(),
            locales: Mutex::new(BTreeMap::new()),
            admins: Vec::new(),
            products: Vec::new(),
            audit: Vec::new(),
            now: now_iso.to_owned(),
        }
    }

    /// ادمین‌ها — fail-closed: فهرست خالی یعنی هیچ فرمان ادمینی اجرا نمی‌شود.
    pub fn with_admins(&mut self, admins: &[i64]) -> &mut Self {
        self.admins = admins.to_vec();
        self
    }

    /// بذر شهروند با موجودی اولیه (مسیر ضرب: mint → user؛ منبع فاز ۲).
    pub fn seed_user(&mut self, tg_id: i64, balance: i64) -> &mut Self {
        let account = user_account(tg_id);
        if !self.ledger.accounts.contains_key(&account) {
            let ledger = add_account(
                &self.ledger,
                Account {
                    id: account.clone(),
                    kind: AccountKind::User,
                    label: Some(format!("tg {tg_id}")),
                },
            )
            .unwrap_or_else(|e| panic!("memory seed: user account must be unique: {e}"));
            self.ledger = ledger;
        }
        if balance > 0 {
            let key = format!("seed:tg:{tg_id}");
            let posted = post(
                &self.ledger,
                &PostCommand {
                    id: None,
                    idempotency_key: Some(key.clone()),
                    postings: vec![
                        Posting::new(MINT_ACCOUNT_ID, -balance),
                        Posting::new(account, balance),
                    ],
                    memo: Some("seed".into()),
                    at: Some(self.now.clone()),
                },
            )
            .unwrap_or_else(|e| panic!("memory seed: mint posting must be valid: {e}"));
            self.ledger = posted.state;
            self.idem.insert(key, posted.tx.id);
        }
        self
    }

    /// بذر محصول (فروشنده به‌شکل شناسهٔ حساب).
    pub fn seed_product(&mut self, id: &str, seller_tg: i64, title: &str, price: i64) -> &mut Self {
        self.products.push(ProductItem {
            id: id.to_owned(),
            seller_id: user_account(seller_tg),
            title: title.to_owned(),
            price_din: price,
        });
        self
    }

    /// جلو بردن ساعت قطعی (تست TTL — منقضی‌شدن درخواست HITL).
    pub fn set_now(&mut self, now_iso: &str) {
        self.now = now_iso.to_owned();
    }

    fn item_of(&self, e: &Escrow) -> EscrowItem {
        EscrowItem {
            id: e.id.clone(),
            buyer_id: e.buyer_id.clone(),
            seller_id: e.seller_id.clone(),
            amount: e.amount.as_i64(),
            status: e.status.as_str().to_owned(),
            product_ref: e.product_ref.clone(),
        }
    }

    fn escrow_of(&self, escrow_id: &str) -> Result<Escrow, BotError> {
        self.registry.escrows.get(escrow_id).cloned().ok_or(BotError::EscrowNotFound)
    }

    fn push_audit(&mut self, line: String) {
        self.audit.push(format!("{} | {line}", self.now));
    }

    /// انقضای TTL ۲۴ ساعت — از ساعت قطعی، مثل هندلر فاز ۴.
    fn ttl_from_now(&self) -> String {
        chrono::DateTime::parse_from_rfc3339(&self.now)
            .map(|t| {
                (t.with_timezone(&chrono::Utc) + chrono::Duration::hours(24))
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_else(|_| self.now.clone())
    }

    /// صف‌کردن درخواست HITL — شناسهٔ دترمیینیستیک؛ تکرار همان درخواست موجود
    /// را برمی‌گرداند (replay-safe مثل `enqueue_hitl` سرور).
    fn enqueue_hitl(&mut self, escrow_id: &str, requester: i64) -> Result<HitlItem, BotError> {
        let id = format!("hitl:{escrow_id}");
        if let Some(existing) = self.hitl.iter().find(|h| h.id == id) {
            return Ok(HitlItem {
                id: existing.id.clone(),
                escrow_id: existing.escrow_id.clone(),
                amount: existing.amount,
                requester_id: existing.requester_id.clone(),
                status: existing.status.clone(),
                expires_at: existing.expires_at.clone(),
            });
        }
        let escrow = self.escrow_of(escrow_id)?;
        let expires_at = self.ttl_from_now();
        let item = HitlItem {
            id,
            escrow_id: escrow_id.to_owned(),
            amount: escrow.amount.as_i64(),
            requester_id: user_account(requester),
            status: "PENDING".into(),
            expires_at: expires_at.clone(),
        };
        self.hitl.push(MemoryHitl {
            id: item.id.clone(),
            escrow_id: item.escrow_id.clone(),
            amount: item.amount,
            requester_id: item.requester_id.clone(),
            status: item.status.clone(),
            expires_at,
            decided_by: None,
        });
        self.push_audit(format!(
            "user:tg:{requester} | hitl.enqueue | {} | {}",
            item.id, item.amount
        ));
        Ok(item)
    }

    /// آستانهٔ HITL منشور — برای تست مرز (مادهٔ ۷).
    #[must_use]
    pub const fn hitl_threshold() -> i64 {
        Charter::HITL_THRESHOLD_DIN.as_i64()
    }

    /// تعداد کل درخواست‌های HITL — برای تست‌های replay.
    #[must_use]
    pub fn hitl_len(&self) -> usize {
        self.hitl.len()
    }
}

impl Env for MemoryDomain {
    fn locale(&self, user: i64, hint: Option<&str>) -> Locale {
        if let Ok(map) = self.locales.lock() {
            if let Some(l) = map.get(&user) {
                return *l;
            }
        }
        Locale::from_language_code(hint)
    }

    fn set_locale(&self, user: i64, locale: Locale) {
        if let Ok(mut map) = self.locales.lock() {
            map.insert(user, locale);
        }
    }

    fn is_admin(&self, user: i64) -> bool {
        self.admins.contains(&user)
    }

    async fn balance(&self, user: i64) -> Result<i64, BotError> {
        let account = user_account(user);
        self.ledger
            .balances
            .get(&account)
            .copied()
            .ok_or(BotError::Domain(DomainErrorCode::UnknownAccount))
    }

    async fn history(&self, user: i64, limit: usize) -> Result<Vec<HistoryItem>, BotError> {
        let account = user_account(user);
        if !self.ledger.accounts.contains_key(&account) {
            return Err(BotError::Domain(DomainErrorCode::UnknownAccount));
        }
        let mut items = Vec::new();
        for tx in self.ledger.txs.values().rev() {
            for p in &tx.postings {
                if p.account_id == account {
                    items.push(HistoryItem {
                        tx_id: tx.id.clone(),
                        amount: p.amount,
                        memo: tx.memo.clone(),
                        at: tx.at.clone(),
                    });
                    break;
                }
            }
            if items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }

    async fn products(&self, limit: usize) -> Result<Vec<ProductItem>, BotError> {
        Ok(self.products.iter().take(limit).cloned().collect())
    }

    async fn escrow(&self, escrow_id: &str) -> Result<Option<EscrowItem>, BotError> {
        Ok(self.registry.escrows.get(escrow_id).map(|e| self.item_of(e)))
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
            .registry
            .escrows
            .values()
            .filter(|e| {
                (e.buyer_id == account || e.seller_id == account) && open.contains(&e.status)
            })
            .map(|e| self.item_of(e))
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
        let buyer = user_account(user);
        let seller_account = seller_account_of(seller, &self.products)
            .ok_or(BotError::Domain(DomainErrorCode::UnknownAccount))?;
        let escrow_id = format!("esc:c{chat_id}m{message_id}");
        // replay-safe create — retry همان پیام همان امانت را برمی‌گرداند
        if let Some(existing) = self.registry.escrows.get(&escrow_id) {
            return Ok((self.item_of(existing), true));
        }
        let amount_din = dropagentx_domain::Din::new(amount)
            .map_err(|e: DomainError| BotError::Domain(e.code))?;
        let outcome = create_escrow(
            &self.ledger,
            &self.registry,
            &CreateEscrowCommand {
                id: escrow_id,
                buyer_id: buyer,
                seller_id: seller_account,
                amount: amount_din,
                product_ref: product_ref.map(str::to_owned),
                expires_at: None,
                at: Some(self.now.clone()),
            },
        )
        .map_err(|e| BotError::Domain(e.code))?;
        self.ledger = outcome.ledger;
        self.registry = outcome.registry;
        let item = self.item_of(&outcome.escrow);
        self.push_audit(format!("user:tg:{user} | escrow.create | {}", item.id));
        Ok((item, false))
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

        // ۱) پیش‌چک replay — عین `V3Store::apply` گام ۱ (فقط گذارهای پولی کلید می‌گیرند)
        if verb.moves_money() && self.idem.contains_key(&key) {
            let escrow = self.escrow_of(escrow_id)?;
            return Ok(TransitionOutcome {
                escrow: self.item_of(&escrow),
                replayed: true,
                hitl_queued: None,
                tx_id: None,
                seller_net: None,
                tax: None,
            });
        }

        // ۲) گارد نقش fail-closed — ADR-009: «خریدار fund/release/dispute/cancel،
        //    فروشنده deliver» — قبل از هر تغییر وضعیت
        let escrow = self.escrow_of(escrow_id)?;
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

        // ۳) دامنهٔ خالص — همان توابع فاز ۱ که هندلرهای فاز ۴ صدا می‌زنند
        let domain_run = match verb {
            Transition::Fund => fund_escrow(
                &self.ledger,
                &self.registry,
                &EscrowRefCommand {
                    escrow_id: escrow.id.clone(),
                    idempotency_key: Some(key.clone()),
                    at: Some(self.now.clone()),
                },
            ),
            Transition::Deliver => deliver_escrow(
                &self.ledger,
                &self.registry,
                &DeliverEscrowCommand {
                    escrow_id: escrow.id.clone(),
                    idempotency_key: Some(key.clone()),
                    at: Some(self.now.clone()),
                    delivery_ref: reason.map(str::to_owned),
                },
            ),
            Transition::Release => release_escrow(
                &self.ledger,
                &self.registry,
                &ReleaseEscrowCommand {
                    escrow_id: escrow.id.clone(),
                    idempotency_key: Some(key.clone()),
                    at: Some(self.now.clone()),
                    approved_by_admin: false,
                },
            ),
            Transition::Dispute => dispute_escrow(
                &self.ledger,
                &self.registry,
                &DisputeEscrowCommand {
                    escrow_id: escrow.id.clone(),
                    idempotency_key: Some(key.clone()),
                    at: Some(self.now.clone()),
                    reason: reason.unwrap_or("no reason given").to_owned(),
                },
            ),
            Transition::Cancel => cancel_escrow(
                &self.ledger,
                &self.registry,
                &EscrowRefCommand {
                    escrow_id: escrow.id.clone(),
                    idempotency_key: Some(key.clone()),
                    at: Some(self.now.clone()),
                },
            ),
        };

        match domain_run {
            Ok(outcome) => {
                self.ledger = outcome.ledger;
                self.registry = outcome.registry;
                if let Some(tx) = &outcome.tx {
                    if let Some(k) = &tx.idempotency_key {
                        self.idem.insert(k.clone(), tx.id.clone());
                    }
                }
                self.push_audit(format!(
                    "user:tg:{user} | {} | {}",
                    verb.action(),
                    outcome.escrow.id
                ));

                if verb == Transition::Release {
                    // زیر آستانه: تسویهٔ مستقیم ۹۰/۱۰ — مادهٔ ۲
                    let split = split_seller_tax(outcome.escrow.amount)
                        .map_err(|e| BotError::Domain(e.code))?;
                    Ok(TransitionOutcome {
                        escrow: self.item_of(&outcome.escrow),
                        replayed: false,
                        hitl_queued: None,
                        tx_id: outcome.tx.as_ref().map(|t| t.id.clone()),
                        seller_net: Some(split.seller.as_i64()),
                        tax: Some(split.tax.as_i64()),
                    })
                } else {
                    Ok(TransitionOutcome {
                        escrow: self.item_of(&outcome.escrow),
                        replayed: false,
                        hitl_queued: None,
                        tx_id: outcome.tx.as_ref().map(|t| t.id.clone()),
                        seller_net: None,
                        tax: None,
                    })
                }
            }
            Err(e)
                if verb == Transition::Release
                    && e.code == DomainErrorCode::RequiresHumanApproval =>
            {
                // ۴) مادهٔ ۷ — ≥ ۵۰٬۰۰۰ Đ بدون تأیید ادمین: امانت همان‌جا می‌ماند
                //    و درخواست HITL صف می‌شود (مثل مسیر سرور)
                let escrow = self.escrow_of(escrow_id)?;
                let hitl = self.enqueue_hitl(&escrow.id, user)?;
                Ok(TransitionOutcome {
                    escrow: self.item_of(&escrow),
                    replayed: false,
                    hitl_queued: Some(hitl),
                    tx_id: None,
                    seller_net: None,
                    tax: None,
                })
            }
            Err(e) => Err(BotError::Domain(e.code)),
        }
    }

    async fn hitl_queue(&self) -> Result<Vec<HitlItem>, BotError> {
        Ok(self
            .hitl
            .iter()
            .filter(|h| h.status == "PENDING")
            .map(|h| HitlItem {
                id: h.id.clone(),
                escrow_id: h.escrow_id.clone(),
                amount: h.amount,
                requester_id: h.requester_id.clone(),
                status: h.status.clone(),
                expires_at: h.expires_at.clone(),
            })
            .collect())
    }

    async fn hitl_approve(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> Result<HitlOutcome, BotError> {
        let admin_account = user_account(admin);
        let idx = self
            .hitl
            .iter()
            .position(|h| h.id == hitl_id)
            .ok_or(BotError::Domain(DomainErrorCode::HitlNotFound))?;
        let row = self.hitl[idx].clone();
        if row.status != "PENDING" {
            // تصمیم دوم — تایپ‌شده؛ هیچ پولی جابه‌جا نمی‌شود
            return Err(BotError::Domain(DomainErrorCode::HitlAlreadyDecided));
        }
        if row.expires_at.as_str() < self.now.as_str() {
            return Err(BotError::Domain(DomainErrorCode::HitlExpired));
        }

        // تسویهٔ اتمیک — کلید قطعی `hitl:<id>:approve`، عین `approve_hitl_and_settle`
        let settle_key = format!("hitl:{hitl_id}:approve");
        let outcome = release_escrow(
            &self.ledger,
            &self.registry,
            &ReleaseEscrowCommand {
                escrow_id: row.escrow_id.clone(),
                idempotency_key: Some(settle_key.clone()),
                at: Some(self.now.clone()),
                approved_by_admin: true,
            },
        )
        .map_err(|e| BotError::Domain(e.code))?;
        self.ledger = outcome.ledger;
        self.registry = outcome.registry;
        self.idem.insert(settle_key, outcome.tx.as_ref().map(|t| t.id.clone()).unwrap_or_default());

        let split =
            split_seller_tax(outcome.escrow.amount).map_err(|e| BotError::Domain(e.code))?;
        self.hitl[idx].status = "APPROVED".into();
        self.hitl[idx].decided_by = Some(admin_account.clone());
        self.push_audit(format!(
            "{admin_account} | hitl.approve | {hitl_id} | {}",
            note.unwrap_or("")
        ));
        Ok(HitlOutcome {
            id: hitl_id.to_owned(),
            status: "APPROVED".into(),
            seller_net: Some(split.seller.as_i64()),
            tax: Some(split.tax.as_i64()),
            replayed: false,
        })
    }

    async fn hitl_reject(
        &mut self,
        admin: i64,
        hitl_id: &str,
        note: Option<&str>,
    ) -> Result<HitlOutcome, BotError> {
        let admin_account = user_account(admin);
        let idx = self
            .hitl
            .iter()
            .position(|h| h.id == hitl_id)
            .ok_or(BotError::Domain(DomainErrorCode::HitlNotFound))?;
        let row = self.hitl[idx].clone();
        if row.status != "PENDING" {
            return Err(BotError::Domain(DomainErrorCode::HitlAlreadyDecided));
        }
        self.hitl[idx].status = "REJECTED".into();
        self.hitl[idx].decided_by = Some(admin_account.clone());
        self.push_audit(format!(
            "{admin_account} | hitl.reject | {hitl_id} | {}",
            note.unwrap_or("")
        ));
        Ok(HitlOutcome {
            id: hitl_id.to_owned(),
            status: "REJECTED".into(),
            seller_net: None,
            tax: None,
            replayed: false,
        })
    }

    async fn economy(&self) -> Result<EconomyItem, BotError> {
        let users_sum: i64 = self
            .ledger
            .balances
            .iter()
            .filter(|(id, _)| {
                self.ledger.accounts.get(*id).map(|a| a.kind) == Some(AccountKind::User)
            })
            .map(|(_, b)| *b)
            .sum();
        let escrow_locked: i64 = self
            .ledger
            .balances
            .iter()
            .filter(|(id, _)| {
                self.ledger.accounts.get(*id).map(|a| a.kind) == Some(AccountKind::Escrow)
            })
            .map(|(_, b)| *b)
            .sum();
        let treasury_balance = self.ledger.balances.get(TREASURY_ACCOUNT_ID).copied().unwrap_or(0);
        let trial_sum: i64 = self.ledger.balances.values().sum();
        let open = [
            EscrowStatus::Created,
            EscrowStatus::Funded,
            EscrowStatus::Delivered,
            EscrowStatus::Disputed,
        ];
        Ok(EconomyItem {
            circulating_supply: users_sum + escrow_locked + treasury_balance,
            treasury_balance,
            escrow_locked,
            users_sum,
            open_escrows: i64::try_from(
                self.registry.escrows.values().filter(|e| open.contains(&e.status)).count(),
            )
            .unwrap_or(i64::MAX),
            balanced: trial_sum == 0,
        })
    }

    async fn audit_tail(&self, limit: usize) -> Result<Vec<String>, BotError> {
        let start = self.audit.len().saturating_sub(limit);
        Ok(self.audit[start..].to_vec())
    }
}

/// شناسهٔ فروشندهٔ هدف — `user:tg:123`، `123` یا شناسهٔ محصول (فروشندهٔ آن).
fn seller_account_of(seller: &str, products: &[ProductItem]) -> Option<String> {
    let raw = seller.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(tg) = raw.parse::<i64>() {
        return Some(user_account(tg));
    }
    if raw.starts_with("user:tg:") {
        return Some(raw.to_owned());
    }
    products.iter().find(|p| p.id == raw).map(|p| p.seller_id.clone())
}
