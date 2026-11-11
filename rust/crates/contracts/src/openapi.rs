//! سازندهٔ OpenAPI 3.1 — پورت یک‌به‌یک `openapi.ts` (ADR-005).
//!
//! در TS، zod v4 مستقیم به JSON Schema (2020-12) تبدیل می‌شود — همان چیزی که
//! OpenAPI 3.1 مصرف می‌کند؛ بدون پل ثالث، بدون drift. در Rust همان نقش را
//! deriveهای utoipa دارند: اسکیمای هر DTO از همان type های serde ساخته می‌شود
//! و این ماژول فقط «مونتاژ» سند را پورت می‌کند (همان کلیدها، همان ترتیب
//! مجموعه‌ها، همان پاسخ 4XX و همان security scheme).
//!
//! تفاوت نحوه‌ای شناخته‌شده (فقط formatting — تست گلدن نگه می‌دارد):
//! * zod در هر اسکیما `$schema` می‌گذارد؛ utoipa نمی‌گذارد.
//! * zod اشیاء را `additionalProperties: false` می‌بندد؛ utoipa باز می‌گذارد
//!   (تطبیق سمت سرور با serde همان کار را می‌کند).
//! * `.nullable()` در zod → `anyOf:[T,null]`؛ utoipa → `nullable: true`.
//! * zod `const` → utoipa `enum` تک‌عضوی (یا `boolean` برای literal true).

use serde_json::{json, Map, Value};
use utoipa::PartialSchema;
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::Schema;

/// فعل HTTP — پورت `HttpMethod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// GET
    Get,
    /// POST
    Post,
    /// PATCH
    Patch,
    /// PUT
    Put,
    /// DELETE
    Delete,
}

impl Method {
    /// رشتهٔ OpenAPI — lowercase.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Patch => "patch",
            Self::Put => "put",
            Self::Delete => "delete",
        }
    }
}

/// سطح امنیت — پورت `Security`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// کلید ادمین لازم دارد (`kiaAdminKey`)
    Admin,
    /// عمومی
    Public,
}

/// اطلاعات سند — پورت `OpenApiInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiInfo {
    /// عنوان
    pub title: String,
    /// نسخه
    pub version: String,
    /// توضیح
    pub description: String,
}

/// پارامتر مسیر/کوئری — اسکیمای آن مستقیم به شکل JSON Schema (پس از نرمال‌سازی).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// نام پارامتر
    pub name: &'static str,
    /// `path` یا `query`
    pub location: &'static str,
    /// اجباری؟ (path همیشه true؛ query false)
    pub required: bool,
    /// JSON Schema پارامتر (مثل خروجی zod برای همان فیلد)
    pub schema: Value,
}

/// تعریف یک مسیر — پورت `ApiRouteDef` (بدون هندلر؛ هندلرها فاز ۴).
#[derive(Clone)]
pub struct ApiRouteDef {
    /// فعل
    pub method: Method,
    /// مسیر سبک OpenAPI با `{placeholders}` — مثل `/escrows/{id}/fund`
    pub path: &'static str,
    /// operationId — یکتا و همان TS
    pub operation_id: &'static str,
    /// خلاصه
    pub summary: &'static str,
    /// برچسب گروه
    pub tag: &'static str,
    /// سطح امنیت
    pub security: Security,
    /// پارامترهای path + query (به همان ترتیب TS)
    pub params: Vec<Param>,
    /// اسکیمای بدنه (inline از derive)
    pub body: Option<RefOr<Schema>>,
    /// اسکیمای پاسخ موفق (inline از derive)
    pub response: RefOr<Schema>,
    /// کد موفقیت — پیش‌فرض ۲۰۰؛ ساخت‌ها ۲۰۱
    pub status: u16,
}

/// تبدیل `/escrows/{id}/fund` → `/escrows/:id/fund` (سبک Fastify) — پورت `toFastifyPath`.
#[must_use]
pub fn fastify_path(path: &str) -> String {
    // همان رفتار regex با دست — {name} → :name
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            out.push(':');
            // تا بستن آکولاد، نام را کپی کن
            for inner in chars.by_ref() {
                if inner == '}' {
                    break;
                }
                out.push(inner);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// اطلاعات واقعی سند — پورت `OPENAPI_INFO` در `index.ts`.
#[must_use]
pub fn openapi_info() -> OpenApiInfo {
    OpenApiInfo {
        title: "DropAgentX v3 — Unified Core API".into(),
        version: crate::VERSION.into(),
        description: "Admin service API over the pure domain core: integer money, immutable double-entry ledger, escrow state machine, 10-article monetary charter. Errors follow { error: { code, message } } — codes are additive.".into(),
    }
}

/// اسکیمای خطای 4XX — کپی همان آبجکت literal در `buildOpenApiDoc` (منبع واحد TS).
fn error_4xx_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "error": {
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "message": { "type": "string" },
                    "meta": {
                        "type": "object",
                        "additionalProperties": { "type": ["string", "number"] }
                    }
                },
                "required": ["code", "message"]
            }
        },
        "required": ["error"]
    })
}

/// ساخت سند OpenAPI 3.1 — پورت `buildOpenApiDoc`.
///
/// ورودی: رجیستری مسیرها (`routes.rs`) + info. خروجی: `serde_json::Value`
/// آمادهٔ سرو کردن روی `/api/v1/openapi.json` (فاز ۴) و diff با گلدن TS.
#[must_use]
pub fn build_openapi_doc(routes: &[ApiRouteDef], info: &OpenApiInfo) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    let mut tags: Vec<Value> = Vec::new();

    for route in routes {
        let entry = paths
            .entry(route.path.to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        // پارامترها — path و query به ترتیب تعریف
        let parameters: Vec<Value> = route
            .params
            .iter()
            .map(|p| {
                json!({
                    "name": p.name,
                    "in": p.location,
                    "required": p.required,
                    "schema": p.schema,
                })
            })
            .collect();

        let mut responses = Map::new();
        responses.insert(
            route.status.to_string(),
            json!({
                "description": "Success",
                "content": { "application/json": { "schema": to_value(&route.response) } },
            }),
        );
        responses.insert("4XX".to_string(), json!({
            "description": "Domain or validation failure",
            "content": { "application/json": { "schema": error_4xx_schema() } },
        }));

        let mut operation = Map::new();
        operation.insert("operationId".into(), json!(route.operation_id));
        operation.insert("summary".into(), json!(route.summary));
        operation.insert("tags".into(), json!([route.tag]));
        if !parameters.is_empty() {
            operation.insert("parameters".into(), Value::Array(parameters));
        }
        if let Some(body) = &route.body {
            operation.insert(
                "requestBody".into(),
                json!({
                    "required": true,
                    "content": { "application/json": { "schema": to_value(body) } },
                }),
            );
        }
        operation.insert("responses".into(), Value::Object(responses));
        if route.security == Security::Admin {
            operation.insert("security".into(), json!([{ "kiaAdminKey": [] }]));
        }

        if let Some(obj) = entry.as_object_mut() {
            obj.insert(route.method.as_str().to_string(), Value::Object(operation));
        }

        // تگ‌ها به ترتیب اولین ظهور — مثل `[...new Set(routes.map(r => r.tag))]`
        if !tags.iter().any(|t| t == &json!({ "name": route.tag })) {
            tags.push(json!({ "name": route.tag }));
        }
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": info.title,
            "version": info.version,
            "description": info.description,
        },
        "servers": [{ "url": "/api/v1" }],
        "tags": tags,
        "components": {
            "securitySchemes": {
                "kiaAdminKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-KIA-Key",
                    "description": "Fail-closed admin key (KIA_ADMIN_KEY). Missing key on the server disables admin endpoints entirely.",
                }
            }
        },
        "paths": Value::Object(paths),
    })
}

/// ساخت سند نهایی با inline کردن `$ref`ها (مثل خروجی zod).
/// نسخهٔ بازگشتیِ عمومی — رجیستری `api_routes` را می‌گیرد و سند کامل می‌دهد.
#[must_use]
pub fn build_openapi_doc_inlined(routes: &[ApiRouteDef], info: &OpenApiInfo) -> Value {
    let mut doc = build_openapi_doc(routes, info);
    let components: Map<String, Value> = component_schemas()
        .into_iter()
        .map(|(name, schema)| (name.to_string(), to_value(&schema)))
        .collect();
    inline_refs(&mut doc, &components, 0);
    doc
}

/// تبدیل RefOr<Schema> به Value (utoipa از قبل Serialize دارد).
fn to_value(ref_or: &RefOr<Schema>) -> Value {
    serde_json::to_value(ref_or).unwrap_or(Value::Null)
}

// ── inlining $ref ها — سند نهایی مثل zod کاملاً inline است ──────────────────

/// رجیستری کامپوننت‌ها: نام → اسکیمای کامل. برای حل `$ref`های utoipa.
/// هر DTO که داخل اسکیمای مسیرها ارجاع می‌شود باید این‌جا ثبت شود؛
/// تست گلدن هر `$ref` حل‌نشده را خطا می‌گیرد.
pub(crate) fn component_schemas() -> Vec<(&'static str, RefOr<Schema>)> {
    use crate::accounts::{
        AccountListResponse, AccountView, BalanceResponse, CreateAccountRequest,
    };
    use crate::ai::{
        AiAgentName, AiAgentView, AiAgentsListResponse, AiChatRequest, AiChatResponse, AiChatRole,
        AiSkillKind, AiSkillView, AiSkillsListResponse, AiToolTrace, AiUsage, AiUsageView,
        HistoryMessage, HistoryRole, AiVia,
    };
    use crate::common::{ApiError, Din, ErrorEnvelope, EscrowStatus, Id, IdempotencyKey, IsoDateTime, MetaValue};
    use crate::dashboard::*;
    use crate::economy::{Currency, DinSymbol, EconomySummary};
    use crate::escrows::*;
    use crate::hitl::*;
    use crate::system::{BackupCreated, HealthResponse, HealthService, HealthStatus};
    use crate::wallet::{BurnRequest, MintRequest, WalletMoveResponse};

    vec![
        // primitive with annotations — utoipa برای i64 حاشیه‌نویسی‌شده ref می‌سازد
        ("i64", utoipa::openapi::schema::Object::builder()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .format(Some(utoipa::openapi::schema::SchemaFormat::KnownFormat(
                utoipa::openapi::KnownFormat::Int64,
            )))
            .build()
            .into()),
        // common
        ("Din", <Din as PartialSchema>::schema()),
        ("Id", <Id as PartialSchema>::schema()),
        ("IdempotencyKey", <IdempotencyKey as PartialSchema>::schema()),
        ("IsoDateTime", <IsoDateTime as PartialSchema>::schema()),
        ("EscrowStatus", <EscrowStatus as PartialSchema>::schema()),
        ("MetaValue", <MetaValue as PartialSchema>::schema()),
        ("ApiError", <ApiError as PartialSchema>::schema()),
        ("ErrorEnvelope", <ErrorEnvelope as PartialSchema>::schema()),
        // accounts
        ("CreateAccountRequest", <CreateAccountRequest as PartialSchema>::schema()),
        ("AccountView", <AccountView as PartialSchema>::schema()),
        ("AccountListResponse", <AccountListResponse as PartialSchema>::schema()),
        ("BalanceResponse", <BalanceResponse as PartialSchema>::schema()),
        // wallet
        ("MintRequest", <MintRequest as PartialSchema>::schema()),
        ("BurnRequest", <BurnRequest as PartialSchema>::schema()),
        ("WalletMoveResponse", <WalletMoveResponse as PartialSchema>::schema()),
        // escrows
        ("CreateEscrowRequest", <CreateEscrowRequest as PartialSchema>::schema()),
        ("EscrowView", <EscrowView as PartialSchema>::schema()),
        ("EscrowListResponse", <EscrowListResponse as PartialSchema>::schema()),
        ("FundEscrowRequest", <FundEscrowRequest as PartialSchema>::schema()),
        ("DeliverEscrowRequest", <DeliverEscrowRequest as PartialSchema>::schema()),
        ("ReleaseEscrowRequest", <ReleaseEscrowRequest as PartialSchema>::schema()),
        ("DisputeEscrowRequest", <DisputeEscrowRequest as PartialSchema>::schema()),
        ("RefundEscrowRequest", <RefundEscrowRequest as PartialSchema>::schema()),
        ("CancelEscrowRequest", <CancelEscrowRequest as PartialSchema>::schema()),
        ("ExpireEscrowRequest", <ExpireEscrowRequest as PartialSchema>::schema()),
        ("EscrowActionResponse", <EscrowActionResponse as PartialSchema>::schema()),
        // hitl
        ("HitlKind", <HitlKind as PartialSchema>::schema()),
        ("HitlStatus", <HitlStatus as PartialSchema>::schema()),
        ("CreateHitlRequest", <CreateHitlRequest as PartialSchema>::schema()),
        ("HitlDecisionRequest", <HitlDecisionRequest as PartialSchema>::schema()),
        ("HitlView", <HitlView as PartialSchema>::schema()),
        ("HitlListResponse", <HitlListResponse as PartialSchema>::schema()),
        ("HitlSweepResponse", <HitlSweepResponse as PartialSchema>::schema()),
        ("HitlActionResponse", <HitlActionResponse as PartialSchema>::schema()),
        // economy
        ("Currency", <Currency as PartialSchema>::schema()),
        ("DinSymbol", <DinSymbol as PartialSchema>::schema()),
        ("EconomySummary", <EconomySummary as PartialSchema>::schema()),
        // system
        ("HealthStatus", <HealthStatus as PartialSchema>::schema()),
        ("HealthService", <HealthService as PartialSchema>::schema()),
        ("HealthResponse", <HealthResponse as PartialSchema>::schema()),
        ("BackupCreated", <BackupCreated as PartialSchema>::schema()),
        // ai
        ("AiAgentName", <AiAgentName as PartialSchema>::schema()),
        ("HistoryMessage", <HistoryMessage as PartialSchema>::schema()),
        ("AiSkillKind", <AiSkillKind as PartialSchema>::schema()),
        ("AiChatRole", <AiChatRole as PartialSchema>::schema()),
        ("AiVia", <AiVia as PartialSchema>::schema()),
        ("HistoryRole", <HistoryRole as PartialSchema>::schema()),
        ("AiAgentView", <AiAgentView as PartialSchema>::schema()),
        ("AiSkillView", <AiSkillView as PartialSchema>::schema()),
        ("AiAgentsListResponse", <AiAgentsListResponse as PartialSchema>::schema()),
        ("AiSkillsListResponse", <AiSkillsListResponse as PartialSchema>::schema()),
        ("AiToolTrace", <AiToolTrace as PartialSchema>::schema()),
        ("AiUsage", <AiUsage as PartialSchema>::schema()),
        ("AiChatRequest", <AiChatRequest as PartialSchema>::schema()),
        ("AiChatResponse", <AiChatResponse as PartialSchema>::schema()),
        ("AiUsageView", <AiUsageView as PartialSchema>::schema()),
        // dashboard
        ("DashPerson", <DashPerson as PartialSchema>::schema()),
        ("BrandResponse", <BrandResponse as PartialSchema>::schema()),
        ("BrandData", <BrandData as PartialSchema>::schema()),
        ("OverviewResponse", <OverviewResponse as PartialSchema>::schema()),
        ("OverviewData", <OverviewData as PartialSchema>::schema()),
        ("SeriesPoint", <SeriesPoint as PartialSchema>::schema()),
        ("TopProduct", <TopProduct as PartialSchema>::schema()),
        ("RecentOrder", <RecentOrder as PartialSchema>::schema()),
        ("ProductStatus", <ProductStatus as PartialSchema>::schema()),
        ("ProductView", <ProductView as PartialSchema>::schema()),
        ("ProductListResponse", <ProductListResponse as PartialSchema>::schema()),
        ("ProductListData", <ProductListData as PartialSchema>::schema()),
        ("CreateProductRequest", <CreateProductRequest as PartialSchema>::schema()),
        ("UpdateProductRequest", <UpdateProductRequest as PartialSchema>::schema()),
        ("ProductMutationResponse", <ProductMutationResponse as PartialSchema>::schema()),
        ("OrderView", <OrderView as PartialSchema>::schema()),
        ("OrderListResponse", <OrderListResponse as PartialSchema>::schema()),
        ("OrderListData", <OrderListData as PartialSchema>::schema()),
        ("OrderAction", <OrderAction as PartialSchema>::schema()),
        ("OrderActionRequest", <OrderActionRequest as PartialSchema>::schema()),
        ("OrderActionResponse", <OrderActionResponse as PartialSchema>::schema()),
        ("OrderActionData", <OrderActionData as PartialSchema>::schema()),
        ("DashUser", <DashUser as PartialSchema>::schema()),
        ("DashUserListResponse", <DashUserListResponse as PartialSchema>::schema()),
        ("DashUserListData", <DashUserListData as PartialSchema>::schema()),
        ("DashWallet", <DashWallet as PartialSchema>::schema()),
        ("DashWalletListResponse", <DashWalletListResponse as PartialSchema>::schema()),
        ("DashWalletListData", <DashWalletListData as PartialSchema>::schema()),
        ("DashListResponse", <DashListResponse as PartialSchema>::schema()),
        ("DashListData", <DashListData as PartialSchema>::schema()),
        ("LeaderboardType", <LeaderboardType as PartialSchema>::schema()),
        ("LeaderboardResponse", <LeaderboardResponse as PartialSchema>::schema()),
        ("LeaderboardData", <LeaderboardData as PartialSchema>::schema()),
        ("LeaderboardEntry", <LeaderboardEntry as PartialSchema>::schema()),
        ("RiskLevel", <RiskLevel as PartialSchema>::schema()),
        ("ApprovalView", <ApprovalView as PartialSchema>::schema()),
        ("ApprovalStatusFilter", <ApprovalStatusFilter as PartialSchema>::schema()),
        ("ApprovalListResponse", <ApprovalListResponse as PartialSchema>::schema()),
        ("ApprovalListData", <ApprovalListData as PartialSchema>::schema()),
        ("ApprovalDecisionRequest", <ApprovalDecisionRequest as PartialSchema>::schema()),
        ("ApprovalDecisionResponse", <ApprovalDecisionResponse as PartialSchema>::schema()),
        ("ApprovalDecisionData", <ApprovalDecisionData as PartialSchema>::schema()),
        ("SettingsListResponse", <SettingsListResponse as PartialSchema>::schema()),
        ("SettingsListData", <SettingsListData as PartialSchema>::schema()),
        ("SettingEntry", <SettingEntry as PartialSchema>::schema()),
        ("SettingsPutRequest", <SettingsPutRequest as PartialSchema>::schema()),
        ("SecurityResponse", <SecurityResponse as PartialSchema>::schema()),
        ("SecurityData", <SecurityData as PartialSchema>::schema()),
        ("SecurityLayer", <SecurityLayer as PartialSchema>::schema()),
        ("AuditRow", <AuditRow as PartialSchema>::schema()),
    ]
}

/// حل `$ref`ها به‌صورت بازگشتی — مثل zod همه‌چیز inline می‌شود.
/// sibling های `$ref` (مثل `default`) روی تایپِ حل‌شده override می‌شوند.
fn inline_refs(value: &mut Value, components: &Map<String, Value>, depth: usize) {
    const MAX_DEPTH: usize = 64;
    if depth > MAX_DEPTH {
        return; // دادهٔ بازگشتی در قراردادها وجود ندارد — گارد دفاعی
    }
    match value {
        Value::Object(map) => {
            if let Some(Value::String(reference)) = map.get("$ref") {
                if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                    if let Some(target) = components.get(name) {
                        let mut merged = target.clone();
                        if let Value::Object(t) = &mut merged {
                            for (k, v) in map.iter() {
                                if k != "$ref" {
                                    t.insert(k.clone(), v.clone());
                                }
                            }
                        }
                        *value = merged;
                        inline_refs(value, components, depth + 1);
                        return;
                    }
                }
            }
            for (_, v) in map.iter_mut() {
                inline_refs(v, components, depth + 1);
            }
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                inline_refs(v, components, depth + 1);
            }
        }
        _ => {}
    }
}

/// صفحهٔ `/api/v1/docs` — پورت HTML ردوکِ `index.ts`.
#[must_use]
pub fn docs_page_html() -> String {
    [
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">",
        "<title>DropAgentX v3 API — OpenAPI</title>",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
        "<style>body{margin:0;background:#0a0f14;color:#dbe4ee;font-family:system-ui,sans-serif}</style>",
        "</head><body>",
        "<redoc spec-url=\"/api/v1/openapi.json\"></redoc>",
        "<script src=\"https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js\"></script>",
        "<noscript>This page needs JavaScript. The raw spec lives at /api/v1/openapi.json</noscript>",
        "</body></html>",
    ]
    .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Din;
    use utoipa::PartialSchema;

    #[test]
    fn fastify_placeholder_conversion_both_ways() {
        assert_eq!(fastify_path("/escrows/{id}/fund"), "/escrows/:id/fund");
        assert_eq!(fastify_path("/health"), "/health");
        assert_eq!(fastify_path("/orders/{id}/status"), "/orders/:id/status");
    }

    #[test]
    fn doc_structure_mirrors_ts_builder() {
        use crate::common::Din;
        use serde_json::json;

        let routes = vec![ApiRouteDef {
            method: Method::Post,
            path: "/wallet/mint",
            operation_id: "mint",
            summary: "Mint dinar (admin)",
            tag: "wallet",
            security: Security::Admin,
            params: Vec::new(),
            body: Some(<Din as PartialSchema>::schema()),
            response: <Din as PartialSchema>::schema(),
            status: 201,
        }];
        let info = OpenApiInfo { title: "t".into(), version: "v".into(), description: "d".into() };
        let doc = build_openapi_doc(&routes, &info);

        assert_eq!(doc["openapi"], json!("3.1.0"));
        assert_eq!(doc["servers"][0]["url"], json!("/api/v1"));
        assert_eq!(doc["tags"], json!([{ "name": "wallet" }]));
        let mint = &doc["paths"]["/wallet/mint"]["post"];
        assert_eq!(mint["operationId"], json!("mint"));
        assert_eq!(mint["security"], json!([{ "kiaAdminKey": [] }]));
        assert_eq!(mint["requestBody"]["required"], json!(true));
        assert!(mint["responses"]["201"]["description"] == json!("Success"));
        assert!(mint["responses"]["4XX"]["content"]["application/json"]["schema"]["required"]
            == json!(["error"]));
        assert_eq!(
            doc["components"]["securitySchemes"]["kiaAdminKey"]["name"],
            json!("X-KIA-Key")
        );
    }

    #[test]
    fn tags_follow_first_appearance_order() {
        use serde_json::json;
        let dummy = <Din as PartialSchema>::schema();
        let mk = |m: Method, p: &'static str, tag: &'static str| ApiRouteDef {
            method: m,
            path: p,
            operation_id: "op",
            summary: "s",
            tag,
            security: Security::Public,
            params: Vec::new(),
            body: None,
            response: dummy.clone(),
            status: 200,
        };
        let routes = vec![
            mk(Method::Get, "/a", "system"),
            mk(Method::Get, "/b", "economy"),
            mk(Method::Post, "/c", "system"),
        ];
        let doc = build_openapi_doc(&routes, &openapi_info());
        assert_eq!(doc["tags"], json!([{ "name": "system" }, { "name": "economy" }]));
    }

    #[test]
    fn docs_page_points_at_openapi_json() {
        let html = docs_page_html();
        assert!(html.contains("<redoc spec-url=\"/api/v1/openapi.json\">"));
        assert!(html.starts_with("<!doctype html>"));
    }
}
