//! پورت ۱۱ تست قرارداد TS (فاز ۳) — `client.test.ts` (۸) + `openapi.test.ts` (۳).
//!
//! معادل `app.inject` فستیفای: روتر آزمایشی axum + `tower::ServiceExt::oneshot`
//! (ADR-009: «axum::body + oneshot»). مشتری تایپ‌شدهٔ آزمایشی همان نظم `client.ts`
//! را رعایت می‌کند: نتیجه‌ها Result هستند، کد خطای وایر عیناً عبور می‌کند و
//! بدنه‌های خراب به `MALFORMED_ERROR_BODY`/`RESPONSE_MISMATCH` نگاشت می‌شوند.

// تست‌ها جای مشروع expect هستند — ورودی گلدن باید شکست صریح بدهد
#![allow(clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tower::ServiceExt;

use dropagentx_contracts::accounts::BalanceResponse;
use dropagentx_contracts::common::{ErrorEnvelope, EscrowStatus};
use dropagentx_contracts::escrows::EscrowListResponse;
use dropagentx_contracts::openapi::{
    build_openapi_doc_inlined, fastify_path, ApiRouteDef, Method, OpenApiInfo, Security,
};
use dropagentx_contracts::system::HealthResponse;
use dropagentx_contracts::wallet::{MintRequest, WalletMoveResponse};
use dropagentx_contracts::{BalanceResponse as BR, EconomySummary};

const ADMIN_KEY: &str = "kia-admin-key-0123456789";

// ── روتر آزمایشی — هندلرهای حداقلی با همان شکل وایر ─────────────────────────

fn test_router() -> Router {
    async fn mint(Json(req): Json<MintRequest>) -> Json<Value> {
        // هندلر «سرور Rust» — قرارداد را parse کرد و تایپ‌شده جواب می‌دهد
        assert_eq!(req.amount.get(), 500);
        Json(
            json!({ "txId": "tx_1", "accountId": req.account_id, "balance": 500, "replayed": false }),
        )
    }
    async fn balance(axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
        Json(json!({ "accountId": id, "balance": 7 }))
    }
    async fn escrows(
        axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> Json<Value> {
        // status باید همان رشتهٔ معتبر وایر باشد
        assert_eq!(q.get("status").map(String::as_str), Some("FUNDED"));
        Json(json!({ "items": [] }))
    }
    async fn health_bad() -> Json<Value> {
        Json(json!({ "status": "ok" })) // ناقص — RESPONSE_MISMATCH باید بگیرد
    }
    async fn health_ok() -> Json<HealthResponse> {
        Json(HealthResponse {
            status: dropagentx_contracts::system::HealthStatus::Ok,
            service: dropagentx_contracts::system::HealthService::DropagentxV3Server,
            version: "4.4.3".into(),
            llm_configured: false,
            llm_model: None,
            db: None,
            bot: None,
            ai: None,
            uptime: None,
        })
    }
    async fn overdraft() -> (StatusCode, Json<Value>) {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                json!({ "error": { "code": "ACCOUNT_OVERDRAFT", "message": "no funds", "meta": { "needed": 90 } } }),
            ),
        )
    }
    async fn gateway_banner() -> (StatusCode, &'static str) {
        (StatusCode::BAD_GATEWAY, "<html>502 gateway</html>")
    }

    Router::new()
        .route("/wallet/mint", post(mint))
        .route("/accounts/{id}/balance", get(balance))
        .route("/escrows", get(escrows))
        .route("/health-bad", get(health_bad))
        .route("/health-ok", get(health_ok))
        .route("/overdraft", get(overdraft))
        .route("/gateway", get(gateway_banner))
}

// ── مشتری تایپ‌شدهٔ آزمایشی — نظم client.ts: همه‌چیز Result ──────────────────

#[derive(Debug)]
enum WireFailure {
    /// خطای وایر با کد عیناً از سرور
    Server {
        code: String,
        #[allow(dead_code)]
        message: String,
        status: u16,
        meta: Option<Value>,
    },
    /// بدنهٔ خطا JSON نبود (بنر گیت‌وی و…)
    MalformedErrorBody { status: u16 },
    /// پاسخ 2xx قرارداد را نقض کرد
    ResponseMismatch { status: u16 },
}

fn parse_wire<T: DeserializeOwned>(status: u16, body: &[u8]) -> Result<T, WireFailure> {
    if !(200..300).contains(&status) {
        return match serde_json::from_slice::<ErrorEnvelope>(body) {
            Ok(env) => Err(WireFailure::Server {
                code: env.error.code,
                message: env.error.message,
                status,
                meta: env.error.meta.map(|m| serde_json::to_value(m).expect("meta serializes")),
            }),
            Err(_) => Err(WireFailure::MalformedErrorBody { status }),
        };
    }
    serde_json::from_slice::<T>(body).map_err(|_| WireFailure::ResponseMismatch { status })
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    with_admin: bool,
) -> (u16, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    if with_admin {
        builder = builder.header("x-kia-key", ADMIN_KEY);
    }
    let req = builder
        .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
        .expect("request builds");
    let res = app.clone().oneshot(req).await.expect("oneshot");
    let status = res.status().as_u16();
    let bytes = res.into_body().collect().await.expect("body").to_bytes();
    (status, bytes.to_vec())
}

// ── ۸ تست پورت‌شدهٔ client.test.ts ──────────────────────────────────────────

#[tokio::test]
async fn sends_typed_json_bodies_with_caller_headers_and_parses_typed_responses() {
    let app = test_router();
    // درخواست با هدر ادمین و بدنهٔ تایپ‌شده — مثل client.mint(...)
    let (status, body) = call(
        &app,
        "POST",
        "/wallet/mint",
        Some(json!({ "accountId": "user:ali", "amount": 500, "idempotencyKey": "mint-ali-1" })),
        true,
    )
    .await;
    assert_eq!(status, 200);
    let parsed: WalletMoveResponse = serde_json::from_slice(&body).expect("typed response");
    assert_eq!(parsed.balance, 500);
    assert!(!parsed.replayed);
    // قرارداد بدنهٔ ورودی را هم به‌سختی رد می‌کند — کلید کوتاه باید رد شود
    assert!(serde_json::from_value::<MintRequest>(
        json!({ "accountId": "user:ali", "amount": 500, "idempotencyKey": "esc-1" })
    )
    .is_err());
}

#[tokio::test]
async fn maps_the_wire_error_envelope_to_a_typed_failure_verbatim_code() {
    let app = test_router();
    let (status, body) = call(&app, "GET", "/overdraft", None, false).await;
    match parse_wire::<Value>(status, &body) {
        Err(WireFailure::Server { code, status, meta, .. }) => {
            assert_eq!(code, "ACCOUNT_OVERDRAFT");
            assert_eq!(status, 422);
            assert_eq!(meta, Some(json!({ "needed": 90 })));
        }
        other => panic!("expected typed server failure, got {other:?}"),
    }
}

#[tokio::test]
async fn survives_a_non_json_error_body_gateway_banners() {
    let app = test_router();
    let (status, body) = call(&app, "GET", "/gateway", None, false).await;
    match parse_wire::<Value>(status, &body) {
        Err(WireFailure::MalformedErrorBody { status }) => assert_eq!(status, 502),
        other => panic!("expected malformed error body, got {other:?}"),
    }
}

#[tokio::test]
async fn reports_unrouted_paths_as_typed_failure_no_silent_death() {
    // معادل NETWORK در oneshot (بدون شبکه): مسیر ناشناخته باید به خطای
    // تایپ‌شده برسد، نه panic — مشتری هرگز از مرز fail نمی‌شود.
    let app = test_router();
    let (status, body) = call(&app, "GET", "/no/such/route", None, false).await;
    assert_eq!(status, 404);
    match parse_wire::<Value>(status, &body) {
        Err(WireFailure::MalformedErrorBody { status }) => assert_eq!(status, 404),
        other => panic!("expected typed failure for unrouted path, got {other:?}"),
    }
}

#[tokio::test]
async fn rejects_2xx_bodies_that_violate_the_wire_contract() {
    let app = test_router();
    let (status, body) = call(&app, "GET", "/health-bad", None, false).await;
    assert_eq!(status, 200);
    match parse_wire::<HealthResponse>(status, &body) {
        Err(WireFailure::ResponseMismatch { status }) => assert_eq!(status, 200),
        other => panic!("expected RESPONSE_MISMATCH, got {other:?}"),
    }
    // و پاسخ سالم parse می‌شود
    let (status, body) = call(&app, "GET", "/health-ok", None, false).await;
    let health: HealthResponse = serde_json::from_slice(&body).expect("typed health");
    assert_eq!(status, 200);
    assert_eq!(health.version, "4.4.3");
}

#[tokio::test]
async fn url_encodes_path_parameters_while_preserving_route_shape() {
    let app = test_router();
    // مشتری قبل از ارسال encode می‌کند — مثل fillPath در client.ts
    let encoded = urlenc("user:ali");
    assert_eq!(encoded, "user%3Aali");
    let (status, body) =
        call(&app, "GET", &format!("/accounts/{encoded}/balance"), None, false).await;
    assert_eq!(status, 200);
    let parsed: BalanceResponse = serde_json::from_slice(&body).expect("typed balance");
    assert_eq!(parsed.account_id, "user:ali");
    assert_eq!(parsed.balance.get(), 7);
}

#[tokio::test]
async fn passes_the_escrow_status_query_through() {
    let app = test_router();
    let (status, body) = call(&app, "GET", "/escrows?status=FUNDED", None, false).await;
    assert_eq!(status, 200);
    let parsed: EscrowListResponse = serde_json::from_slice(&body).expect("typed list");
    assert!(parsed.items.is_empty());
}

#[tokio::test]
async fn rejects_an_invalid_status_locally_without_touching_the_network() {
    // client.test.ts: «FLOATING» قبل از شبکه رد می‌شود — همان‌جا در parse نوع
    assert!(serde_json::from_value::<EscrowStatus>(json!("FLOATING")).is_err());
    assert!(serde_json::from_value::<EscrowStatus>(json!("FUNDED")).is_ok());
}

/// encode سادهٔ path param — پورت `encodeURIComponent` برای نویسه‌های مجاز id.
fn urlenc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

// ── ۳ تست پورت‌شدهٔ openapi.test.ts ─────────────────────────────────────────

#[test]
fn converts_fastify_style_placeholders_both_ways() {
    assert_eq!(fastify_path("/escrows/{id}/fund"), "/escrows/:id/fund");
    assert_eq!(fastify_path("/health"), "/health");
}

#[test]
fn builds_a_valid_31_doc_with_paths_params_request_body_and_security() {
    use dropagentx_contracts::common::Din;
    use utoipa::PartialSchema;

    // همان ۴ مسیر تست TS
    let routes = vec![
        ApiRouteDef {
            method: Method::Get,
            path: "/health",
            operation_id: "health",
            summary: "Liveness",
            tag: "system",
            security: Security::Public,
            params: vec![],
            body: None,
            response: <dropagentx_contracts::system::HealthStatus as PartialSchema>::schema(),
            status: 200,
        },
        ApiRouteDef {
            method: Method::Get,
            path: "/escrows/{id}",
            operation_id: "getEscrow",
            summary: "Read one escrow",
            tag: "escrows",
            security: Security::Public,
            params: vec![dropagentx_contracts::openapi::Param {
                name: "id",
                location: "path",
                required: true,
                schema: serde_json::json!({
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128,
                    "pattern": dropagentx_contracts::common::ID_PATTERN,
                }),
            }],
            body: None,
            response: <dropagentx_contracts::escrows::EscrowView as PartialSchema>::schema(),
            status: 200,
        },
        ApiRouteDef {
            method: Method::Post,
            path: "/wallet/mint",
            operation_id: "mint",
            summary: "Mint dinar (admin)",
            tag: "wallet",
            security: Security::Admin,
            params: vec![],
            body: Some(<Din as PartialSchema>::schema()),
            response: <Din as PartialSchema>::schema(),
            status: 201,
        },
        ApiRouteDef {
            method: Method::Get,
            path: "/economy",
            operation_id: "economy",
            summary: "Nation economy summary",
            tag: "economy",
            security: Security::Public,
            params: vec![],
            body: None,
            response: <EconomySummary as PartialSchema>::schema(),
            status: 200,
        },
    ];
    let doc = build_openapi_doc_inlined(
        &routes,
        &OpenApiInfo {
            title: "DropAgentX v3 API".into(),
            version: "0.2.0".into(),
            description: "test".into(),
        },
    );

    assert_eq!(doc["openapi"], json!("3.1.0"));
    let mut paths: Vec<&String> = doc["paths"].as_object().expect("paths").keys().collect();
    paths.sort();
    assert_eq!(paths, vec!["/economy", "/escrows/{id}", "/health", "/wallet/mint"]);

    let escrow = &doc["paths"]["/escrows/{id}"]["get"];
    let params = escrow["parameters"].as_array().expect("parameters");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["name"], json!("id"));
    assert_eq!(params[0]["in"], json!("path"));
    assert_eq!(params[0]["required"], json!(true));

    let mint = &doc["paths"]["/wallet/mint"]["post"];
    assert_eq!(mint["security"], json!([{ "kiaAdminKey": [] }]));
    assert!(mint["requestBody"].is_object());
    assert!(mint["responses"]["201"].is_object());

    let mut tags: Vec<String> = doc["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .map(|t| t["name"].as_str().expect("name").to_string())
        .collect();
    tags.sort();
    assert_eq!(tags, vec!["economy", "escrows", "system", "wallet"]);
    assert!(doc["components"]["securitySchemes"]["kiaAdminKey"].is_object());
}

#[test]
fn emits_json_schema_that_accepts_valid_payloads_and_rejects_invalid_ones() {
    let doc = build_openapi_doc_inlined(
        &dropagentx_contracts::routes::api_routes(),
        &dropagentx_contracts::openapi_info(),
    );
    // اسکیمای پاسخ economy باید circulatingSupply را داشته باشد (تست TS)
    let schema = &doc["paths"]["/economy"]["get"]["responses"]["200"]["content"]
        ["application/json"]["schema"];
    assert!(schema["properties"]["circulatingSupply"].is_object());
    assert_eq!(schema["properties"]["circulatingSupply"]["type"], json!("integer"));
    // و اسکیمای بدنهٔ mint مرز امن 2^53−1 را دارد
    let mint_body = &doc["paths"]["/wallet/mint"]["post"]["requestBody"]["content"]
        ["application/json"]["schema"];
    assert_eq!(mint_body["properties"]["amount"]["maximum"], json!(9_007_199_254_740_991_i64));
    assert_eq!(mint_body["properties"]["amount"]["minimum"], json!(0_i64));
    // برای معادل «rejects invalid»: نمونهٔ نامعتبر باید در تایپ Rust رد شود
    assert!(
        serde_json::from_value::<BR>(json!({ "accountId": "user:ali", "balance": -1 })).is_err()
    );
}
