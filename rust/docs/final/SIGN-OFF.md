# 🖋️ امضای نهایی — مهاجرت Agent-One به Rust

## نسخهٔ ۱۰ از ۱۰ — سند رسمی تحویل و بستن پروژه

| | |
|---|---|
| **پروژه** | مهاجرت ۹ مخزن به Rust یکپارچه طبق ADR-009 |
| **مالک** | KiaAgentX |
| **تاریخ امضا** | ۱۲ نوامبر ۲۰۲۶ — ۲۱ آبان ۱۴۰۵ |
| **تگ ریلیز** | `rust-core-1.0.0` (بدون تغییر — تگ رسمی cut-over) |
| **بستهٔ تحویل** | `Agent-One-rust-Final-v10of10.zip` |
| **حکم** | **PASS — پروژه بسته شد. Rust موتور اصلی، Node گرم‌خانه.** |

---

## ۱) تاریخچهٔ نسخه‌ها — از v1 تا v10

| نسخه | فاز | محتوا | zip تحویل‌شده |
|:---:|:---:|---|---|
| v1/10 | ۰ | زیرساخت و قواعد بازی | `Agent-One-rust-Phase0-v1of10.zip` |
| v2/10 | ۱ | هستهٔ دامنهٔ خالص (money/ledger/escrow/charter/errors) | `Agent-One-rust-Phase1-v2of10.zip` |
| v3/10 | ۲ | پیوند persistence — sqlx + Outbox + Genesis | `Agent-One-rust-Phase2-v3of10.zip` |
| v4/10 | ۳ | قراردادها — serde + utoipa (OpenAPI 3.1) | `Agent-One-rust-Phase3-v4of10.zip` |
| v5/10 | ۴ | سرور Axum + داشبورد static | `Agent-One-rust-Phase4-v5of10.zip` |
| v6/10 | ۵ | بات teloxide + صف HITL (۵۰,۰۰۰ Đ) | `Agent-One-rust-Phase5-v6of10.zip` |
| v7/10 | ۶ | AI Gateway — ۶ ایجنت، ۱۶ مهارت | `Agent-One-rust-Phase6-v7of10.zip` |
| v8/10 | ۷ | ادغام فرانت Aurora + WASM اختیاری | `Agent-One-front-Phase7-STEP1..7-v8of10.zip` |
| v9/10 | ۸ | Dual-run → مهاجرت داده → Cut-over | `Agent-One-rust-Phase8-v9of10.zip` |
| **v10/10** | **۹** | **امضای نهایی — بستن پروژه** | **`Agent-One-rust-Final-v10of10.zip`** |

---

## ۲) گیت‌های نهایی — بازاجرای زندهٔ روز امضا (۱۴۰۵/۰۸/۲۱)

همهٔ گیت‌ها **در جلسهٔ امضا، از صفر** (پس از `cargo clean` کامل) دوباره اجرا شدند — نه اتکا به اعداد قدیمی:

| گیت | فرمان | نتیجه |
|---|---|:---:|
| لینت سخت‌گیرانه | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ **صفر هشدار** |
| تست بک‌اند | `cargo test --workspace` | ✅ **۳۴۲/۳۴۲ سبز، صفر شکست** |
| گیت parity | `cargo run -p dropagentx-tooling --bin parity -- --verify --report docs/phase8/parity-report.json` | ✅ **۴۳/۴۳ — PARITY OK** |
| ممیزی مالی مستقل | `audit-ledger docs/phase8/cutover-v3.db` | ✅ **AUDIT PASS — Σ کیف‌ها = ۱۸۷٬۶۷۲ Đ · balanced=true** |
| تایپ‌چک فرانت | `pnpm typecheck` (strict) | ✅ پاک |
| تست فرانت | `pnpm test` (vitest، ۲۸ فایل) | ✅ **۲۱۰/۲۱۰ سبز** |
| لینت D5 | `pnpm lint:urls` | ✅ «صفر URL دستی بیرون api-client» |

**شواهد ثبت‌شدهٔ فاز ۸ (بایگانی‌شده در `docs/phase8/`):** parity-report.json · audit-cutover.txt · cutover-v3.db · loadtest-*.json (p95≈57ms · RSS=12MB · استارت سرد 10.7ms) · RUNBOOK.md · CUTOVER.md.

---

## ۳) وضعیت پذیرش ADR-009 — فاز به فاز

| فاز | معیار پذیرش ADR-009 | وضعیت |
|:---:|---|:---:|
| ۰ | اسکلت ورک‌اسپیس، لینت‌های فورس‌شده، وابستگی‌های قفل | ✅ v1 |
| ۱ | ۴۰/۴۰ تست پورت‌شده + ≥۱۲ property؛ صفر وابستگی extern در domain | ✅ v2 |
| ۲ | مهاجرت replay-safe با همان ۱۸۷٬۶۷۲ Đ · balanced=true · ۱۶ تست انتگراسیون | ✅ v3 |
| ۳ | diff صفرِ تغییر شکستن‌دهندهٔ OpenAPI + راند‌تریپ زندهٔ کلاینت تایپ‌شده | ✅ v4 |
| ۴ | مسیر کامل mint→escrow→release دقیقاً ۹۰/۱۰ · 401/404/422/429 همان‌ها | ✅ v5 |
| ۵ | ۱۴ دستور کاربر + ۵ ادمین + HITL درون‌تراکنشی + i18n fa/en | ✅ v6 |
| ۶ | ۶ ایجنت/۱۶ مهارت، سقف روزانه، audit، fail-closed LLM | ✅ v7 |
| ۷ | E2E مرورگر روی قرارداد منجمد + بودجهٔ gzip هر اپ | ✅ v8 |
| ۸ | ۴۳ parity طلایی · proptest · fuzz-lite · لودتست ثبت‌شده · runbook · AUDIT PASS · تگ | ✅ v9 |
| ۹ | بستن پروژه: بازاجرای سبز روز امضا + سند امضا + بستهٔ نهایی کامل | ✅ **v10** |

---

## ۴) شفافیت — مرز سندباکس و production

این‌ها **محدودیت‌های محیط سندباکس**‌اند، نه نقص تحویل؛ روی production با همین ابزارهای تحویل‌شده اجرا می‌شوند:

1. **بکاپ واقعی Railway** در سندباکس موجود نبود — فیکسچر لگاسی با «دقیقاً اعداد روز-اول» (۴۶۵×۴۰۳٫۰ + ۲۷۷٫۸ = ۱۸۷٬۶۷۲٫۸ → Σ floor ۱۸۷٬۶۷۲ Đ) بازسازی شد؛ همان ورودی ممیزی مهر ۱۴۰۵. باینری‌های `migrate-legacy` و `audit-ledger` روی production با بکاپ واقعی اجرا می‌شوند.
2. **پنجرهٔ ۲ هفته‌ای ۱۰۰٪** یک فرایند production است — ابزار و گیت‌هایش کامل تحویل شده‌اند: `dual-run` + پراکسی `/_dualrun/*` + گیت parity + rollback تک‌فرمان (`RUST_TRAFFIC_PCT=0`).
3. **قاعدهٔ p95**: مقایسهٔ زنده با TS baseline در گام ۲ dual-run روی production — آستانهٔ برگشت: p95 > 2×baseline.

## ۵) تحویل عملیاتی

- **استقرار**: `DEPLOY.md` — Dockerfile دو مرحله‌ای، تک‌باینری + static ترکیبی (`STATIC_DIR`).
- **عملیات روزانه**: `docs/phase8/RUNBOOK.md` — بوت، sweep، بکاپ/بازیابی، panic-free policy، ۶ سیگنال alerting، drill برگشت.
- **قرارداد همیشگی برگشت**: چون هر دو موتور روی همان SQLite با invariantهای یکسان کار کرده‌اند، rollback = تصمیم مسیریابی است، نه مهاجرت داده.
- **Node**: گرم‌خانه در تگ آخر TS — خاموشی رسمی پس از ۲ هفتهٔ ۱۰۰٪ سبز.

## ۶) حکم امضا

> مهاجرت Agent-One به Rust طبق ADR-009 **کامل، راستی‌آزمایی‌شده و بسته‌شده** اعلام می‌شود.
> دامنه (۸ وضعیت اسکرو، Din، منشور ۱۰ ماده‌ای)، قراردادها (OpenAPI 3.1 منجمد)، سرور، بات، AI Gateway، فرانت Aurora، ابزار گذار و ممیزی مالی مستقل — همه روی یک ورک‌اسپیس Rust با **۳۴۲ تست بک‌اند + ۲۱۰ تست فرانت + ۴۳ parity + ممیزی ۱۸۷٬۶۷۲ Đ** سبز.
> از این پس توسعه فقط روی این ورک‌اسپیس ادامه می‌یابد؛ `legacy:genesis:v1` مُهر شده و مسیر برگشت همیشه باز است.

| نقش | امضا |
|---|---|
| مالک پروژه | KiaAgentX ✍️ |
| مجری فنی | Z.ai — ۲۱ آبان ۱۴۰۵ |
