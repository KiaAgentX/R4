//! `dropagentx-db` — لایهٔ ماندگاری Agent-One (فاز ۲ از ADR-009).
//!
//! پل بین دامنهٔ خالص فاز ۱ و SQLite واقعی v3 — با همان الگوی
//! `back/server/src/persistence.ts` و «همان» اسکیمای Prisma تا dual-run
//! فاز ۸ بدون تبدیل روی یک فایل بنشیند:
//!
//! - [`store`] — `V3Store`: بارگذاری → دامنهٔ خالص → درج، همه در
//!   `sqlx::Transaction`؛ ترازها همیشه مشتق از `Posting` (ADR-003)
//! - [`outbox`] — transactional outbox برای `DomainEvent`ها + audit
//! - [`wire`] — شکل سیمی JSON رویدادها (یکسان با `JSON.stringify` در TS)
//! - [`genesis`] — برنامهٔ خالص genesis + جدول گردکردن صریح ADR-002
//! - [`legacy`] — خوانندهٔ فقط‌خواندنی دیتابیس لگاسی (SQL خام)
//! - [`migrate`] — پورت `migrate-legacy.ts`: ۴۶۶ کاربر → `user:tg:<id>`،
//!   genesis یک‌بارمصرف با کلید قطعی `legacy:genesis:v1`
//! - [`proof`] — پورت `genesis-proof.ts`: ممیزی خارجی با SQL خام —
//!   Σ کیف‌ها = 187,672 Đ · Σ ردیف‌ها = 0
//!
//! اصل ساختاری: این کرِیت به `dropagentx-domain` بسته می‌شود، اما دامنه
//! هیچ‌چیز از اینجا نمی‌داند.
#![forbid(unsafe_code)]

pub mod error;
pub mod genesis;
pub mod legacy;
pub mod migrate;
pub mod outbox;
pub mod proof;
pub mod queries;
pub mod store;
pub mod wire;

#[cfg(test)]
mod integration_tests;

/// شناسه کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-db";

/// نسخه کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------- re-exports اصلی (API آرگونومیک) ----------
pub use error::{DbError, DbResult};
pub use genesis::{
    plan_genesis, GenesisCitizen, GenesisPlan, LegacyProductRow, LegacyUserRow, EXPECTED_SUPPLY,
    GENESIS_KEY, GENESIS_TX_ID,
};
pub use legacy::{read_legacy, LegacySnapshot};
pub use migrate::{migrate_legacy, reconcile, MigrateReport, ReconcileReport};
pub use proof::{genesis_proof, ProofReport};
pub use store::{
    now_iso, AccountBalance, ApplyOptions, ApplyResult, CommandOutcome, EconomySummary, OutboxRow,
    RecentPosting, V3Store,
};
pub use wire::{event_kind, event_payload};

// فاز ۴ — کوئری‌های صف HITL، محصولات داشبورد، audit و بکاپ
pub use queries::{
    deliver_order, AuditRowRead, BackupInfo, HitlDecision, HitlRow, ProductFilter, ProductRow,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(NAME, "dropagentx-db");
        assert!(!VERSION.is_empty());
        // ثابت‌های قرارداد genesis — همان TS
        assert_eq!(GENESIS_KEY, "legacy:genesis:v1");
        assert_eq!(GENESIS_TX_ID, "tx_legacy_genesis");
        assert_eq!(EXPECTED_SUPPLY, 187_672);
    }
}
