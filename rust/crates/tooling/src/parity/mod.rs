//! موتور parity فاز ۸ — «همان ورودی → همان پاسخ → همان وضعیت».
//!
//! قانون طلایی ADR-009 §۶: قرارداد ثابت، پیاده‌سازی متغیر. این ماژول سوئیت
//! parity را اجرا می‌کند:
//!
//! * سناریوهای طلایی (ورودی HTTP عین قرارداد OpenAPI) روی روتر Rust اجرا
//!   می‌شوند؛ پاسخ **بایت‌به‌بایت** (پس از نرمال‌سازی فیلدهای فرّار مثل
//!   timestamp و uptime — که بین دو پروسه هرگز برابر نمی‌شوند) با گلدن مقایسه
//!   می‌شود و وضعیت دیتابیس هم عیناً چک می‌گردد.
//! * حالت «مقایسهٔ زنده با TS» (`--ts-url`) در باینری `parity` — همان
//!   سناریوها به سرور TS هم زده می‌شود و diff بایتی گزارش می‌شود؛ این همان
//!   گیت خروجِ هر پلهٔ dual-run است (ADR-009 §۱۲).
//!
//! فایل‌های گلدن در `tests/golden/` کرِیت ذخیره‌اند؛ `--record` دوباره
//! تولیدشان می‌کند و تستِ replay در CI جلوی هر drift بایتی را می‌گیرد.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower::ServiceExt;

use dropagentx_db::V3Store;
use dropagentx_server::{build_router, AppState, ServerConfig};

/// کلید ادمین سناریوها (≥۱۶ کاراکتر — قاعدهٔ fail-closed).
pub const PARITY_KEY: &str = "parity-admin-key-0123456789";
/// توکن مخفی webhook بات در سناریوهای بات.
pub const PARITY_SECRET: &str = "parity-webhook-secret-0123456";

pub mod scenarios;

pub use scenarios::{scenarios, DbExpect, Scenario, Step};

/// یک خوانش وضعیت دیتابیس پس از سناریو — نگه‌دارندهٔ مقادیر خام SQL.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DbActual {
    /// Σ کیف کاربران (`Account.kind = 'user'`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_wallet_sum: Option<i64>,
    /// تراز خزانهٔ ملی
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treasury: Option<i64>,
    /// تعداد ردیف‌های Tx
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_count: Option<i64>,
    /// تعداد رویدادهای Outbox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbox_count: Option<i64>,
    /// وضعیت اسکرو — کلید `escrow:<id>`
    #[serde(flatten)]
    pub escrow_status: BTreeMap<String, String>,
}

/// نتیجهٔ اجرای یک سناریو — قبل از مقایسه با گلدن.
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// نام سناریو
    pub name: String,
    /// کد وضعیت HTTP
    pub status: u16,
    /// مقدار `content-type`
    pub content_type: String,
    /// بدنهٔ پاسخ پس از نرمال‌سازی (بایت‌های دقیق)
    pub body: Vec<u8>,
    /// خوانش دیتابیس
    pub db: DbActual,
}

/// گلدن یک سناریو — همان چیزی که در فایل JSON ذخیره/مقایسه می‌شود.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Golden {
    /// نام سناریو
    pub name: String,
    /// کد وضعیت مورد انتظار
    pub status: u16,
    /// `content-type` مورد انتظار
    pub content_type: String,
    /// بدنهٔ نرمال‌شده‌ی مورد انتظار (رشتهٔ UTF-8)
    pub body: String,
    /// وضعیت دیتابیس مورد انتظار
    pub db: DbActual,
}

/// نتیجهٔ مقایسهٔ یک سناریو با گلدن.
#[derive(Debug, Clone)]
pub struct ScenarioCheck {
    /// نام سناریو
    pub name: String,
    /// منطبق بودن کامل (status + content-type + بدنهٔ نرمال + دیتابیس)
    pub ok: bool,
    /// فهرست مغایرت‌های یافت‌شده — خالی یعنی سبز
    pub diffs: Vec<String>,
}

/// گزارش کامل سوئیت parity — ورودی گیت dual-run.
#[derive(Debug, Clone)]
pub struct ParityReport {
    /// نتیجهٔ هر سناریو به ترتیب رجیستری
    pub checks: Vec<ScenarioCheck>,
}

impl ParityReport {
    /// همهٔ سناریوها سبزند؟
    #[must_use]
    pub fn ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }

    /// تعداد سناریوهای سبز / کل.
    #[must_use]
    pub fn summary(&self) -> (usize, usize) {
        let green = self.checks.iter().filter(|c| c.ok).count();
        (green, self.checks.len())
    }

    /// متن گزارش قابل چاپ — برای لاگ گیت.
    #[must_use]
    pub fn render(&self) -> String {
        let (green, total) = self.summary();
        let mut out = vec![format!("parity: {green}/{total} green")];
        for c in &self.checks {
            if c.ok {
                out.push(format!("  ✅ {}", c.name));
            } else {
                out.push(format!("  ❌ {} — {}", c.name, c.diffs.join(" · ")));
            }
        }
        out.join("\n")
    }
}

/// الگوی RFC3339 — هر timestamp فرّار روی سیم به توکن ثابت تبدیل می‌شود.
/// الگو ثابتِ زمان‌کامپایل است؛ اگر روزی خراب شود تستِ
/// `normalizer_flattens_volatile_fields` در CI قرمز می‌شود (fail-loud)،
/// پس اینجا هیچ panic/expect ای لازم نیست.
fn ts_pattern() -> Option<&'static Regex> {
    static RE: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        // نمونهٔ منطبق: `2026-11-12T09:41:00.123Z` یا با آفست `+00:00`
        Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})$").ok()
    })
    .as_ref()
}

/// الگوی UUID — شناسه‌های سرورساخت (مثل `prod:<uuid>`) بین موتورها هرگز
/// برابر نمی‌شوند؛ به توکن ثابت تبدیل می‌شوند (پیشوند `prod:` حفظ می‌شود).
/// همان سیاست fail-loud الگوی timestamp — گیتِ آن تستِ replay بایتی است.
fn uuid_pattern() -> Option<&'static Regex> {
    static RE: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^([A-Za-z0-9]+:)?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
        )
        .ok()
    })
    .as_ref()
}

/// نرمال‌سازی مقدار JSON — بازگشتی: timestamp → `<TS>`، uptime → `<UPTIME>`.
#[must_use]
pub fn normalize_value(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                if k == "uptime" {
                    out.insert(k.clone(), Value::String("<UPTIME>".into()));
                } else {
                    out.insert(k.clone(), normalize_value(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize_value).collect()),
        Value::String(s) if ts_pattern().is_some_and(|re| re.is_match(s)) => {
            Value::String("<TS>".into())
        }
        Value::String(s) if uuid_pattern().is_some_and(|re| re.is_match(s)) => {
            let prefix = s.split(':').next().map_or(String::new(), |p| format!("{p}:"));
            Value::String(format!("{prefix}<UUID>"))
        }
        other => other.clone(),
    }
}

/// نرمال‌سازی بدنهٔ پاسخ — JSON را هوشمندانه، بقیه را با جایگزینی متنی.
#[must_use]
pub fn normalize_body(bytes: &[u8]) -> Vec<u8> {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(v) => serde_json::to_vec(&normalize_value(&v)).unwrap_or_default(),
        Err(_) => {
            // بدنهٔ غیر JSON (مثل صفحهٔ docs) — الگوی متنی با محدودهٔ کامل؛
            // الگو ثابت است و نمی‌تواند خراب شود، اما بی‌panic: خرابی فرضی =
            // عبور متن بدون تغییر (تست‌های replay بایتی در CI آن را می‌گیرند).
            let text = String::from_utf8_lossy(bytes).into_owned();
            match Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})")
                .ok()
            {
                Some(re) => re.replace_all(&text, "<TS>").into_owned().into_bytes(),
                None => text.into_bytes(),
            }
        }
    }
}

/// کانفیگ بوت برای یک سناریو — حداقلی و از محیط مستقل.
#[derive(Debug, Clone)]
pub struct BootCfg {
    /// webhook بات فعال؟ (توکن مخفی + ادمین بات ۴۲)
    pub bot_secret: bool,
    /// سقف نرخ (پیش‌فرض ۱۲۰ — سناریوی 429 عدد کوچک می‌دهد)
    pub rate_limit: usize,
}

impl Default for BootCfg {
    fn default() -> Self {
        Self { bot_secret: false, rate_limit: 120 }
    }
}

/// بوت روتر کامل روی دیتابیس داده‌شده — همان مسیر `main.rs` تولید.
///
/// # Errors
/// خطای باز شدن/مهاجرت دیتابیس (در تست یعنی خود سناریو شکست خورده).
pub async fn boot(cfg: &BootCfg, database_url: &str) -> anyhow::Result<(Router, V3Store)> {
    let server_cfg = ServerConfig {
        admin_key: Some(PARITY_KEY.into()),
        database_url: database_url.into(),
        static_dir: None,
        rate_limit: cfg.rate_limit,
        bot_webhook_secret: if cfg.bot_secret { Some(PARITY_SECRET.into()) } else { None },
        bot_admin_ids: if cfg.bot_secret { vec![42] } else { Vec::new() },
        ..ServerConfig::default()
    };
    let store = V3Store::connect(database_url).await?;
    store.migrate().await?;
    store.ensure_system_accounts().await?;
    let state = AppState::new(store.clone(), server_cfg).shared();
    Ok((build_router(state), store))
}

/// ارسال یک گام HTTP — پاسخ (status, content-type, بدنهٔ خام).
///
/// # Errors
/// خطای شبکه/بدنه (در oneshot عملاً نمی‌افتد؛ خرابی یعنی باگ سناریو).
pub async fn send_step(
    router: &Router,
    step: &Step,
) -> anyhow::Result<(StatusCode, String, Vec<u8>)> {
    // مسیرهای سناریو نسبت به /api/v1‌اند — همان عادت app.inject فستیفای
    let full_path = format!("/api/v1{}", step.path);
    let mut builder = Request::builder().method(step.method).uri(full_path.as_str());
    if step.admin {
        builder = builder.header("x-kia-key", PARITY_KEY);
    }
    if step.bot_secret_header {
        builder = builder.header("x-telegram-bot-api-secret-token", PARITY_SECRET);
    }
    if step.body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = step.body.as_ref().map_or_else(String::new, ToString::to_string);
    let req = builder.body(Body::from(payload))?;
    let res = router.clone().oneshot(req).await?;
    let status = res.status();
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = axum::body::to_bytes(res.into_body(), 4 << 20).await?;
    Ok((status, ctype, bytes.to_vec()))
}

/// خوانش خام وضعیت دیتابیس برای مقایسهٔ parity — SQL فقط‌خواندنی مستقل از
/// دامنه و store (همان روح `proof.rs`: ممیزی نباید باگ کدِ ممیزی‌شده را به
/// ارث ببرد).
///
/// # Errors
/// خطای SQL (یعنی دیتابیس ناسازگار است — خودِ یافتهٔ parity).
pub async fn db_snapshot(pool: &sqlx::SqlitePool, expect: &DbExpect) -> anyhow::Result<DbActual> {
    use sqlx::Row;
    let mut actual = DbActual::default();
    if expect.user_wallet_sum.is_some() {
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM(p."amount"),0) AS s
               FROM "Posting" p JOIN "Account" a ON a."id" = p."accountId"
               WHERE a."kind" = 'user'"#,
        )
        .fetch_one(pool)
        .await?;
        actual.user_wallet_sum = Some(row.get::<i64, _>("s"));
    }
    if expect.treasury.is_some() {
        let row = sqlx::query(
            r#"SELECT COALESCE(SUM(p."amount"),0) AS s
               FROM "Posting" p WHERE p."accountId" = 'treasury:nation'"#,
        )
        .fetch_one(pool)
        .await?;
        actual.treasury = Some(row.get::<i64, _>("s"));
    }
    if expect.tx_count.is_some() {
        let row = sqlx::query(r#"SELECT COUNT(*) AS n FROM "Tx""#).fetch_one(pool).await?;
        actual.tx_count = Some(row.get::<i64, _>("n"));
    }
    if expect.outbox_count.is_some() {
        let row = sqlx::query(r#"SELECT COUNT(*) AS n FROM "OutboxEvent""#).fetch_one(pool).await?;
        actual.outbox_count = Some(row.get::<i64, _>("n"));
    }
    for id in expect.escrow_status.keys() {
        let row = sqlx::query(r#"SELECT "status" FROM "Escrow" WHERE "id" = ?"#)
            .bind(id)
            .fetch_optional(pool)
            .await?;
        if let Some(r) = row {
            actual.escrow_status.insert(format!("escrow:{id}"), r.get::<String, _>("status"));
        }
    }
    Ok(actual)
}

/// اجرای کامل یک سناریو روی روتر تازه — نتیجهٔ خام (بدون مقایسه).
///
/// # Errors
/// خطای بوت/SQL — یعنی محیط parity سالم نیست.
pub async fn run_scenario(
    scenario: &Scenario,
    database_url: &str,
) -> anyhow::Result<ScenarioResult> {
    let (router, store) = boot(&scenario.boot, database_url).await?;
    for step in &scenario.setup {
        let (status, _, _) = send_step(&router, step).await?;
        // گام‌های setup باید هرگز 5xx ندهند — 5xx یعنی خود موتور شکسته است
        if status.is_server_error() {
            anyhow::bail!("setup step {} {} → {status}", step.method, step.path);
        }
    }
    let (status, ctype, raw) = send_step(&router, &scenario.request).await?;
    let body = normalize_body(&raw);
    let db = db_snapshot(store.pool_ref(), &scenario.db).await?;
    Ok(ScenarioResult {
        name: scenario.name.to_owned(),
        status: status.as_u16(),
        content_type: ctype,
        body,
        db,
    })
}

/// مقایسهٔ نتیجه با گلدن — فهرست مغایرت‌ها.
#[must_use]
pub fn compare(result: &ScenarioResult, golden: &Golden) -> ScenarioCheck {
    let mut diffs = Vec::new();
    if result.status != golden.status {
        diffs.push(format!("status {} ≠ {}", result.status, golden.status));
    }
    if result.content_type != golden.content_type {
        diffs.push(format!("content-type {:?} ≠ {:?}", result.content_type, golden.content_type));
    }
    if result.body != golden.body.as_bytes() {
        let first_diff = result
            .body
            .iter()
            .zip(golden.body.as_bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(result.body.len().min(golden.body.len()));
        diffs.push(format!(
            "body mismatch: {} bytes ≠ {} bytes — first diff at byte {first_diff}",
            result.body.len(),
            golden.body.len()
        ));
    }
    let golden_db = serde_json::to_value(&golden.db).unwrap_or(Value::Null);
    let actual_db = serde_json::to_value(&result.db).unwrap_or(Value::Null);
    if golden_db != actual_db {
        diffs.push(format!("db state {actual_db} ≠ {golden_db}"));
    }
    ScenarioCheck { name: result.name.clone(), ok: diffs.is_empty(), diffs }
}

/// پیش‌فرض پوشهٔ گلدن — `crates/tooling/tests/golden`.
#[must_use]
pub fn default_golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// مسیر فایل گلدن یک سناریو.
#[must_use]
pub fn golden_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.json"))
}

/// بارگذاری همهٔ گلدن‌ها از پوشه — غایب → `None` (گزارش «گلدن نیست»).
///
/// # Errors
/// خطای I/O یا JSON خراب.
pub fn load_goldens(dir: &Path) -> anyhow::Result<Vec<Option<Golden>>> {
    scenarios::scenarios()
        .iter()
        .map(|s| {
            let path = golden_path(dir, s.name);
            if path.exists() {
                let raw = std::fs::read_to_string(&path)?;
                Ok(Some(serde_json::from_str::<Golden>(&raw)?))
            } else {
                Ok(None)
            }
        })
        .collect()
}

/// اجرای همهٔ سناریوها و مقایسه با گلدن‌ها — هستهٔ گیت parity.
///
/// آرگومان تابع، برای هر سناریو یک URL دیتابیس تازه می‌سازد (ایزوله بودن
/// سناریوها از هم).
///
/// # Errors
/// خطای I/O گلدن‌ها یا شکست بوت (مغایرت‌ها خودشان در گزارش‌اند).
pub async fn verify_all(
    database_url_factory: impl Fn(usize) -> String,
) -> anyhow::Result<ParityReport> {
    let dir = default_golden_dir();
    let goldens = load_goldens(&dir)?;
    let mut checks = Vec::new();
    for (idx, scenario) in scenarios::scenarios().iter().enumerate() {
        let url = database_url_factory(idx);
        let result = run_scenario(scenario, &url).await?;
        let check = match &goldens[idx] {
            Some(golden) => compare(&result, golden),
            None => ScenarioCheck {
                name: scenario.name.to_owned(),
                ok: false,
                diffs: vec!["golden file missing — run `parity --record`".into()],
            },
        };
        checks.push(check);
    }
    Ok(ParityReport { checks })
}

/// ثبت همهٔ گلدن‌ها از رفتار فعلی Rust — نقطهٔ شروع parity؛ مقایسهٔ واقعی
/// TS↔Rust در تولید با `parity --ts-url` انجام می‌شود.
///
/// # Errors
/// خطای I/O یا بوت.
pub async fn record_all(database_url_factory: impl Fn(usize) -> String) -> anyhow::Result<usize> {
    let dir = default_golden_dir();
    std::fs::create_dir_all(&dir)?;
    let mut count = 0;
    for (idx, scenario) in scenarios::scenarios().iter().enumerate() {
        let url = database_url_factory(idx);
        let result = run_scenario(scenario, &url).await?;
        let golden = Golden {
            name: result.name.clone(),
            status: result.status,
            content_type: result.content_type.clone(),
            body: String::from_utf8(result.body.clone())?,
            db: result.db.clone(),
        };
        std::fs::write(golden_path(&dir, scenario.name), serde_json::to_vec_pretty(&golden)?)?;
        count += 1;
    }
    Ok(count)
}
