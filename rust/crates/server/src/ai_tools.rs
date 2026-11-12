//! `AiTools` — backend واقعی مهارت‌های AI روی `V3Store` (فاز ۶).
//!
//! گیت دوم authz داخل هر مهارت اعمال می‌شود (fail-closed):
//! * کاربر عادی فقط روی دادهٔ خودش (`ctx.actor`) — هیچ کیف/امانت دیگری
//! * staff (کلید ادمین + `role: staff`) می‌تواند روی دادهٔ دیگران بخواند و
//!   مهارت‌های staff_only را اجرا کند
//! * هر گذار پولی از مسیر دامنهٔ خالص فاز ۱ داخل `apply` می‌گذرد (audit/outbox
//!   همان تراکنش) — عین هندلرهای فاز ۴
//!
//! مادهٔ ۷: `escrow_refund` هرگز مستقیم بازپرداخت نمی‌کند — درخواست HITL با
//! شناسهٔ دترمیینیستیک صف می‌شود و اسکرو همان‌جا می‌ماند (معیار پذیرش فاز ۶).

use dropagentx_ai::{AiUsageRecord, ToolBackend, ToolCtx, ToolOutcome};
use dropagentx_db::{now_iso, ApplyOptions, CommandOutcome, HitlRow, ProductRow, V3Store};
use dropagentx_domain::{
    create_escrow, deliver_escrow, dispute_escrow, release_escrow, CreateEscrowCommand,
    DeliverEscrowCommand, DisputeEscrowCommand, DomainError, ReleaseEscrowCommand,
    TREASURY_ACCOUNT_ID,
};

/// backend مهارت‌ها روی فروشگاه واقعی — ارزان برای ساخت به ازای هر درخواست.
#[derive(Debug, Clone)]
pub struct AiTools {
    /// فروشگاه v3
    store: V3Store,
}

impl AiTools {
    /// ساخت از فروشگاه.
    #[must_use]
    pub fn new(store: V3Store) -> Self {
        Self { store }
    }

    fn failed(code: &str, message: impl Into<String>) -> ToolOutcome {
        ToolOutcome::Failed { code: code.to_owned(), message: message.into() }
    }

    async fn escrow_of(&self, id: &str) -> Result<dropagentx_domain::Escrow, ToolOutcome> {
        self.store
            .get_escrow(id)
            .await
            .map_err(|e| Self::failed("INTERNAL", e.to_string()))?
            .ok_or_else(|| Self::failed("ESCROW_NOT_FOUND", format!("Escrow {id} does not exist")))
    }

    /// صف‌کردن درخواست HITL — شناسهٔ دترمیینیستیک؛ تکرار همان ردیف را می‌دهد.
    async fn enqueue_hitl(
        &self,
        id: &str,
        kind: &str,
        escrow_id: &str,
        amount: i64,
        requester: &str,
        note: Option<String>,
    ) -> Result<(), ToolOutcome> {
        let row = HitlRow {
            id: id.to_owned(),
            kind: kind.to_owned(),
            escrow_id: escrow_id.to_owned(),
            amount,
            requester_id: requester.to_owned(),
            note,
            status: "PENDING".into(),
            created_at: now_iso(),
            expires_at: (chrono::Utc::now() + chrono::Duration::hours(24))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            decided_by: None,
            decided_at: None,
            decision_note: None,
        };
        self.store
            .enqueue_hitl(row, escrow_id)
            .await
            .map_err(|e| Self::failed("INTERNAL", e.to_string()))?;
        Ok(())
    }

    /// اجرای گذار امانت از مسیر دامنهٔ خالص داخل `apply`.
    async fn apply_escrow<F>(
        &self,
        actor: &str,
        action: &str,
        escrow_id: &str,
        idempotency_key: Option<String>,
        run: F,
    ) -> Result<dropagentx_db::ApplyResult, ToolOutcome>
    where
        F: FnOnce(
            &dropagentx_domain::LedgerState,
            &dropagentx_domain::EscrowRegistry,
        ) -> Result<CommandOutcome, DomainError>,
    {
        self.store
            .apply(
                ApplyOptions {
                    actor: actor.to_owned(),
                    action: action.to_owned(),
                    target: Some(escrow_id.to_owned()),
                    escrow_id: Some(escrow_id.to_owned()),
                    idempotency_key,
                },
                run,
            )
            .await
            .map_err(|e| match e {
                dropagentx_db::DbError::Domain(err) => {
                    Self::failed(err.code.as_str(), err.message.clone())
                }
                other => Self::failed("INTERNAL", other.to_string()),
            })
    }
}

fn to_outcome(o: dropagentx_domain::EscrowOutcome) -> CommandOutcome {
    CommandOutcome {
        ledger: o.ledger,
        tx: o.tx,
        escrow: Some(o.escrow),
        events: o.events.into_iter().map(dropagentx_domain::DomainEvent::Escrow).collect(),
    }
}

impl ToolBackend for AiTools {
    async fn execute(&self, ctx: &ToolCtx, skill: &str, args: &serde_json::Value) -> ToolOutcome {
        let arg_str = |key: &str| args.get(key).and_then(|v| v.as_str()).map(str::to_owned);
        match skill {
            // ── خواندنی‌ها ──────────────────────────────────────────────────
            "wallet.balance" => {
                // کاربر فقط کیف خودش؛ staff با accountId صریح می‌تواند بخواند
                let account = if ctx.staff {
                    arg_str("accountId").unwrap_or_else(|| ctx.actor.clone())
                } else {
                    ctx.actor.clone()
                };
                match self.store.balance_of(&account).await {
                    Ok(Some(b)) => {
                        let din = dropagentx_domain::Din::new(b)
                            .map(dropagentx_domain::format_din)
                            .unwrap_or_else(|_| format!("{b} Đ"));
                        ToolOutcome::Data {
                            value: serde_json::json!({ "account": account, "balance": b }),
                            summary: format!("Balance of {account} is {din}"),
                        }
                    }
                    Ok(None) => {
                        Self::failed("UNKNOWN_ACCOUNT", format!("Account {account} does not exist"))
                    }
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }
            "wallet.history" => {
                let account = if ctx.staff {
                    arg_str("accountId").unwrap_or_else(|| ctx.actor.clone())
                } else {
                    ctx.actor.clone()
                };
                if self.store.balance_of(&account).await.ok().flatten().is_none() {
                    return Self::failed(
                        "UNKNOWN_ACCOUNT",
                        format!("Account {account} does not exist"),
                    );
                }
                match self.store.recent_postings_for(&account, 10).await {
                    Ok(rows) => {
                        let items: Vec<serde_json::Value> = rows
                            .iter()
                            .map(|p| serde_json::json!({"txId": p.tx_id, "amount": p.amount, "memo": p.memo, "at": p.at}))
                            .collect();
                        ToolOutcome::Data {
                            value: serde_json::json!({ "account": account, "items": items }),
                            summary: format!("{account} has {} recent postings.", items.len()),
                        }
                    }
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }
            "escrow.status" => {
                let Some(id) = arg_str("escrowId") else {
                    return Self::failed("INVALID_REQUEST", "args.escrowId is required");
                };
                // شفافیت عمومی مادهٔ ۶ — هر امانتی قابل خواندن است
                match self.escrow_of(&id).await {
                    Ok(e) => ToolOutcome::Data {
                        value: serde_json::json!({
                            "id": e.id, "status": e.status.as_str(), "amount": e.amount.as_i64(),
                            "buyerId": e.buyer_id, "sellerId": e.seller_id
                        }),
                        summary: format!(
                            "Escrow {} is {} ({} Đ)",
                            e.id,
                            e.status.as_str(),
                            e.amount.as_i64()
                        ),
                    },
                    Err(outcome) => outcome,
                }
            }
            "product.search" => {
                let q = arg_str("q");
                let filter =
                    dropagentx_db::ProductFilter { q, limit: 10, offset: 0, active_only: true };
                match self.store.list_products(&filter).await {
                    Ok(rows) => {
                        let items: Vec<serde_json::Value> = rows
                            .iter()
                            .map(|p| serde_json::json!({"id": p.id, "title": p.title, "priceDin": p.price_din}))
                            .collect();
                        ToolOutcome::Data {
                            value: serde_json::json!({ "items": items }),
                            summary: format!("Found {} active products.", items.len()),
                        }
                    }
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }
            "kb.search" => {
                let q = arg_str("q").unwrap_or_default();
                match self.store.search_kb(&q, 5).await {
                    Ok(rows) => {
                        let items: Vec<serde_json::Value> = rows
                            .iter()
                            .map(|(id, title, body)| serde_json::json!({"id": id, "title": title, "excerpt": body.chars().take(140).collect::<String>()}))
                            .collect();
                        ToolOutcome::Data {
                            value: serde_json::json!({ "items": items }),
                            summary: format!("Found {} knowledge entries for {q:?}.", items.len()),
                        }
                    }
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }
            "economy.summary" => match self.store.economy_summary().await {
                Ok(e) => ToolOutcome::Data {
                    value: serde_json::json!({
                        "circulatingSupply": e.circulating_supply,
                        "treasuryBalance": e.treasury_balance,
                        "escrowLocked": e.escrow_locked,
                        "usersSum": e.users_sum,
                        "openEscrows": e.open_escrows,
                        "balanced": e.balanced
                    }),
                    summary: format!(
                        "Circulating {} Đ, treasury {} Đ, escrow locked {} Đ, balanced: {}.",
                        e.circulating_supply, e.treasury_balance, e.escrow_locked, e.balanced
                    ),
                },
                Err(e) => Self::failed("INTERNAL", e.to_string()),
            },

            // ── نوشتنی‌ها ──────────────────────────────────────────────────
            "escrow.create" => {
                let Some(seller) = arg_str("sellerId") else {
                    return Self::failed("INVALID_REQUEST", "args.sellerId is required");
                };
                let buyer = arg_str("buyerId").unwrap_or_else(|| ctx.actor.clone());
                if buyer != ctx.actor && !ctx.staff {
                    return Self::failed(
                        "UNAUTHORIZED",
                        "Only staff may create escrows for others.",
                    );
                }
                let Some(amount) = args.get("amount").and_then(|v| v.as_i64()) else {
                    return Self::failed("INVALID_REQUEST", "args.amount must be an integer");
                };
                let Ok(amount_din) = dropagentx_domain::Din::new(amount) else {
                    return Self::failed(
                        "AMOUNT_OUT_OF_RANGE",
                        "amount must be a positive safe integer",
                    );
                };
                let product_ref = arg_str("productRef");
                let escrow_id = format!("esc:ai:{}", uuid::Uuid::now_v7());
                let at = now_iso();
                let apply_id = escrow_id.clone();
                let buyer_id = buyer.clone();
                let seller_id = seller.clone();
                match self
                    .apply_escrow(
                        &ctx.actor,
                        "escrow.create",
                        &escrow_id,
                        None,
                        |ledger, registry| {
                            create_escrow(
                                ledger,
                                registry,
                                &CreateEscrowCommand {
                                    id: apply_id.clone(),
                                    buyer_id: buyer_id.clone(),
                                    seller_id: seller_id.clone(),
                                    amount: amount_din,
                                    product_ref: product_ref.clone(),
                                    expires_at: None,
                                    at: Some(at.clone()),
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
                {
                    Ok(res) => match res.escrow {
                        Some(e) => ToolOutcome::Data {
                            value: serde_json::json!({"id": e.id, "status": e.status.as_str(), "amount": e.amount.as_i64()}),
                            summary: format!(
                                "Escrow {} created ({} Đ). Next: fund it.",
                                e.id,
                                e.amount.as_i64()
                            ),
                        },
                        None => Self::failed("INTERNAL", "escrow missing after create"),
                    },
                    Err(outcome) => outcome,
                }
            }
            "escrow.deliver" => {
                let Some(id) = arg_str("escrowId") else {
                    return Self::failed("INVALID_REQUEST", "args.escrowId is required");
                };
                let escrow = match self.escrow_of(&id).await {
                    Ok(e) => e,
                    Err(outcome) => return outcome,
                };
                if escrow.seller_id != ctx.actor && !ctx.staff {
                    return Self::failed("UNAUTHORIZED", "Only the seller (or staff) may deliver.");
                }
                let delivery_ref = arg_str("deliveryRef");
                let at = now_iso();
                let apply_id = id.clone();
                match self
                    .apply_escrow(&ctx.actor, "escrow.deliver", &id, None, |ledger, registry| {
                        deliver_escrow(
                            ledger,
                            registry,
                            &DeliverEscrowCommand {
                                escrow_id: apply_id.clone(),
                                idempotency_key: None,
                                at: Some(at.clone()),
                                delivery_ref: delivery_ref.clone(),
                            },
                        )
                        .map(to_outcome)
                    })
                    .await
                {
                    Ok(res) => match res.escrow {
                        Some(e) => ToolOutcome::Data {
                            value: serde_json::json!({"id": e.id, "status": e.status.as_str()}),
                            summary: format!("Escrow {} delivered — money stays locked.", e.id),
                        },
                        None => Self::failed("INTERNAL", "escrow missing after deliver"),
                    },
                    Err(outcome) => outcome,
                }
            }
            "escrow.dispute" => {
                let Some(id) = arg_str("escrowId") else {
                    return Self::failed("INVALID_REQUEST", "args.escrowId is required");
                };
                let escrow = match self.escrow_of(&id).await {
                    Ok(e) => e,
                    Err(outcome) => return outcome,
                };
                if escrow.buyer_id != ctx.actor && !ctx.staff {
                    return Self::failed("UNAUTHORIZED", "Only the buyer (or staff) may dispute.");
                }
                let reason = arg_str("reason").unwrap_or_else(|| "buyer dispute via AI".into());
                let at = now_iso();
                let apply_id = id.clone();
                match self
                    .apply_escrow(&ctx.actor, "escrow.dispute", &id, None, |ledger, registry| {
                        dispute_escrow(
                            ledger,
                            registry,
                            &DisputeEscrowCommand {
                                escrow_id: apply_id.clone(),
                                idempotency_key: None,
                                at: Some(at.clone()),
                                reason: reason.clone(),
                            },
                        )
                        .map(to_outcome)
                    })
                    .await
                {
                    Ok(res) => match res.escrow {
                        Some(e) => ToolOutcome::Data {
                            value: serde_json::json!({"id": e.id, "status": e.status.as_str()}),
                            summary: format!(
                                "Dispute opened on escrow {} — funds stay locked.",
                                e.id
                            ),
                        },
                        None => Self::failed("INTERNAL", "escrow missing after dispute"),
                    },
                    Err(outcome) => outcome,
                }
            }
            "product.create" => {
                let Some(title) = arg_str("title") else {
                    return Self::failed("INVALID_REQUEST", "args.title is required");
                };
                let Some(price) = args.get("priceDin").and_then(|v| v.as_i64()) else {
                    return Self::failed("INVALID_REQUEST", "args.priceDin must be an integer");
                };
                let row = ProductRow {
                    id: format!("prod:{}", uuid::Uuid::now_v7()),
                    seller_id: TREASURY_ACCOUNT_ID.to_owned(),
                    title: title.clone(),
                    description: arg_str("description").unwrap_or_default(),
                    category: arg_str("category"),
                    price_din: price,
                    delivery_payload: arg_str("deliveryPayload"),
                    active: true,
                    created_at: now_iso(),
                };
                match self.store.insert_product(&row).await {
                    Ok(()) => ToolOutcome::Data {
                        value: serde_json::json!({"id": row.id, "title": row.title, "priceDin": row.price_din}),
                        summary: format!("Product {} created at {} Đ.", row.id, row.price_din),
                    },
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }
            "kb.upsert" => {
                let Some(title) = arg_str("title") else {
                    return Self::failed("INVALID_REQUEST", "args.title is required");
                };
                let Some(body) = arg_str("body") else {
                    return Self::failed("INVALID_REQUEST", "args.body is required");
                };
                let id = arg_str("id").unwrap_or_else(|| format!("kb:{}", uuid::Uuid::now_v7()));
                match self.store.upsert_kb(&id, &title, &body).await {
                    Ok(()) => ToolOutcome::Data {
                        value: serde_json::json!({"id": id, "title": title}),
                        summary: format!("Knowledge entry {id} saved."),
                    },
                    Err(e) => Self::failed("INTERNAL", e.to_string()),
                }
            }

            // ── پولی‌ها — همیشه HITL-gated / staff ─────────────────────────
            "escrow_refund" => {
                let Some(id) = arg_str("escrowId") else {
                    return Self::failed("INVALID_REQUEST", "args.escrowId is required");
                };
                let escrow = match self.escrow_of(&id).await {
                    Ok(e) => e,
                    Err(outcome) => return outcome,
                };
                // مادهٔ ۷ — بازپرداخت هرگز مستقیم اجرا نمی‌شود؛ صف HITL و
                // اسکرو همان‌جا می‌ماند (معیار پذیرش فاز ۶)
                let hitl_id = format!("hitl:refund:{id}");
                match self
                    .enqueue_hitl(&hitl_id, "ESCROW_REFUND", &id, escrow.amount.as_i64(), &ctx.actor, arg_str("note"))
                    .await
                {
                    Ok(()) => ToolOutcome::Pending {
                        summary: format!(
                            "Refund of escrow {id} ({} Đ) queued for human approval (HITL) — escrow stays {}.",
                            escrow.amount.as_i64(),
                            escrow.status.as_str()
                        ),
                    },
                    Err(outcome) => outcome,
                }
            }
            "escrow_release" => {
                let Some(id) = arg_str("escrowId") else {
                    return Self::failed("INVALID_REQUEST", "args.escrowId is required");
                };
                let escrow = match self.escrow_of(&id).await {
                    Ok(e) => e,
                    Err(outcome) => return outcome,
                };
                if escrow.seller_id != ctx.actor && !ctx.staff {
                    return Self::failed("UNAUTHORIZED", "Only the buyer (or staff) may release.");
                }
                // staff = تأیید انسانی معتبر (approved_by_admin: true مثل /hitl approve)
                let key = format!("ai:release:{id}");
                let at = now_iso();
                let apply_id = id.clone();
                match self
                    .apply_escrow(
                        &ctx.actor,
                        "escrow.release",
                        &id,
                        Some(key.clone()),
                        |ledger, registry| {
                            release_escrow(
                                ledger,
                                registry,
                                &ReleaseEscrowCommand {
                                    escrow_id: apply_id.clone(),
                                    idempotency_key: Some(key.clone()),
                                    at: Some(at.clone()),
                                    approved_by_admin: true,
                                },
                            )
                            .map(to_outcome)
                        },
                    )
                    .await
                {
                    Ok(res) => {
                        match res.escrow {
                            Some(e) if e.status.as_str() == "RELEASED" => {
                                let amount = e.amount.as_i64();
                                let tax = dropagentx_domain::tax_of(e.amount)
                                    .map(|t| t.as_i64())
                                    .unwrap_or(0);
                                ToolOutcome::Data {
                                value: serde_json::json!({"id": e.id, "status": "RELEASED", "amount": amount, "tax": tax}),
                                summary: format!("Escrow {id} settled 90/10 — seller gets {} Đ, treasury {} Đ.", amount - tax, tax),
                            }
                            }
                            Some(e) => Self::failed(
                                "INVALID_TRANSITION",
                                format!("Cannot release escrow in status {}", e.status.as_str()),
                            ),
                            None => Self::failed("INTERNAL", "escrow missing after release"),
                        }
                    }
                    Err(outcome) => outcome,
                }
            }

            // ── دستوری‌ها ──────────────────────────────────────────────────
            "ai.handoff" => {
                let note = arg_str("note").or_else(|| arg_str("reason"));
                let hitl_id = format!("hitl:handoff:{}", uuid::Uuid::now_v7());
                match self
                    .enqueue_hitl(&hitl_id, "HUMAN_HANDOFF", "handoff:pending", 0, &ctx.actor, note)
                    .await
                {
                    Ok(()) => ToolOutcome::Pending {
                        summary: format!("Queued for the council — request {hitl_id}."),
                    },
                    Err(outcome) => outcome,
                }
            }

            // مهارت‌های بدون ایجنت مجاز — هرگز نباید اینجا برسند (گیت موتور)
            _ => Self::failed(
                "SKILL_NOT_ALLOWED",
                "This skill is not executable by any AI agent (council-only via dashboard).",
            ),
        }
    }

    async fn tokens_today(&self, user_id: &str) -> Result<i64, dropagentx_ai::GateError> {
        self.store
            .ai_tokens_today(user_id)
            .await
            .map(|(used, _)| used)
            .map_err(|e| dropagentx_ai::GateError::Backend(e.to_string()))
    }

    async fn record_usage(&self, record: AiUsageRecord) -> Result<(), dropagentx_ai::GateError> {
        let row = dropagentx_db::AiUsageRow {
            user_id: record.user_id,
            agent: record.agent,
            provider: record.provider,
            model: record.model,
            kind: record.kind,
            prompt_tokens: record.prompt_tokens,
            completion_tokens: record.completion_tokens,
            total_tokens: record.total_tokens,
            outcome: record.outcome,
        };
        self.store
            .insert_ai_usage(&row)
            .await
            .map_err(|e| dropagentx_ai::GateError::Backend(e.to_string()))
    }
}
