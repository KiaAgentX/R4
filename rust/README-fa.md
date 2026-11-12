# 🧩 Agent-One — Rust Workspace

پیاده‌سازی ADR-009: مهاجرت کامل DropAgentX از TypeScript/Python به Rust.
این پوشه (`rust/`) داخل مونوریپوی `Agent-One` زندگی می‌کند و TS فعلی را تا پایان dual-run جایگزین نمی‌کند.

## ساختار فاز ۰ (۱۳ کرِیت + xtask)

```
rust/
├── Cargo.toml            ← members + وابستگی‌های پایه قفل‌شده + لینت‌های فورس‌شده
├── rust-toolchain.toml   ← پین دقیق 1.99.1 (clippy + rustfmt)
├── clippy.toml           ← unwrap/expect فقط در تست
├── deny.toml             ← cargo-deny (مجوز/آدیت)
├── xtask/                ← اتوماسیون: `cargo xtask check|progress`
├── docs/adr/             ← ADR-001…009 (ADR-009 کامل، HTML)
├── PROGRESS.md           ← نسخه‌بندی ۱۰/۱۰
└── crates/
    ├── domain/ 💰   contracts/ 📄   ai/ 🧠   hermes/ 🪄   agents/ 🤹
    ├── wallet/ 💳   superapp/ 🎪   nation/ 🏛️  db/ 💾    server/ 🌐
    └── bot/ 🤖      observe/ 🔭    tooling/ 🧰
```

نام رسمی پکیج‌ها با پیشوند `dropagentx-` است (مطابق ADR-009 فاز ۱: `dropagentx-domain`) و مسیرها همان نقشه ساختار نهایی‌اند.

## قواعد بازی (فورس‌شده در workspace)

- `#![forbid(unsafe_code)]` — در همه کرِیت‌ها
- `clippy::unwrap_used` و `clippy::expect_used` = **deny** (فقط در تست مجاز)
- `cargo clippy --workspace --all-targets -- -D warnings` باید سبز باشد
- مدل خطا: `thiserror` در کتابخانه‌ها، `anyhow` فقط در باینری‌ها (از فاز ۱)

## دستورها

```bash
cargo build --workspace          # ساخت
cargo test  --workspace          # تست
cargo fmt --all                  # فرمت
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask progress             # جدول پیشرفت (نسخه‌بندی ۱۰/۱۰)
cargo xtask check                # fmt + clippy + test یکجا
```

## CI

سه گردش‌کار در `rust/.github/workflows/` آماده است. نکته: GitHub Actions فقط `.github/` **ریشه مخزن** را می‌بیند؛
هنگام merge، محتوای این پوشه را با `.github/` مونوریپو ادغام کنید (فیلتر `paths: ["rust/**"]` از قبل تنظیم شده).
`cargo-deny` و `cargo-audit` شبانه اجرا می‌شوند (workflow: security).

## وابستگی‌های پایه (تصمیم ثبت‌شده — ADR-009 فاز ۰)

tokio 1.45 · axum 0.8 · teloxide 0.13 · sqlx 0.8 · serde 1.0 · utoipa 5.3 · thiserror 2.0 · tracing 0.1 · reqwest 0.12 (rustls)
خط‌مشی به‌روزرسانی: ماهانه، با `cargo update` + `cargo deny check`؛ ارتقای major فقط با ADR جدید.

## وضعیت

**v9/10** — فازهای ۰ تا ۸ کامل: اسکلت ۱۳ کرِیت + دامنهٔ خالص + ماندگاری + قراردادهای OpenAPI 3.1 + سرور Axum + بات teloxide/HITL + AI Gateway + ادغام فرانت Aurora + **dual-run → مهاجرت داده → cut-over** — بک‌اند ۳۴۲ تست سبز، clippy صفر؛ parity ۴۳/۴۳ سبز؛ ممیزی cut-over AUDIT PASS (Σ = ۱۸۷٬۶۷۲ Đ · balanced=true)؛ RSS ۱۲MB · استارت سرد ۱۰٫۷ms.

* **فاز ۸ (v9/10):** `crates/tooling` — مغز dual-run (تک‌متغیر `RUST_TRAFFIC_PCT`، سایه، سپر HITL سه‌روزه، rollback تک‌فرمان) + پراکسی HTTP با نقاط کنترل `/_dualrun/*`؛ سوئیت parity با **۴۳ سناریوی طلایی** (بایت‌به‌بایت + وضعیت DB + مقایسهٔ زندهٔ TS با `--ts-url`)؛ proptest روی invariantها؛ fuzz (libfuzzer روی nightly + fuzz-lite قطعی درون‌CI)؛ لودتست با ثبت p95/RSS (12MB) و استارت سرد (10.7ms)؛ مهاجرت + ممیزی مستقل cut-over → **AUDIT PASS**؛ RUNBOOK + CUTOVER + DEPLOY مستند شد. شواهد: `docs/phase8/`.
* **فاز ۷ (v8/10):** `front/` — Aurora + i18n + api-client تولیدی از گلدن (Step 2)؛ شل راهبردی + static-root ترکیبی (Step 3)؛ داشبورد ادمین ۸بخشی (Step 4)؛ اپ شهروندان ۳۳صفحه‌ای (Step 5)؛ PWA + Capacitor + Tauri v2 (Step 6)؛ **Step 7 — E2E پولی روی باینری واقعی (۳۷ انتظار)، تفکیک ناحیه‌ای دیکشنری (شل ۷۱٫۳KB ≤ ۸۰)، تست‌های RTL flip + گارد CSS، گیت PWA لایت‌هاوس**.
* **فاز ۶ (v7/10):** `crates/ai` — گیت‌وی AI کامل: ۶ ایجنت، ۱۶ مهارت، سه‌اسلات ADR-008، پاکت سخت، redaction ۹قاعده، سقف روزانه و audit.
* **فاز ۵ (v6/10):** `crates/bot` + webhook آکسوم — ۱۹ فرمان، i18n fa/en، صف HITL (آستانهٔ ۵۰٬۰۰۰ Đ)، replay-safe.
* **فاز ۴ (v5/10):** `crates/server` — Axum با ۴۶ مسیر رجیستری، امنیت چندلایه، داشبورد static، Dockerfile distroless.
* **فاز ۳ (v4/10):** `crates/contracts` — ۲۰+ قرارداد وایر با deriveهای serde+utoipa؛ سازندهٔ OpenAPI 3.1
  (پورت `buildOpenApiDoc` روی رجیستری ۴۶ مسیر)؛ **دیف گلدن صفر-شکست** با سند واقعی TS (۵۵ نمونهٔ وایر +
  سند کامل، تولیدشده از خود zod)؛ طبقه‌بندی خطای HTTP (۴۰۱/۴۰۴/۴۲۲/۴۲۹/۵۰۳)؛ لایهٔ امنیت قراردادی
  (X-KIA-Key زمان‌ثابت با subtle، initData تلگرام با HMAC-SHA256 رسمی، rate-limit پنجرهٔ لغزان)؛
  راند‌تریپ axum oneshot (معادل app.inject) — ۵۸ تست سبز.
* **فاز ۲ (v3/10):** `crates/db` — آداپتور `V3Store` با الگوی load → دامنهٔ خالص → persist در
  `sqlx::Transaction`؛ outbox + audit؛ اسکیمای نسخه‌دار ۱:۱ با `schema.prisma` (dual-run روی همان فایل)؛
  مهاجرت لگاسی `legacy:genesis:v1` (۴۶۶ کاربر → ۱۸۷٬۶۷۲ Đ) و ممیزی خارجی `genesis-proof`.
* **فاز ۱ (v2/10):** `crates/domain` — Din/ledger/escrow/charter/errors/events، صفر وابستگی، صفر IO.
* گام بعدی (فاز ۹): امضای نهایی — تحویل نسخهٔ ۱۰ از ۱۰ (پس از ۲ هفتهٔ پیوستهٔ ۱۰۰٪ Rust در production با صفر parity-fail).

## ابزارهای فاز ۸ (CLI)

```bash
# گیت parity — بازپخش ۴۳ سناریوی طلایی بایت‌به‌بایت + وضعیت DB (exit 0/1)
cargo run -p dropagentx-tooling --bin parity -- --verify --report parity.json
# ثبت مجدد گلدن‌ها پس از تغییر عمدی قرارداد
cargo run -p dropagentx-tooling --bin parity -- --record
# مقایسهٔ زندهٔ TS↔Rust — گیت خروج هر پلهٔ dual-run
cargo run -p dropagentx-tooling --bin parity -- --ts-url http://127.0.0.1:3000

# پراکسی dual-run — تک‌متغیر RUST_TRAFFIC_PCT؛ rollback = همان متغیر روی ۰
TS_URL=http://127.0.0.1:3000 RUST_URL=http://127.0.0.1:3002 RUST_TRAFFIC_PCT=0 ./dual-run

# لودتست — p50/p95/p99/RPS + JSON برای ثبت baseline
./target/release/loadtest --url http://127.0.0.1:3002/api/v1/health --duration 10 --json lt.json

# ممیزی نهایی cut-over — حکم go/no-go (exit 0 = AUDIT PASS)
cargo run -p dropagentx-tooling --bin audit-ledger -- docs/phase8/cutover-v3.db
```

شرح عملیاتی کامل: `docs/phase8/RUNBOOK.md` · گزارش شواهد: `docs/phase8/CUTOVER.md` · دیپلوی: `DEPLOY.md`.

## ابزارهای فاز ۲ (CLI)

```bash
# مهاجرت لگاسی — پل روز-اول از دیتابیس بازیابی‌شدهٔ Railway به v3 (replay-safe)
cargo run -p dropagentx-tooling --bin migrate-legacy -- \
  --legacy /path/to/marketplace-20260924-0400.db --target file:./db/v3.db [--dry-run]

# ممیزی خارجی — SQL خام، بدون ORM؛ Σ کیف‌ها باید ۱۸۷٬۶۷۲ Đ و Σ ردیف‌ها ۰ باشد
cargo run -p dropagentx-tooling --bin genesis-proof -- --db file:./db/v3.db
```

خروجی‌ها: `exit 0` موفق · `exit 1` نقض انتظار/خطا · `exit 2` استفادهٔ غلط.
اجرای دومِ مهاجرت خط نمی‌زند — کلید `legacy:genesis:v1` با قید UNIQUE دوبار ضرب را غیرممکن می‌کند.
