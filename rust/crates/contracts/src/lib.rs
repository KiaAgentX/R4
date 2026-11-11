//! `dropagentx-contracts` — قراردادهای وایر (فاز ۳) — ADR-005/ADR-009.
//!
//! منبع واحد حقیقتِ سیم: DTOهای serde با deriveهای utoipa، سازندهٔ OpenAPI 3.1
//! (پورت `buildOpenApiDoc`)، طبقه‌بندی خطای HTTP و لایهٔ امنیت قراردادی.
//! سازگاری وایر با TS با گلدن‌فیکسچرِ تولیدشده از **خود** zod اثبات می‌شود:
//! ۵۵ نمونهٔ JSON و سند OpenAPI کامل — تست‌ها.diff صفر شکستن‌دهنده را نگه می‌دارند.
#![forbid(unsafe_code)]

pub mod accounts;
pub mod ai;
pub mod common;
pub mod dashboard;
pub mod error;
pub mod escrows;
pub mod economy;
pub mod hitl;
pub mod openapi;
pub mod routes;
pub mod security;
pub mod system;
pub mod wallet;

/// شناسهٔ کرِیت — برای smoke test و لاگ‌ها.
pub const NAME: &str = "dropagentx-contracts";

/// نسخهٔ کرِیت — از `Cargo.toml` خوانده می‌شود.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use accounts::{
    AccountListResponse, AccountView, BalanceResponse, CreateAccountRequest,
};
pub use ai::{
    AiAgentName, AiAgentView, AiAgentsListResponse, AiChatRequest, AiChatResponse, AiChatRole,
    AiSkillKind, AiSkillView, AiSkillsListResponse, AiToolTrace, AiUsage, AiUsageView,
    HistoryMessage, HistoryRole, AiVia,
};
pub use common::{
    ApiError, Din, Din as Dnx, ErrorEnvelope, EscrowStatus, Id, IdempotencyKey, IsoDateTime,
    Meta, MetaValue, DIN_MAX, ID_PATTERN,
};
pub use dashboard::{
    ApprovalDecisionRequest, ApprovalDecisionResponse, ApprovalStatusFilter, ApprovalView,
    AuditRow, BrandResponse, DashListResponse, DashPerson, DashUser, DashUserListResponse,
    DashWallet, DashWalletListResponse, LeaderboardResponse, LeaderboardType, OrderAction,
    OrderActionRequest, OrderActionResponse, OrderView, OverviewResponse, ProductListResponse,
    ProductMutationResponse, ProductStatus, ProductView, RiskLevel, SecurityResponse,
    SeriesPoint, SettingEntry, SettingsListResponse, SettingsPutRequest, SettingsPutRequest as SettingsPut,
    TopProduct, UpdateProductRequest, CreateProductRequest, ApprovalListResponse, OrderListResponse,
};
pub use economy::{Currency, DinSymbol, EconomySummary};
pub use error::{DomainFailure, HttpFailure, status_for_domain_code};
pub use escrows::{
    CancelEscrowRequest, CreateEscrowRequest, DeliverEscrowRequest, DisputeEscrowRequest,
    EscrowActionResponse, EscrowListResponse, EscrowView, ExpireEscrowRequest, FundEscrowRequest,
    RefundEscrowRequest, ReleaseEscrowRequest,
};
pub use hitl::{
    CreateHitlRequest, HitlActionResponse, HitlDecisionRequest, HitlKind, HitlListResponse,
    HitlStatus, HitlSweepResponse, HitlView,
};
pub use openapi::{
    build_openapi_doc, build_openapi_doc_inlined, docs_page_html, fastify_path, openapi_info,
    ApiRouteDef, Method, OpenApiInfo, Security,
};
pub use routes::api_routes;
pub use security::{
    make_test_init_data, verify_kia_key, verify_telegram_init_data, RateLimiter, TelegramIdentity,
};
pub use system::{BackupCreated, HealthResponse, HealthService, HealthStatus};
pub use wallet::{BurnRequest, MintRequest, WalletMoveResponse};
