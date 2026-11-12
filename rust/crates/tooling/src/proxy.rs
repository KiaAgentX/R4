//! پراکسی HTTP مسیریابی — بدنهٔ dual-run فاز ۸ (ADR-009 §۱۲).
//!
//! تک‌متغیرِ سوییچ `RUST_TRAFFIC_PCT` است؛ برگشت اضطراری همان متغیر روی ۰.
//! تصمیم هر درخواست با [`crate::dual_run::DualRunConfig::decide`] گرفته می‌شود
//! و به‌صورت خط JSON لاگ می‌شود. در حالت سایه، پاسخ‌دهنده TS است و اجرای Rust
//! فقط ثبت/مقایسه می‌شود (`parity: match|diff` — بدنه‌ها پس از نرمال‌سازی
//! parity بایت‌به‌بایت سنجیده می‌شوند).
//!
//! نقاط کنترل (خارج `/api/v1`):
//! * `GET /_dualrun/status` — پیکربندی جاری + برنامهٔ پله‌ها + شمارنده‌ها
//! * `POST /_dualrun/pct` — `{"pct": 10}` — ارتقای پله (تک‌متغیر)
//! * `POST /_dualrun/rollback` — برگشت فوری: `pct=0` + سایهٔ روشن
//!
//! تست انتگراسیون با دو آپ‌استریم شبیه‌سازی‌شده روی پورت موقت، کل چرخهٔ
//! shadow → pct → rollback را می‌پوشاند.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use serde_json::{json, Value};

use crate::dual_run::{Decision, DualRunConfig, RequestMeta};
use crate::parity::normalize_body;

/// نام‌های هدر که بین مشتری و آپ‌استریم منتقل نمی‌شوند (hop-by-hop).
const STRIP_HEADERS: [&str; 4] = ["host", "connection", "content-length", "transfer-encoding"];

/// شمارنده‌های dual-run — برای status و تست.
#[derive(Debug, Default)]
pub struct ProxyStats {
    /// تعداد ارجاع به Rust
    pub rust: AtomicU64,
    /// تعداد ارجاع به TS
    pub ts: AtomicU64,
    /// تعداد اجرای سایه
    pub shadow: AtomicU64,
    /// مغایرت parity در سایه
    pub parity_diff: AtomicU64,
    /// تطابق parity در سایه
    pub parity_match: AtomicU64,
}

impl ProxyStats {
    /// خوانش اسنپ‌شات شمارنده‌ها.
    #[must_use]
    pub fn snapshot(&self) -> Value {
        json!({
            "rust": self.rust.load(Ordering::Relaxed),
            "ts": self.ts.load(Ordering::Relaxed),
            "shadow": self.shadow.load(Ordering::Relaxed),
            "parityDiff": self.parity_diff.load(Ordering::Relaxed),
            "parityMatch": self.parity_match.load(Ordering::Relaxed),
        })
    }
}

/// وضعیت مشترک پراکسی — پیکربندی تغییرپذیر زنده + آپ‌استریم‌ها + شمارنده‌ها.
pub struct ProxyState {
    /// پیکربندی جاری (تغییر از نقاط کنترل)
    pub config: RwLock<DualRunConfig>,
    /// پایهٔ URL آپ‌استریم TS (مثل `http://127.0.0.1:3000`)
    pub ts_base: String,
    /// پایهٔ URL آپ‌استریم Rust
    pub rust_base: String,
    /// شمارنده‌ها
    pub stats: ProxyStats,
    /// کلاینت HTTP بازمصرف — timeout صریح (قاعدهٔ ADR-006)
    pub client: reqwest::Client,
}

impl ProxyState {
    /// ساخت وضعیت پراکسی.
    ///
    /// # Panics
    /// هرگز — ساخت کلاینت ناموفق → کلاینت پیش‌فرض.
    #[must_use]
    pub fn new(config: DualRunConfig, ts_base: String, rust_base: String) -> Arc<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Arc::new(Self {
            config: RwLock::new(config),
            ts_base,
            rust_base,
            stats: ProxyStats::default(),
            client,
        })
    }
}

/// ساخت روتر پراکسی — همهٔ مسیرها → تصمیم → آپ‌استریم؛ نقاط کنترل `/_dualrun`.
pub fn proxy_router(state: Arc<ProxyState>) -> axum::Router {
    let control: axum::Router<Arc<ProxyState>> = axum::Router::new()
        .route("/_dualrun/status", get(status))
        .route("/_dualrun/pct", post(set_pct))
        .route("/_dualrun/rollback", post(rollback));
    control.fallback(forward_all).with_state(state)
}

async fn status(State(st): State<Arc<ProxyState>>) -> Response {
    let cfg = st.config.read().map_or_else(|_| DualRunConfig::default(), |c| c.clone());
    let body = json!({
        "config": cfg,
        "plan": crate::dual_run::rollout_plan(),
        "stats": st.stats.snapshot(),
    });
    Json(body).into_response()
}

async fn set_pct(State(st): State<Arc<ProxyState>>, body: String) -> Response {
    let parsed: Result<Value, _> = serde_json::from_str(&body);
    let pct = parsed.ok().and_then(|v| v.get("pct").and_then(Value::as_u64));
    let Some(pct) = pct else {
        return err_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_PCT",
            "body must be {\"pct\": 0..=100}",
        );
    };
    if pct > 100 {
        return err_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_PCT",
            "pct must be 0..=100",
        );
    }
    let verdict = match st.config.write() {
        Ok(mut cfg) => {
            cfg.rust_traffic_pct = pct as u8;
            cfg.shadow = false; // ترافیک واقعی یعنی خروج از سایه
            cfg.validate().err()
        }
        Err(_) => Some("config lock poisoned".into()),
    };
    if let Some(msg) = verdict {
        return err_response(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_CONFIG", &msg);
    }
    log_line(&json!({"event": "pct", "value": pct}));
    Json(json!({ "ok": true, "pct": pct })).into_response()
}

async fn rollback(State(st): State<Arc<ProxyState>>) -> Response {
    if let Ok(mut cfg) = st.config.write() {
        cfg.rust_traffic_pct = 0;
        // سایه هم خاموش — به موتور خراب هیچ ترافیکی (حتی سایه) نمی‌فرستیم
        cfg.shadow = false;
    }
    log_line(&json!({"event": "rollback", "pct": 0, "note": "همهٔ ترافیک فوراً به TS"}));
    Json(json!({ "ok": true, "rollback": true, "pct": 0 })).into_response()
}

/// پاسخ خطای پاکت‌دار — همان شکل `ErrorEnvelope`.
fn err_response(status: StatusCode, code: &str, message: &str) -> Response {
    let envelope = json!({ "error": { "code": code, "message": message } });
    (status, envelope.to_string()).into_response()
}

/// لایهٔ اصلی — هر درخواست ناشناخته (یعنی همهٔ `/api/v1/...` و بقیه).
async fn forward_all(State(st): State<Arc<ProxyState>>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let method = parts.method.as_str().to_owned();
    let path_and_query = parts
        .uri
        .path_and_query()
        .map_or_else(|| parts.uri.path().to_owned(), |pq| pq.as_str().to_owned());
    let bytes = axum::body::to_bytes(body, 8 << 20).await.unwrap_or_default().to_vec();

    let cfg = st.config.read().map_or_else(|_| DualRunConfig::default(), |c| c.clone());
    let meta = RequestMeta { method: &method, path: &path_and_query, body: &bytes };
    let decision = cfg.decide(&meta);

    match decision {
        Decision::ToRust => {
            st.stats.rust.fetch_add(1, Ordering::Relaxed);
            log_line(&json!({"decision": "rust", "method": method, "path": path_and_query}));
            respond(
                forward_raw(&st, &st.rust_base, &parts.headers, &method, &path_and_query, &bytes)
                    .await,
            )
        }
        Decision::ToTs => {
            st.stats.ts.fetch_add(1, Ordering::Relaxed);
            log_line(&json!({"decision": "ts", "method": method, "path": path_and_query}));
            respond(
                forward_raw(&st, &st.ts_base, &parts.headers, &method, &path_and_query, &bytes)
                    .await,
            )
        }
        Decision::Shadow => {
            st.stats.shadow.fetch_add(1, Ordering::Relaxed);
            // پاسخ‌دهندهٔ واقعی: TS؛ Rust سایه‌وار — نتیجه فقط لاگ/شمارنده
            let ts_raw =
                forward_raw(&st, &st.ts_base, &parts.headers, &method, &path_and_query, &bytes)
                    .await;
            let rust_raw =
                forward_raw(&st, &st.rust_base, &parts.headers, &method, &path_and_query, &bytes)
                    .await;
            // گیت parity سایه: status + بدنهٔ نرمال‌شده بایت‌به‌بایت
            let same_status = ts_raw.0 == rust_raw.0;
            let same_body = normalize_body(&ts_raw.2) == normalize_body(&rust_raw.2);
            if same_status && same_body {
                st.stats.parity_match.fetch_add(1, Ordering::Relaxed);
            } else {
                st.stats.parity_diff.fetch_add(1, Ordering::Relaxed);
            }
            log_line(&json!({
                "decision": "shadow", "method": method, "path": path_and_query,
                "parity": if same_status && same_body { "match" } else { "diff" },
                "tsStatus": ts_raw.0,
                "rustStatus": rust_raw.0,
            }));
            respond(ts_raw)
        }
    }
}

/// تبدیل پاسخ خام آپ‌استریم به پاسخ آکسوم.
fn respond(raw: (u16, HeaderMap, Vec<u8>)) -> Response {
    let (status, headers, body) = raw;
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for (name, value) in &headers {
        if let (Ok(n), Ok(v)) = (
            axum::http::HeaderName::from_bytes(name.as_str().as_bytes()),
            axum::http::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            builder = builder.header(n, v);
        }
    }
    builder.body(Body::from(body)).unwrap_or_else(|_| {
        err_response(StatusCode::BAD_GATEWAY, "PROXY_BODY_ERROR", "failed to rebuild body")
    })
}

/// ارسال درخواست به آپ‌استریم — `(status, هدرها, بدنهٔ خام)`.
///
/// آپ‌استریم مرده → ۵۰۲ با پاکت همان شکل (fail-closed).
async fn forward_raw(
    st: &ProxyState,
    base: &str,
    headers: &HeaderMap,
    method: &str,
    path_and_query: &str,
    body: &[u8],
) -> (u16, HeaderMap, Vec<u8>) {
    let url = format!("{base}{path_and_query}");
    let method_reqwest =
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);
    let mut req = st.client.request(method_reqwest, &url);
    for (name, value) in headers {
        if !STRIP_HEADERS.contains(&name.as_str()) {
            if let (Ok(n), Ok(v)) = (
                HeaderName::from_bytes(name.as_str().as_bytes()),
                reqwest::header::HeaderValue::from_bytes(value.as_bytes()),
            ) {
                req = req.header(n, v);
            }
        }
    }
    if !body.is_empty() {
        req = req.body(body.to_vec());
    }
    match req.send().await {
        Ok(up) => {
            let status = up.status().as_u16();
            let mut out_headers = HeaderMap::new();
            for (name, value) in up.headers() {
                if STRIP_HEADERS.contains(&name.as_str().to_lowercase().as_str()) {
                    continue;
                }
                if let (Ok(n), Ok(v)) = (
                    HeaderName::from_bytes(name.as_str().as_bytes()),
                    axum::http::HeaderValue::from_bytes(value.as_bytes()),
                ) {
                    out_headers.insert(n, v);
                }
            }
            let bytes = up.bytes().await.unwrap_or_default().to_vec();
            (status, out_headers, bytes)
        }
        Err(e) => {
            let envelope = json!({
                "error": { "code": "UPSTREAM_DOWN", "message": format!("upstream unreachable: {e}") }
            });
            (502, HeaderMap::new(), envelope.to_string().into_bytes())
        }
    }
}

/// خط JSON لاگ — استاندارد به stderr (جدا از پاسخ‌ها).
fn log_line(msg: &Value) {
    eprintln!("{msg}");
}
