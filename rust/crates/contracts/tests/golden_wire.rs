//! تست گلدن وایر — سازگاری JSON با TS (فاز ۳، معیار پذیرش ADR-009).
//!
//! هر ۵۵ نمونهٔ `golden-wire.json` از **خود** zod ساخته و تأیید شده‌اند
//! (`scripts/ts-golden/generate.ts`). این تست ثابت می‌کند:
//! ۱. هر نمونه در Rust بدون خطا parse می‌شود (deserialization سخت‌گیرانه).
//! ۲. serialize مجدد در Rust همان JSON را می‌دهد (camelCase، null صریح،
//!    optional غایب، صحیح امن) — یعنی کلاینت TS همان شکل را می‌فهمد.


// تست‌ها جای مشروع expect هستند — ورودی گلدن باید شکست صریح بدهد
#![allow(clippy::expect_used)]

use dropagentx_contracts::accounts::*;
use dropagentx_contracts::ai::*;
use dropagentx_contracts::common::{Din, ErrorEnvelope, EscrowStatus, Id, IdempotencyKey};
use dropagentx_contracts::dashboard::*;
use dropagentx_contracts::economy::EconomySummary;
use dropagentx_contracts::escrows::*;
use dropagentx_contracts::hitl::*;
use dropagentx_contracts::system::{BackupCreated, HealthResponse};
use dropagentx_contracts::wallet::*;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn golden() -> Value {
    let raw = std::fs::read_to_string(format!("{FIXTURES}/golden-wire.json")).expect("golden-wire.json");
    serde_json::from_str(&raw).expect("golden wire parses")
}

/// چرخهٔ کامل: JSON نمونه → تایپ Rust → JSON — باید با ورودی برابر بماند.
fn roundtrip<T: DeserializeOwned + serde::Serialize>(sample: &Value) {
    let typed: T = serde_json::from_value(sample.clone())
        .unwrap_or_else(|e| panic!("parse failed: {e}\nsample: {sample}"));
    let back = serde_json::to_value(&typed).expect("serialize works");
    assert_eq!(&back, sample, "reserialization drifted\nsample: {sample}\nback:   {back}");
}

#[test]
fn all_55_golden_samples_roundtrip_through_rust_types() {
    let g = golden();
    let count = g.as_object().expect("object").len();
    assert_eq!(count, 55, "fixture has 55 samples");

    // ── پاسخ‌ها ──
    roundtrip::<HealthResponse>(&g["health"]);
    roundtrip::<EconomySummary>(&g["economy"]);
    roundtrip::<AccountView>(&g["accountView"]);
    roundtrip::<AccountListResponse>(&g["accountList"]);
    roundtrip::<BalanceResponse>(&g["balance"]);
    roundtrip::<WalletMoveResponse>(&g["walletMove"]);
    roundtrip::<EscrowView>(&g["escrowView"]);
    roundtrip::<EscrowListResponse>(&g["escrowList"]);
    roundtrip::<EscrowActionResponse>(&g["escrowAction"]);
    roundtrip::<HitlView>(&g["hitlView"]);
    roundtrip::<HitlListResponse>(&g["hitlList"]);
    roundtrip::<HitlSweepResponse>(&g["hitlSweep"]);
    roundtrip::<HitlActionResponse>(&g["hitlAction"]);
    roundtrip::<BackupCreated>(&g["backupCreated"]);
    roundtrip::<AiChatResponse>(&g["aiChat"]);
    roundtrip::<AiAgentsListResponse>(&g["aiAgents"]);
    roundtrip::<AiSkillsListResponse>(&g["aiSkills"]);
    roundtrip::<AiUsageView>(&g["aiUsage"]);
    roundtrip::<BrandResponse>(&g["brand"]);
    roundtrip::<OverviewResponse>(&g["overview"]);
    roundtrip::<ProductView>(&g["productView"]);
    roundtrip::<ProductListResponse>(&g["productList"]);
    roundtrip::<ProductMutationResponse>(&g["productMutation"]);
    roundtrip::<OrderView>(&g["orderView"]);
    roundtrip::<OrderListResponse>(&g["orderList"]);
    roundtrip::<OrderActionResponse>(&g["orderAction"]);
    roundtrip::<DashUserListResponse>(&g["dashUserList"]);
    roundtrip::<DashWalletListResponse>(&g["dashWalletList"]);
    roundtrip::<DashListResponse>(&g["dashList"]);
    roundtrip::<LeaderboardResponse>(&g["leaderboard"]);
    roundtrip::<ApprovalListResponse>(&g["approvalList"]);
    roundtrip::<ApprovalDecisionResponse>(&g["approvalDecision"]);
    roundtrip::<SettingsListResponse>(&g["settingsList"]);
    roundtrip::<SecurityResponse>(&g["security"]);

    // ── درخواست‌ها (چیزی که کلاینت TS می‌فرستد) ──
    roundtrip::<CreateAccountRequest>(&g["req:createAccount"]);
    roundtrip::<MintRequest>(&g["req:mint"]);
    roundtrip::<BurnRequest>(&g["req:burn"]);
    roundtrip::<CreateEscrowRequest>(&g["req:createEscrow"]);
    roundtrip::<FundEscrowRequest>(&g["req:fund"]);
    roundtrip::<DeliverEscrowRequest>(&g["req:deliver"]);
    roundtrip::<ReleaseEscrowRequest>(&g["req:release"]);
    roundtrip::<DisputeEscrowRequest>(&g["req:dispute"]);
    roundtrip::<RefundEscrowRequest>(&g["req:refund"]);
    roundtrip::<CancelEscrowRequest>(&g["req:cancel"]);
    roundtrip::<ExpireEscrowRequest>(&g["req:expire"]);
    roundtrip::<CreateHitlRequest>(&g["req:createHitl"]);
    roundtrip::<HitlDecisionRequest>(&g["req:hitlDecision"]);
    roundtrip::<AiChatRequest>(&g["req:aiChat"]);
    roundtrip::<CreateProductRequest>(&g["req:createProduct"]);
    roundtrip::<UpdateProductRequest>(&g["req:updateProduct"]);
    roundtrip::<OrderActionRequest>(&g["req:orderAction"]);
    roundtrip::<ApprovalDecisionRequest>(&g["req:approvalDecision"]);
    roundtrip::<SettingsPutRequest>(&g["req:settingsPut"]);

    // ── پاکت خطای وایر ──
    roundtrip::<ErrorEnvelope>(&g["wireError"]);
    roundtrip::<ErrorEnvelope>(&g["wireErrorNoMeta"]);
}

#[test]
fn malformed_wire_is_rejected_like_zod() {
    // وضعیت ناشناخته — TS client هم رد می‌کند (client.test.ts: FLOATING)
    assert!(serde_json::from_value::<EscrowStatus>(json!("FLOATING")).is_err());
    // پول float یا منفی — ADR-002
    assert!(serde_json::from_value::<Din>(json!(-1)).is_err());
    assert!(serde_json::from_value::<Din>(json!(9_007_199_254_740_992_i64)).is_err());
    // شناسهٔ نامعتبر
    assert!(serde_json::from_value::<Id>(json!("bad id!")).is_err());
    // کلید یکسان‌سازی کوتاه (< ۸) — همان نمونهٔ zod که در تولید گلدن رد شد
    assert!(serde_json::from_value::<IdempotencyKey>(json!("esc-1")).is_err());
    // پاسخ سلام ناقص — RESPONSE_MISMATCH در client.test.ts
    assert!(serde_json::from_value::<HealthResponse>(json!({"status": "ok"})).is_err());
    // پاکت خطای بدشکل
    assert!(serde_json::from_value::<ErrorEnvelope>(json!({"error": {"message": "no code"}})).is_err());
}
