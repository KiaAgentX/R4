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

**v4/10** — اسکلت ۱۳ کرِیت + دامنهٔ خالص + لایهٔ ماندگاری + **قراردادهای وایر کامل (OpenAPI 3.1)** — همهٔ گیت‌ها سبز (۱۲۵ تست).

* **فاز ۳ (v4/10):** `crates/contracts` — ۲۰+ قرارداد وایر با deriveهای serde+utoipa؛ سازندهٔ OpenAPI 3.1
  (پورت `buildOpenApiDoc` روی رجیستری ۴۶ مسیر)؛ **دیف گلدن صفر-شکست** با سند واقعی TS (۵۵ نمونهٔ وایر +
  سند کامل، تولیدشده از خود zod)؛ طبقه‌بندی خطای HTTP (۴۰۱/۴۰۴/۴۲۲/۴۲۹/۵۰۳)؛ لایهٔ امنیت قراردادی
  (X-KIA-Key زمان‌ثابت با subtle، initData تلگرام با HMAC-SHA256 رسمی، rate-limit پنجرهٔ لغزان)؛
  راند‌تریپ axum oneshot (معادل app.inject) — ۵۸ تست سبز.
* **فاز ۲ (v3/10):** `crates/db` — آداپتور `V3Store` با الگوی load → دامنهٔ خالص → persist در
  `sqlx::Transaction`؛ outbox + audit؛ اسکیمای نسخه‌دار ۱:۱ با `schema.prisma` (dual-run روی همان فایل)؛
  مهاجرت لگاسی `legacy:genesis:v1` (۴۶۶ کاربر → ۱۸۷٬۶۷۲ Đ) و ممیزی خارجی `genesis-proof`.
* **فاز ۱ (v2/10):** `crates/domain` — Din/ledger/escrow/charter/errors/events، صفر وابستگی، صفر IO.
* گام بعدی (فاز ۴): `crates/server` — Axum + داشبورد static + Dockerfile دو مرحله‌ای.

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
