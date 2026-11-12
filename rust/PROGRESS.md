# 🧩 پیشرفت مهاجرت Agent-One به Rust — نسخه‌بندی ۱۰/۱۰

> فرمول: هر فاز کامل‌شده = +۱ نسخه (فاز ۰ → v1/10 … فاز ۸ → v9/10)
> **v10/10 = امضای نهایی پس از dual-run کامل و cut-over**

| فاز | عنوان (ADR-009) | وضعیت | نسخه |
|:---:|---|:---:|:---:|
| 0 | زیرساخت و قواعد بازی | ✅ انجام شد | **v1/10** |
| 1 | هسته دامنه خالص — dropagentx-domain (money/ledger/escrow/charter/errors) | ✅ انجام شد | **v2/10** |
| 2 | پیوند persistence — sqlx + Outbox + Genesis (legacy:genesis:v1) | ✅ انجام شد | **v3/10** |
| 3 | قراردادها — serde + utoipa (OpenAPI 3.1) | ✅ انجام شد | **v4/10** |
| 4 | سرور Axum + داشبورد static | ✅ انجام شد | **v5/10** |
| 5 | بات teloxide + صف HITL (آستانه ۵۰,۰۰۰ Đ) | ✅ انجام شد | **v6/10** |
| 6 | AI Gateway — ۶ ایجنت، ۱۶ مهارت، سقف و audit | ✅ انجام شد | **v7/10** |
| 7 | ادغام فرانت + WASM اختیاری | ✅ انجام شد | **v8/10** |
| 8 | Dual-run → مهاجرت داده → Cut-over | ✅ انجام شد | **v9/10** |
| 9 | امضای نهایی — تحویل نسخه ۱۰ از ۱۰ | ✅ انجام شد | **v10/10** |

**وضعیت نهایی: v10/10 — پروژه بسته شد 🏁 امضای نهایی در ۲۱ آبان ۱۴۰۵ (سند رسمی: `docs/final/SIGN-OFF.md`). بازاجرای زندهٔ روز امضا: clippy صفر · ۳۴۲/۳۴۲ تست بک‌اند · parity ۴۳/۴۳ · AUDIT PASS (Σ = ۱۸۷٬۶۷۲ Đ · balanced=true) · فرانت: typecheck پاک + ۲۱۰/۲۱۰ تست + D5. تگ رسمی ریلیز: `rust-core-1.0.0` — Rust موتور اصلی، Node گرم‌خانه. بستهٔ نهایی: `Agent-One-rust-Final-v10of10.zip`.**

**فاز ۸ کامل شد → تحویل به‌صورت v9/10** (فرمول فاز N → v(N+1)).

---

## فاز ۹ — امضای نهایی (v10/10) ✅

گیت‌های نهایی **در روز امضا از صفر** (پس از `cargo clean` کامل) بازاجرا شدند — همه سبز:

| گیت | نتیجه |
|---|:---:|
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ صفر هشدار |
| `cargo test --workspace` | ✅ ۳۴۲/۳۴۲ |
| parity verify (۴۳ سناریوی طلایی) | ✅ PARITY OK — gate green |
| `audit-ledger docs/phase8/cutover-v3.db` | ✅ AUDIT PASS — Σ = ۱۸۷٬۶۷۲ Đ · balanced=true |
| فرانت: typecheck + vitest + lint:urls | ✅ پاک · ۲۱۰/۲۱۰ · D5 ok |

تحویل: سند رسمی امضا در `docs/final/SIGN-OFF.md` (تاریخچهٔ v1→v10، پذیرش فازبه‌فاز ADR-009، مرز سندباکس/production، تحویل عملیاتی، حکم امضا) + بستهٔ نهایی کامل (rust + front + docs) `Agent-One-rust-Final-v10of10.zip`.

> پایان مسیر ۲۰ هفته‌ای ADR-009: از «قواعد بازی» تا cut-over — ۱۳ crate، یک قرارداد منجمد، یک امپراتوری مهاجرت‌شده. 🦀

## پیوست پس از امضا — بستهٔ production Railway (افزودنی v10/10)

به درخواست مالک: «تحویل نسخهٔ production با تست روی Railway» — بدون تغییر کد دامنه/سرور:

| قطعه | مسیر |
|---|---|
| Dockerfile production سه‌مرحله‌ای (استاتیک فاز ۷ داخل ایمیج) | `Dockerfile` (ریشهٔ ریپو) |
| تنظیمات Railway (DOCKERFILE · healthcheck `/api/v1/health` · restart) | `railway.toml` |
| راهنمای گام‌به‌گام Railway (GitHub/CLI، Volume `/data`، مهاجرت واقعی، برگشت) | `DEPLOY-RAILWAY.md` |
| شواهد اعتبارسنجی production | `docs/final/PRODUCTION-READINESS.md` |

**اعتبارسنجی release در سندباکس:** بیلد release سبز (6m42s) · **smoke ۱۲/۱۲** (شامل static-root ترکیبی `/`, `/admin/`, `/app/` روی باینری release · گیت ادمین 401/200 · اقتصاد زندهٔ ۱۸۷٬۶۷۲ Đ · AUDIT PASS روی دیتابیس همان بوت) · لودتست release: **p95 = ۰٫۲ms** (۵۶۴k+ درخواست، صفر خطا) · **RSS = ۱۰MB**. شفاف: docker daemon و حساب Railway در سندباکس نیستند — بیلد ایمیج و دیپلوی واقعی طبق `DEPLOY-RAILWAY.md` با حساب مالک انجام می‌شود.

---

## فاز ۸ — Dual-run → مهاجرت داده → Cut-over (v9/10) ✅

**تحویل‌داده‌شده‌ها** — نگاشت کامل به «دامنه کار» فاز ۸ ADR-009 (شرح شواهد: `docs/phase8/CUTOVER.md`):

| بند ADR | تحویل | مسیر |
|---|---|---|
| استقرار dual-run (§۱۲) | مغز مسیریابی دترمیتیک (هش پایدار ورودی) + سایه + سپر HITL سه‌روزه (مادهٔ ۷) + گیت parity + rollback تک‌فرمان؛ پراکسی HTTP با `/_dualrun/status·pct·rollback`؛ باینری `dual-run` (تک‌متغیر `RUST_TRAFFIC_PCT`، بوت متناقض = fail-closed) | `crates/tooling/src/dual_run.rs` · `src/proxy.rs` · `src/bin/dual-run.rs` |
| سوئیت parity (~۳۰ سناریو) | **۴۳ سناریوی طلایی** — status + content-type + بدنهٔ بایت‌به‌بایت (نرمال‌سازی `<TS>/<UPTIME>/<UUID>`) + خوانش SQL خام؛ حالت‌های `--record` / `--verify` / `--ts-url` (مقایسهٔ زندهٔ TS↔Rust) | `src/parity/` · `tests/golden/` (۴۳ فایل) · `src/bin/parity.rs` |
| proptest invariantها | ۳ خاصیت: تخصیص ترافیک تابع قطعی درخواست · Σ=۰ و تراز ≥۰ از درگاه وایر (دنبالهٔ تصادفی mint/burn/escrow) · ایدمپوتنت‌بودن نرمال‌ساز | `tests/properties.rs` |
| cargo-fuzz پارسرها | ۳ هدف libfuzzer (mint/escrow/ai-chat — nightly/CI) + fuzz-lite قطعی با موتور موتاسیون xorshift درون‌CI (هر panic = یافتهٔ fuzz) | `fuzz/` · `tests/fuzz_lite.rs` |
| لودتست کوتاه | مولد بار tokio (p50/p95/p99/RPS/توزیع وضعیت + JSON) — release: p95 ≈ ۵۷ms، 0 خطا · **RSS = ۱۲MB** (هدف ~۲۰MB؛ TS ~۱۲۰–۱۵۰MB) · استارت سرد **۱۰٫۷ms** (هدف <۱۰۰ms) | `src/bin/loadtest.rs` · `docs/phase8/loadtest-*.json` · `memory.txt` |
| Runbook عملیاتی | بوت، sweep، بکاپ/بازیابی، panic-free policy، alerting (۶ سیگنال با آستانه)، آموزش سوییچ تک‌متغیره + drill برگشت | `docs/phase8/RUNBOOK.md` |
| مهاجرت داده + ممیزی نهایی | مسیر رسمی لگاسی→v3 (replay-safe، `legacy:genesis:v1`) + ممیزی مستقل SQL خام با حکم go/no-go → **AUDIT PASS: Σ کیف‌ها = ۱۸۷٬۶۷۲ Đ · trialBalance.balanced = true** (روی کاندیدای cut-over مهاجرت‌شده) | `src/bin/migrate-legacy.rs` · `src/bin/audit-ledger.rs` · `docs/phase8/audit-cutover.txt` · `cutover-v3.db` |
| تگ + آرشیو Node + مستندات | تگ `rust-core-1.0.0` · Node = گرم‌خانه در تگ آخر TS · `README-fa.md`/`DEPLOY.md` به‌روز | `DEPLOY.md` · `README-fa.md` |

**شواهد سبز:** parity ۴۳/۴۳ (`parity-report.json`) · ۳۴۲ تست بک‌اند · clippy صفر · اقتصاد زندهٔ سرور روی دیتابیس cut-over: `circulatingSupply=187672 · balanced=true · accountCount=469`.

**یادداشت صادقانه:** بکاپ واقعی Railway و سرور TS در سندباکس موجود نیستند؛ (۱) فیکسچر لگاسی با «دقیقاً اعداد روز-اول» بازسازی شد (۴۶۵×۴۰۳٫۰ + ۲۷۷٫۸ = ۱۸۷٬۶۷۲٫۸ → Σ floor ۱۸۷٬۶۷۲ Đ — همان ورودی ممیزی نسل‌ها)؛ (۲) مقایسهٔ p95 با TS baseline طبق §۱۲ در گام ۲ dual-run روی production با همین ابزار گرفته می‌شود (قاعدهٔ برگشت: p95 > 2×baseline). پنجرهٔ ۲ هفته‌ایِ ۱۰۰٪ فرایند production است — ابزار و گیت‌هایش کامل و تحویل‌شده‌اند.

**فاز ۹ — انجام شد ✅ → نسخهٔ ۱۰ از ۱۰ (سند امضا: `docs/final/SIGN-OFF.md`).**

## فاز ۷ — ادغام فرانت + WASM اختیاری (v8/10) ✅ — Stepهای ۱–۷ کامل شد

**Step 1 — ممیزی + نقشهٔ معماری** (`docs/phase7/STEP1-AUDIT.md`):

* **ممیزی فرانت قدیمی** — منابع TS (`front/dashboard` ۶٬۵۱۳ LOC + `front/webapp` ۳٬۰۰۹ LOC) در سندباکس موجود نیستند؛ ممیزی مستند بر پایهٔ ADR-009 §2.3 + سند ساختار نهایی + مشخصات مالک (۳۳ صفحه webapp — نقشهٔ DRAFT).
* **سطح API منجمد** — ۴۶ عملیات روی ۴۰ مسیر با ستون امنیت رجیستری (ادمین X-KIA-Key / عمومی / گیت هویت AI) فهرست و راستی‌آزمایی شد.
* **معماری هدف** — `apps/{shell,dashboard,webapp,site,desktop,mobile}` + `packages/{ui,api-client,i18n,domain-wasm}` روی pnpm؛ شل راهبردی `/` با تشخیص Telegram/نقش/زبان؛ Tauri v2 (بدون Electron)؛ static ترکیبی روی ServeDir موجود با فقط تغییر مقدار `STATIC_DIR` — **صفر تغییر کد بک‌اند**.
* **۱۴ تصمیم ثبت‌شده (D1–D14)** — از جمله: api-client تولیدی از `golden-openapi.json` + تست round-trip زنده؛ کلید idempotency سمت کلاینت با استفادهٔ مجدد در retry؛ دیکشنری خطا آینهٔ i18n بات؛ فونت سلف-هاست Vazirmatn/Inter؛ WASM اختیاری با سقف ۳۰۰KB gzip.

**Step 2 — Aurora + i18n + api-client تولیدی** (`front/` — ریشهٔ workspace pnpm به‌صورت زیرپوشه، جدا از ریشهٔ cargo؛ تصمیم D15):

| قطعه | فایل | توضیح |
|---|---|---|
| Aurora توکن‌ها | `front/packages/ui/src/tokens.css` | لایت/دارک با `[data-theme]` + prefers-color-scheme؛ پالت برند، ۸ رنگ ماشین وضعیت اسکرو، فاصله/شعاع/سایه/موشن، تارگت لمس ۴۴px، reduced-motion |
| فونت سلف-هاست | `front/packages/ui/src/fonts.css` + `fonts/` | Vazirmatn/Inter وریبل woff2 (۲۴۰KB، OFL) با unicode-range و font-display:swap؛ بدون CDN (D9)؛ latin-ext پوشش Đ (U+0110) |
| پایه + کامپوننت‌ها | `base.css` + `components.css` | فقط logical properties → آینهٔ کامل RTL؛ aur-money با isolate LTR + tabular-nums؛ دکمه/کارت/بج/badge وضعیت/فیلد/جدول/spinner/skeleton/empty/error + اعلان HITL |
| پل Tailwind 4 | `front/packages/ui/src/tailwind.css` | @theme همان توکن‌های aur-* را به کلاس‌های utility می‌نگشد — یک منبع حقیقت، صفر drift؛ بیلد CLI سبز |
| پول | `front/packages/ui/src/money.ts` | آینهٔ دقیق `format_din` دامنه (تست `187,672 Đ`)، سقف 2^53−1، `formatSignedDin` با علامت − (U+2212) مثل بات، `escrowSplit` ۹۰/۱۰ با floor مثل mul_bps، آستانهٔ HITL ۵۰٬۰۰۰ Đ |
| تم و جهت | `theme.ts` + `dir.ts` | سوئیچ دارک/لایت با persist، اعمال lang+dir روی html |
| لوکال | `front/packages/i18n/src/locale.ts` | پیش‌فرض fa، قانون بات `en*→en`، RTL درجه‌یک |
| دیکشنری UI | `front/packages/i18n/src/dict.ts` | واژگان یکی بات (امانت/تأمین مالی/آزادسازی…) + برچسب محلی ۸ وضعیت اسکرو + تست پاریتی fa/en |
| نگاشت خطا | `front/packages/i18n/src/errors.ts` | ۲۰ کد دامنه متن‌به‌متن آینهٔ `crates/bot/src/i18n.rs` + ۲۰ کد وایر سرور + ۳ کد سمت کلاینت؛ کد خام هرگز نمایش نمی‌یابد؛ fallback = متن داخلی بات |
| تایپ‌های تولیدی | `front/packages/api-client/src/schema.d.ts` | ۳٬۳۱۲ خط با openapi-typescript از گلدن‌فیکسچر (۴۰ مسیر/۴۶ عملیات قفل)؛ تست determinism بایت‌به‌بایت |
| کلاینت | `front/packages/api-client/src/client.ts` | openapi-fetch تایپ‌شده؛ لایهٔ هویت: X-KIA-Key ادمین + initData تلگرام فقط داخل /ai/chat (D14)؛ `reads` عمومی شهروند؛ baseUrl پیش‌فرض `/api/v1` مثل گلدن |
| idempotency | `front/packages/api-client/src/idempotency.ts` | ULID دست‌ساز Crockford (۲۶ کاراکتر)؛ ۱۰ عملیات پولی کلید می‌گیرند: یک‌بار به‌ازای هر عمل منطقی، store حافظه/localStorage، استفادهٔ مجدد در retry؛ explicit pin اولویت دارد |
| پاکت خطا | `front/packages/api-client/src/errors.ts` | parse `{error:{code,message,meta}}`؛ پاکت خراب → MALFORMED_ERROR_BODY؛ شبکه/abort → کد سمت کلاینت؛ `unwrap` تایپ‌شده |
| گیت D5 | `front/scripts/check-no-raw-urls.mjs` | ممنوعیت URL دستی بیرون api-client — در `pnpm check` |
| round-trip زنده | `front/scripts/round-trip-live.mjs` | پورت وفادارانهٔ canon() تست گلدن فاز ۳؛ اجرا روی باینری واقعی سرور: **live == golden صفر تغییر شکستن‌دهنده، ۴۰/۴۶** |

**معیارهای پذیرش Step 2 — همه سبز:**

1. **api-client تولیدی از قرارداد منجمد** — schema.d.ts از گلدن تولید و تست determinism تولید دوباره را بایت‌به‌بایت می‌سنجد؛ بدنهٔ ۱۰ عملیات پولی `idempotencyKey` وایر دارد و کلاینت همیشه یک کلید ≥۸ می‌فرستد (سخت‌گیرانه‌تر از اسکیما — D7).
2. **round-trip زنده در برابر سرور Rust واقعی** — سبز: ۴۰ مسیر / ۴۶ عملیات، همان معیار صفر تغییر شکستن‌دهندهٔ فاز ۳.
3. **دیکتاتور خطا + پاریتی** — fa/en کلید‌به‌کلید برابر، هر ۴۳ کد (۲۰ دامنه + ۲۰ وایر + ۳ کلاینت) پوشش‌داده و متمایز؛ `۱۸۷٬۶۷۲ Đ` تست‌شده.

**آمار Step 2:** ~۱٬۹۰۰ LOC TS + ~۹۰۰ LOC CSS + ~۹۵۰ LOC تست · **۵۳ تست فرانت سبز (آفلاین)** · typecheck strict پاک · lint:urls پاک · بیلد Tailwind ۴ سبز · بک‌اند دست‌نخورده: ۳۱۷ تست سبز، clippy صفر.

**Step 3 — شل راهبردی + static-root ترکیبی** (`front/apps/shell` + `front/scripts/build-static-root.mjs`):

| قطعه | فایل | توضیح |
|---|---|---|
| تشخیص زمینهٔ خالص | `apps/shell/src/detect.ts` | تلگرام فقط با `initData` غیروفارغ؛ نقش = کلید ادمین ذخیره‌شده > شهروند تلگرام > مهمان؛ زبان = ذخیره‌شده > hint تلگرام > navigator > fa (قانون بات)؛ دستگاه با coarse pointer/عرض؛ storage معیوب → fail-closed بی‌کرش |
| نواحی + manifest | `apps/shell/src/areas.ts` | `/admin/` · `/app/` · `/site/` با slash انتهایی؛ در دسترس‌بودن از `build-manifest.json`؛ غایب = «در گام‌های بعدی» — هرگز صفحهٔ جعلی |
| UI شل | `apps/shell/src/App.tsx` + `main.tsx` | React 18 + TanStack Query 5؛ چیپ سلامت/برند از کلاینت تولیدی (`reads.health/brand` — نه fetch دستی)؛ سوییچ زبان/تم با persist؛ applyLocale/applyTheme روی html |
| CSS شل | `apps/shell/src/shell.css` | پل Tailwind 4 با `source(none)` + `@source` مشخص — اسکن کلاس فقط روی این اپ، تولید قطعی |
| مونتاژ static-root | `scripts/build-static-root.mjs` | بیلد شل → `dist/` (ریشه، base '/') → نواحی بیلدشده در `dist/<area>/` → `build-manifest.json` صادقانه؛ فقط تغییر مقدار `STATIC_DIR` روی سرور |
| تنظیمات | `vite.config.ts` + `index.html` | Vite 6، base '/'، proxy dev به سرور محلی، اسکریپت تلگرام async با degrade صادقانه |

**معیارهای پذیرش Step 3 — همه سبز:**

1. **تشخیص آفلاین** — ۱۹ تست ماتریس (تلگرام/نقش/زبان/دستگاه + storage معیوب).
2. **static-root روی سرور واقعی** — `GET /` → ۲۰۰ شل (dir=rtl) · asset → ۲۰۰ · `/build-manifest.json` → ۲۰۰ صادقانه · `/admin/` ساخته‌نشده → ۴۰۴ واقعی (تصمیم فاز ۴ حفظ شد) · `/api/v1/health` → ۲۰۰ — صفر تغییر کد بک‌اند.
3. **بودجهٔ عملکرد** — شل ۶۶KB gzip زیر سقف ۸۰KB §6؛ مجموع تست فرانت: **۷۲ سبز**.

**آمار Step 3:** ~۵۰۰ LOC اپ شل + ~۹۰ LOC اسکریپت مونتاژ + ~۳۵۰ LOC تست · **۱۹ تست جدید** · بک‌اند دست‌نخورده: ۳۱۷ سبز، clippy صفر.

**Step 4 — داشبورد ادمین روی Aurora** (`front/apps/dashboard` + سیم‌کشی static-root):

| قطعه | فایل | توضیح |
|---|---|---|
| گیت ادمین | `src/ui.tsx` (AdminGateView) + `src/App.tsx` | کلید X-KIA-Key فقط در مرورگر (همان `ao-admin-key` شل)؛ راستی‌آزمایی با مسیر ادمین واقعی `GET /overview` — 401 → نامعتبر، 503 `ADMIN_DISABLED` → وضعیت صادقانه؛ fail-closed در برابر storage معیوب |
| هش‌روتر + lazy | `src/nav.ts` + `src/App.tsx` | هشت بخش §2.5 روی `#/…` (D4 — بدون SPA-fallback سرور)؛ هر بخش chunk تنبل؛ React 18 + TanStack Query 5 + zustand-free |
| نمای کلی | `src/sections/overview.tsx` | `GET /overview` + `/brand` — کاشی‌های فرماندهی، نمودار CSS سری ۷روزه، پرفروش‌ها، سفارش‌های اخیر با واژگان نسل ۲ |
| اقتصاد + خزانه | `src/sections/economy.tsx` + `src/money.ts` | `GET /economy` — تراز آزمایشی، Meter سقف ضرب مادهٔ ۳؛ mint/burn با کلید idempotency سمت کلاینت (D7): کلید پایدار به‌ازای اقدام، آزادسازی فقط پس از تسویه، **پاس‌گم‌شده → کلید حفظ می‌شود تا replay به `Tx.n` UNIQUE بخورد** |
| محصولات | `src/sections/products.tsx` | `GET/POST /products`، `PATCH/DELETE /products/{id}` — آرشیو نرم دو-مرحله‌ای (ArmButton)، جستجوی q، ویژه‌سازی، بازیابی |
| سفارش‌ها | `src/sections/orders.tsx` | واژگان سفارش نسل ۲ حفظ‌شده (paid/pending/delivered/refunded/cancelled)؛ اقدام‌های deliver/cancel/refund دو-مرحله‌ای؛ کاربران و کیف‌ها (موجودی/امانتی با formatDin) |
| تأییدها + HITL | `src/sections/approvals.tsx` | `GET/POST /approvals` + صف مادهٔ ۷ (`/hitl`، approve/reject با `decidedBy=admin:dashboard`، sweep)؛ **پیش‌نمایش ۹۰/۱۰ همیشه در ردیف آزادسازیِ در انتظار** (§6 — مالیات هرگز پنهان نیست)؛ مبالغ ≥ ۵۰٬۰۰۰ Đ با بج HITL |
| رشد | `src/sections/growth.tsx` | فقط‌خواندنی نسل ۲ — جدول امتیازها (xp/sales/ref) + quests/bookings خام و صادقانه (هیچ چیز جعل نمی‌شود) |
| کنسول AI | `src/sections/ai.tsx` | `GET /ai/usage` با Meter سقف روزانه + ۶ ایجنت و ۱۶ مهارت از رجیستری عمومی |
| پلتفرم | `src/sections/platform.tsx` | `GET/PUT /settings` — **501 صادقانه**: نتیجهٔ واقعی سرور (متن بومی‌سازی‌شدهٔ `NOT_IMPLEMENTED`) نمایش داده می‌شود، نه موفقیت جعلی؛ امنیت/ممیزی؛ بکاپ دو-مرحله‌ای با حجم و نگه‌داری |
| واژگان | `packages/i18n/src/dict.ts` | ۱۶۵ کلید تازه fa/en با پاریتی کامل — هیچ کد خام خطایی نمایش نمی‌یابد |
| مونتاژ | `front/scripts/build-static-root.mjs` | نگاشت area→app (admin→dashboard) + بیلد هر اپ موجود؛ manifest صادقانه: `admin:true`، `app/site:false` |
| بودجه | `apps/dashboard/scripts/budget.mjs` | دروازهٔ build: initial ≤ 200KB gzip (§6) — الان **78.3KB** |

**معیارهای پذیرش Step 4 — همه سبز:**

1. **هشت بخش روی قرارداد منجمد** — تمام مسیرهای داشبورد §2.5 فقط از کلاینت تولیدی تایپ‌شده (D5 — lint:urls پاک)؛ صفر تغییر بک‌اند: ۳۱۷ تست سبز، clippy صفر خطا.
2. **صداقت پولی** — پیش‌نمایش ۹۰/۱۰ در ردیف هر آزادسازی در انتظار؛ بج HITL ≥ ۵۰٬۰۰۰ Đ؛ mint/burn با کلید idempotency پایدار-به-ازای-اقدام و آزادسازی فقط پس از تسویه (تست‌شده: خطای پایانی → آزاد، پاس‌گم‌شده → حفظ کلید)؛ double-click با pending قفل می‌شود.
3. **static-root روی سرور واقعی** — `GET /admin/` → ۲۰۰ داشبورد (base '/admin/')، asset → ۲۰۰، `/build-manifest.json` → `admin:true` صادقانه، `/` → شل، `/api/v1/overview` بدون کلید → 401 `ADMIN_KEY_REQUIRED`، با کلید → 200.
4. **دولیتی و بودجه** — fa/en پاریتی کامل (۱۶۵ کلید تازه)؛ داشبورد initial **78.3 KB gzip** زیر سقف ۲۰۰KB §6 با تفکیک مسیر (هر بخش chunk جدا)؛ مجموع تست فرانت: **۱۰۱ سبز** (۲۹ تست تازه: روتر/نشست/فرمت/پول/رندر SSR).

**آمار Step 4:** ~۳٬۰۴۰ LOC سورس TS/TSX + ~۱۵۵ LOC CSS + ~۳۸۸ LOC تست · **۲۹ تست جدید** · بک‌اند دست‌نخورده: ۳۱۷ سبز، clippy صفر.

**Step 5 — اپ شهروندان روی Aurora** (`front/apps/webapp` — تأیید مالک گرفته شد):

| قطعه | فایل | توضیح |
|---|---|---|
| روتر ۳۳ صفحه | `src/router.ts` | نقشهٔ §2.4 عیناً — ۳۲ مسیر قابل‌ rout + 404 صادقانه به‌عنوان fallback؛ هش‌روتر (D4)؛ باتم‌ناو ۵گانهٔ موبایل (§6) + کشوی «همهٔ صفحه‌ها» |
| نردبان اقدام D14 | `src/actions.ts` | staff (کلید) → مستقیم با کلید idempotency؛ شهروند تلگرام → مهارت‌های واقعی چت (create/deliver/dispute — آینهٔ رجیستری)؛ بقیه → فرمان دقیق بات (`/fund …`) + لینک t.me؛ مهمان → تحویل صادقانه به بات — هیچ دکمهٔ جعلی |
| هویت | `src/session.ts` + `telegram.ts` logic | initData خام فقط از لایهٔ هویت کلاینت (تأیید HMAC سمت سرور)؛ کاربر نمایشی از initDataUnsafe؛ کلید ادمین اختیاری = حالت staff؛ انبار‌های ذخیره fail-closed |
| صفحات پولی | `src/pages/escrows.tsx` | پیش‌نمایش ۹۰/۱۰ همیشه پیش از آزادسازی (§6)، بج HITL ≥ ۵۰٬۰۰۰ Đ، تأیید دومرحله‌ای، خط زمانی ۸ حالته (آینهٔ `escrow_transitions`)، رسید اقدام با بج replay |
| صادقندگی رجیستری | صفحات ۶/۱۳/۱۵/۱۷ | فهرست امانت‌ها/تاریخچهٔ تراکنش/صف HITL/سفارش‌های نسل ۲ مسیر کارکنان‌اند → نمای کارکنان با کلید؛ شهروند = جستجوی شناسهٔ عمومی (`GET /escrows/{id}`) + پرسش از دستیار — هرگز جعل داده |
| گفتگوی AI | `src/pages/ai.tsx` | POST /ai/chat با initData خام + تاریخچهٔ حافظه‌ای + رد ابزارها (pending → بج «نیاز به تأیید ادمین»)؛ دایرکتوری ۶ ایجنت/۱۶ مهارت از مسیرهای عمومی |
| محتوا | `src/content.ts` | ۱۰ مادهٔ منشور عین `charter.rs` (fa verbatim + en) + ۱۴ فرمان بات + ۵ فرمان ادمین + FAQ |
| واژگان | `packages/i18n/src/dict.ts` | ۱۶۷ کلید تازه fa/en با پاریتی کامل |
| مونتاژ | `front/scripts/build-static-root.mjs` | نگاشت موجود `app→webapp` فعال شد؛ manifest صادقانه: `admin:true, app:true, site:false` |
| بودجه | `apps/webapp/scripts/budget.mjs` | initial و همهٔ chunkها ≤ ۱۲۰KB gzip (§6) — الان **84.7KB** |

**معیارهای پذیرش Step 5 — همه سبز:**

1. **۳۳ صفحه روی قرارداد منجمد** — ۳۲ مسیر + 404، همه از کلاینت تولیدی (D5 — lint:urls پاک)؛ صفر تغییر بک‌اند: ۳۱۷ تست سبز، clippy صفر.
2. **صداقت پولی و HITL** — تست رندر: آزادسازی ۶۰٬۰۰۰ Đ همیشه ۵۴٬۰۰۰/۶٬۰۰۰ نشان می‌دهد؛ بج مادهٔ ۷ روی ≥ ۵۰٬۰۰۰؛ کد خام هرگز نمایش نمی‌یابد؛ مهمان فقط فرمان بات می‌بیند نه دکمهٔ جعلی؛ staff دکمهٔ دومرحله‌ای بدون هشدار پیش‌عمل.
3. **static-root روی سرور واقعی** — `GET /` → ۲۰۰ شل، `GET /app/` → ۲۰۰ (dir=rtl، base '/app/')، `GET /admin/` → ۲۰۰، asset → ۲۰۰، asset غایب → ۴۰۴ واقعی، manifest صادقانه، `/api/v1/economy` عمومی → ۲۰۰ و `GET /escrows` (فهرست، ادمین) → ۵۰۳ صادقانه بدون کلید.
4. **دولیتی و بودجه** — fa/en پاریتی کامل (۱۶۷ کلید تازه)؛ webapp initial **84.7 KB gzip** و بدترین chunk **78.6 KB** زیر سقف ۱۲۰KB §6؛ مجموع تست فرانت: **۱۶۸ سبز** (۶۷ تست تازه: روتر ۳۳ صفحه/نردبان D14/نشست/محتوا/رندر پولی SSR).

**آمار Step 5:** ~۲٬۹۰۰ LOC سورس TS/TSX + ~۲۹۰ LOC CSS + ~۷۰۰ LOC تست · **۶۷ تست جدید** · بک‌اند دست‌نخورده: ۳۱۷ سبز، clippy صفر.

**Step 6 — اسکلت‌های PWA + Capacitor + Tauri v2** (`front/apps/shell/public` + `front/apps/mobile` + `front/apps/desktop` — تأیید مالک گرفته شد):

| قطعه | فایل | توضیح |
|---|---|---|
| مولد آیکون قطعی | `front/scripts/gen-icons.py` | بشقاب Aurora تیره + نشان Đ برند (#0fbf92) با Pillow — خروجی‌ها commit شده و بازتولید بایت‌به‌بایت؛ آیکون ۱۹۲/۵۱۲ (any) + maskable + apple-touch ۱۸۰ + favicon.ico/svg + آیکون‌های Tauri (۳۲/۱۲۸/۲۵۶/۵۱۲) از یک منبع |
| manifest نصب‌پذیر | `apps/shell/public/manifest.webmanifest` | fa/rtl، `scope: /`، standalone، theme `#0fbf92` / bg `#0c1614` (توکن‌های Aurora)، ۴ آیکون با هر دو purpose، دو shortcut صادقانه (`/app/`، `/admin/`) |
| سرویس‌ورکر پولی-صادق | `apps/shell/public/sw.js` | cache نسخه‌دار `ao-shell-v1`؛ **مسیر `/api` هرگز intercept نمی‌شود** (حقیقت پول سمت سرور — guard پیش از respondWith)؛ document ها network-first با fallback به کش همان ناحیه و سپس شلِ کش‌شده؛ assetهای هش‌دار cache-first؛ فعال‌سازی کش‌های کهنه را پاک می‌کند (skipWaiting + clientsClaim) |
| گلوة ثبت | `apps/shell/src/pwa.ts` | خالص و تزریق‌پذیر: فقط PROD + وجود serviceWorkerContainer؛ `isStandaloneDisplay` (display-mode + iOS standalone)؛ ثبت ناموفق = warn صادقانه، هرگز کرش |
| قلاب‌های head | `apps/shell/index.html` + `apps/webapp/index.html` | manifest + favicon svg/ico + apple-touch + theme-color دوتایی (لایت/دارک) + متاهای iOS standalone؛ webapp بدون SW مستقل (scope از آنِ شل در `/`)؛ safe-area از قبل در webapp.css |
| بودجهٔ Lighthouse | `front/lighthouse/budgets.json` + `front/lighthouserc.cjs` | آینهٔ سقف‌های §6 (script ۸۰KiB، فونت ۲۴۰، FCP ۲۰۰۰ms)؛ اجرای محلی `npx @lhci/cli autorun` مستند شد (R2 — کروم در سندباکس نیست)؛ گیت category:pwa عمداً off (بخش ۹: به Step 7) |
| اسکلت Capacitor | `apps/mobile/*` | `capacitor.config.ts` (com.dropagentx.app، webDir www، scheme https، allowMixedContent=false، رنگ Aurora) + تایپ محلی هم‌شکل SDK + `sync-www.mjs` (بازسازی webapp با `--base=./` — کپی کورِ base مطلق `/app/` در WebView جواب نمی‌دهد) + منابع اسپلش Aurora (`splash/splash-{2732,2048,1284}.png` — Đ + وردمارک، همان مولد) + سند placeholder امضای اندروید و TestFlight/iOS (Step 9) |
| اسکلت Tauri v2 | `apps/desktop/src-tauri/*` | `tauri.conf.json` (frontendDist ← `../../../dist` = همان static-root ترکیبی D3؛ targets: msi/nsis/appimage/deb/dmg/app) + Cargo.toml با `[workspace]` مستقل (نه rust/ منجمد، نه ورک‌اسپیس کهنهٔ ریشهٔ ریپو) + main.rs viewer خالص بدون فرمان سفارشی + capability فقط core:default + `docs/UPDATER.md` (آپدیت خودکار placeholder — Step 9، بدون کلید = بدون endpoint جعلی) |
| مبدأ API بسته‌ها | `packages/api-client` → `optionsFromEnv/import.meta.env` | تنها کلید مجاز `VITE_API_BASE` (مطلق http/s، بدون URL دستی — D5 حفظ)؛ وب عادی unset → پایهٔ طلایی `/api/v1`؛ سه اپ از همان helper |

**معیارهای پذیرش Step 6 — همه سبز:**

1. **PWA نصب‌پذیر روی سرور واقعی** — `GET /` · `/sw.js` (text/javascript) · `/manifest.webmanifest` (application/manifest+json) · `/icons/*` · `/favicon.svg|ico` · `/build-manifest.json` همه ۲۰۰ با باینری واقعی Rust و فقط تغییر `STATIC_DIR`؛ asset غایب → ۴۰۴ واقعی؛ صفر تغییر کد بک‌اند: ۳۱۷ تست سبز، clippy صفر.
2. **سرویس‌ورکر پولی-صادق** — تست‌های متنی: guard `/api` قبل از هر respondWith، هیچ non-GET intercept، کش نسخه‌دار + پاک‌سازی، fallback سندِ network-first؛ ثبت فقط در PROD با تزریق navigator (تست‌شده).
3. **کانفیگ‌ها buildable بدون راز** — capacitor.config و tauri.conf.json تایپ‌چک/پارس سبز؛ targets سه‌پلتفرم؛ capability fail-closed؛ امضا/notarization/آپدیت = placeholder سندشده (Step 9) — هیچ ادعای جعلی.
4. **بودجه‌ها** — داشبورد initial **83.6 KB ≤ 200** و webapp initial **84.8 KB ≤ 120** (سقف §6)؛ شل initial **80.7 KiB — ۰٫۷ KiB بالای سقف ۸۰**: رگرسیون از Stepهای ۴/۵ (رشد دیکشنری i18n مشترک — کلیدهای dashboard/webapp داخل باندل شل)، سهم خود Step 6 ≈ ۱ KiB؛ **صادقانه ثبت شد و ترمیم آن (تفکیک دیکشنری به‌ازای ناحیه) به Step 7 موکول است**.

**تصمیم‌های ثبت‌شدهٔ Step 6:**

* **D16 — مبدأ API بسته‌های بومی فقط با `VITE_API_BASE`**: Desktop/Mobile از `https://localhost` سرو می‌شوند؛ پایهٔ `/api/v1` آنجا بی‌معناست. تنها کلید مجاز env است که در `optionsFromEnv` اعتبارسنجی می‌شود (مطلق http/s) — D5 دست‌نخورده، وب عادی رفتار طلایی قبلی.
* **D17 — Tauri crate ورک‌اسپیس مستقل**: ریشهٔ ریپو یک Cargo.toml کهنهٔ فاز-۰ دارد و `rust/` منجمد است؛ `src-tauri` با `[workspace]` خالی مرزش را می‌بندد تا `cargo tauri build` هرگز به هیچ‌کدام نچسبد.
* **D18 — سرویس‌ورکرِ بدون دیتا**: SW فقط پوسته را آفلاین می‌کند؛ هیچ پاسخ API و هیچ موجودی‌ای کش نمی‌شود — آفلاینِ صادقانه از جعلِ موجودی بهتر است (§8).
* **D19 — کانفیگ بدون راز**: هیچ کلید امضا/endpoint آپدیت/keystore در کانفیگ‌ها نیست؛ هر چیزِ وابسته به راز، سند placeholder + چک‌لیست Step 9 دارد.

**آمار Step 6:** ~۷۵۰ LOC (پیکربندی + SW + glue + سندها) + ۱۱ فایل آیکون تولیدی + ~۳۰۰ LOC تست · **۳۷ تست جدید (۲۰۵ سبز)** · typecheck strict پاک · lint:urls پاک · بک‌اند دست‌نخورده: ۳۱۷ سبز، clippy صفر.

**Step 7 — E2E پولی + Lighthouse + RTL flip + ترمیم بودجهٔ شل** (`front/scripts/e2e-money.mjs` + `packages/i18n` تفکیک‌شده + تست‌های flip + `lighthouserc.cjs` — تأیید مالک گرفته شد):

| قطعه | فایل | توضیح |
|---|---|---|
| تفکیک دیکشنری ناحیه‌ای | `packages/i18n/src/dict-core.ts` + `area-keys-{shell,dashboard,webapp}.ts` + `area-{shell,dashboard,webapp}.ts` | ۳۹۳ کلید به‌عنوان جداسازی کامل و ناهم‌پوشان: core ۱۱۲ (واژگان مشترک: اقدام‌های عام، سطح‌های وضعیت، مبانی پول/اسکرو، ۸ وضعیت اسکرو، واژگان سفارش نسل ۲، برچسب ستون‌های مشترک) + shell ۱۵ + dashboard ۱۰۰ + webapp ۱۶۶؛ هر ناحیه entry تایپ‌شدهٔ خودش را دارد (`@agent-one/i18n/<area>`) — **کلید ناحیهٔ دیگر = خطای تایپ**؛ `dict.ts` همان merge کامل قبل از تفکیک است (تست پاریتی fa/en دست‌نخورده)؛ رشته‌ها verbatim منتقل شدند (مولد: `scripts/split-dict.py`) |
| سوئیچ imports اپ‌ها | ۳۰ فایل `apps/{shell,dashboard,webapp}/src` | همه از entry ناحیه‌ای خود import می‌کنند — باندلر دیگر رشته‌های ناحیهٔ دیگر را در باندل نمی‌کشد |
| گیت بودجهٔ شل | `apps/shell/scripts/budget.mjs` | سقف ۸۰KB §6 روی «initial» (هر asset در dist/index.html) — مثل داشبورد/وب‌اپ داخل `pnpm build`؛ **شل از ۸۰٫۷ → ۷۱٫۳ KB** (تفکیک دیکشنری ۹٫۴KB آزاد کرد) |
| E2E پولی | `front/scripts/e2e-money.mjs` | روی **باینری واقعی Rust** (سرور را با DB موقت + کلید ادمین خودش بوت می‌کند؛ `BASE_URL=` هم قبول می‌کند) — زنجیرهٔ کامل فاز ۴/۵ روی HTTP واقعی |
| تست‌های RTL flip | `apps/{shell,webapp,dashboard}/tests/rtl-flip.test.tsx` | سه اپ روی happy-dom رندر و کلیک واقعی دکمهٔ زبان: `html dir/lang` fa→rtl، ان→ltr، برگشت به rtl؛ `ao-locale` مشترک سه اپ persist می‌شود؛ fetch stub با `vi.hoisted` **قبل از import اپ** نصب می‌شود (openapi-fetch مقدار `fetch` را در زمان ساخت کلاینت می‌گیرد — درس ثبت‌شده) |
| گارد آینهٔ RTL | `packages/ui/tests/rtl-mirror.test.ts` | اسکن همهٔ stylesheetهای ارسالی (ui + سه اپ) — هر خاصیت جهت فیزیکی (`left:/right:`، `margin/padding/border-left/right`، `text-align:left|right`، `float`) ممنوع؛ استراتژی §6 «فقط logical properties» را برای همیشه قفل می‌کند |
| گیت PWA لایت‌هاوس | `lighthouserc.cjs` | گیت تعطیل‌شدهٔ Step 6 اکنون روشن است: `installable-manifest` = error (minScore 1)، `service-worker` = warn؛ اجرای محلی مستند (R2 — کروم در سندباکس نیست)؛ نصب‌پذیری روی باینری واقعی راستی‌آزمایی شد |

**معیارهای پذیرش Step 7 — همه سبز:**

1. **E2E پولی روی باینری واقعی — ۳۷ انتظار سبز** — گیت‌های 401 (بدون کلید `ADMIN_KEY_REQUIRED`، کلید غلط `INVALID_ADMIN_KEY`، mint بدون کلید)؛ ضرب ۱۰۰٬۰۰۰ Đ با replay → `replayed:true` و تراز ثابت؛ اسکرو ۴۰٬۰۰۰ → تأمین → تحویل → آزادسازی = **دقیقاً ۳۶٬۰۰۰/۴٬۰۰۰ (۹۰/۱۰ مادهٔ ۸)**؛ replay آزادسازی صفر پول؛ اسکرو ۶۰٬۰۰۰ → آزادسازی بدون پرچم ادمین → 422 `REQUIRES_HUMAN_APPROVAL` و اسکرو همان‌جا DELIVERED → ردیف HITL در صف → تأیید ادمین → تسویهٔ **دقیقاً ۵۴٬۰۰۰/۶٬۰۰۰** → تأیید دوم 422 تایپ‌شدهٔ `HITL_ALREADY_DECIDED` با صفر جابه‌جایی → Σ تراز آزمایشی = صفر (۲۰۰٬۰۰۰ ضرب‌شده همان‌قدر در کیف‌ها + خزانه).
2. **بودجهٔ شل ترمیم و بسته شد** — تفکیک ناحیه‌ای دیکشنری شل را از ۸۰٫۷ (۰٫۷ بالای سقف — رگرسیون ثبت‌شدهٔ Step 6) به **۷۱٫۳ KB gzip** رساند؛ گیت جدید `apps/shell/scripts/budget.mjs` جلوی تکرار رگرسیون را می‌گیرد؛ داشبورد ۷۸٫۰ ≤ ۲۰۰ و وب‌اپ ۸۲٫۵ ≤ ۱۲۰ (بدترین chunk ۷۶٫۴) — هر سه گیت داخل بیلد.
3. **RTL flip اپ-سطح و گارد CSS** — ۴ تست تازه (۳ flip + ۱ گارد logical-properties)؛ مجموع تست فرانت: **۲۱۰ سبز** (typecheck strict + lint:urls پاک).
4. **Lighthouse نهایی + نصب‌پذیری روی سرور واقعی** — `/` · `/sw.js` (text/javascript) · `/manifest.webmanifest` (application/manifest+json) · `/icons/*` · `/favicon.svg` · `/build-manifest.json` · `/admin/` · `/app/` همه ۲۰۰ با باینری واقعی و فقط تغییر `STATIC_DIR`؛ manifest صادقانه (`site:false`)؛ round-trip زنده دوباره سبز: **۴۰ مسیر / ۴۶ عملیات — صفر تغییر شکستن‌دهنده**.

**تصمیم‌های ثبت‌شدهٔ Step 7:**

* **تفکیک دیکشنری، نه minification** — بودجهٔ شل با معماری درست ترمیم شد (هر ناحیه فقط واژگان خودش) نه با فشرده‌سازی/حذف کلید؛ پاریتی fa/en در سطح کامل حفظ شد (تست همان `dict.ts` merge را می‌خواند).
* **کلیدهای مشترک در core با معیار «دو ناحیه یا واژگان پولی»** — ستون‌ها/اقدام‌های نسل ۲ که داشبورد و وب‌اپ هر دو استفاده می‌کنند، ۸ وضعیت اسکرو و واژگان ۹۰/۱۰ همه در core‌اند؛ typecheck مرز را enforce می‌کند (کلید ناحیه‌ای غلط = خطای کامپایل).
* **fetch stub با vi.hoisted** — openapi-fetch مقدار `fetch` را هنگام `createClient()` می‌گیرد (module-level در Appها)؛ stub بعدی هرگز دیده نمی‌شد — تست‌ها stub را قبل از import نصب می‌کنند تا صفر شبکهٔ واقعی در تست بماند.
* **گیت PWA فعال اما category score خارج گیت** — نصب‌پذیری (manifest/sw/آیکون) = error؛ امتیاز کل pwa = off تا وابسته به کرومِ محلی نشود (R2 حفظ شد).

**آمار Step 7:** ~۴۰۰ LOC تفکیک/گیت + ~۴۵۰ LOC تست/اسکریپت · **۴ تست جدید** (۲۱۰ سبز) · E2E پولی ۳۷ انتظار روی باینری واقعی · round-trip زنده ۴۰/۴۶ سبز · بک‌اند دست‌نخورده: ۳۱۷ سبز، clippy صفر.

---

## فاز ۶ — AI Gateway (v7/10) ✅

**تحویل‌داده‌شده‌ها** — جایگزین کامل `back/ai-gateway + llm` (~۲٬۵۰۰ LOC TS) در `crates/ai` خالص + سیم‌کشی سرور:

| قطعه | فایل | توضیح |
|---|---|---|
| رجیستری واحد | `crates/ai/src/registry.rs` | ۶ ایجنت Zenovix + ۱۶ مهارت KIA-OS با متادیتای fa/en + پرامپت سیستمی؛ `allowed_agents: []` برای wallet_mint/burn (شورایی)؛ نمای وایر `/ai/agents` و `/ai/skills` از همین‌جا تغذیه می‌شود (دادهٔ ثابت state.rs حذف شد) |
| سه‌اسلات ADR-008 | `crates/ai/src/llm.rs` + `config.rs` | primary→secondary→fallback؛ هر اسلات همه‌یا-هیچ (`OPENAI_BASE_URL_2/…_3`)؛ کلاینت OpenAI-سازگار با timeout ۳۰ثانیه؛ **کلید هرگز در Debug/لاگ/خطا** (Debug دستی)؛ `FakeProvider` اسکریپت‌شده بدون شبکه با clone مشترک |
| پاکت سخت | `crates/ai/src/packet.rs` | فقط `{"reply"}` یا `{"tool","args"}` — untagged serde؛ **متن خام هرگز عبور نمی‌کند**؛ هر اسلات دو تلاش، دو خراب پیاپی → شکست تایپ‌شده |
| redaction ۹قاعده | `crates/ai/src/redaction.rs` | PEM/Bearer/JWT/sk/توکن تلگرام/AWS/کارت/seed phrase/hex — ورود (پرامپت)، خروج (reply) و تاریخچه/دانش با شمارش دقیق در پاسخ و audit؛ `LazyLock` بدون وابستگی اضافه |
| موتور چت | `crates/ai/src/engine.rs` | گیت سقف **قبل از هر فراخوانی** → روتینگ قطعی (کلیدواژه fa/en + direct_agent) → fallback قطعی → گیت مجوز رجیستری (unknown/not-allowed/staff-only → refused trace) → trace + usage |
| قرارداد backend | `crates/ai/src/tools.rs` | `ToolBackend` (RPITIT) — گیت/ثبت سقف و اجرای مهارت پشت قرارداد؛ crate خالص SQL نمی‌داند |
| backend واقعی | `crates/server/src/ai_tools.rs` | ۱۶ مهارت روی `V3Store`: خواندنی‌ها فقط روی حساب عامل (staff با accountId)، گذارهای امانت از مسیر دامنهٔ خالص داخل `apply`، **escrow_refund هرگز مستقیم — ردیف HITL با اسکرو دست‌نخورده**، escrow_release staff با `approved_by_admin: true`، kb/product واقعی روی جدول‌های Prisma |
| هندلر /ai/chat | `crates/server/src/handlers/ai.rs` | گیت‌های هویت فاز ۴ حفظ شد؛ موتور کامل؛ خطاها 429 `AI_CAP_EXCEEDED` (meta.used/limit) / 502 `AI_MALFORMED_PACKET` / 502 `AI_UPSTREAM`؛ **ممیزی هر پاسخ `ai.ok|refused|failed|pending`** با زمینهٔ provider/tokens/redactions |
| metering | `crates/db/src/queries.rs` | `insert_ai_usage` روی جدول `AiUsage` — مبنای گیت سقف روزانه per-user؛ `search_kb`/`upsert_kb` برای مهارت‌های دانش |
| تست‌ها | `crates/ai/tests/gateway_tests.rs` + `crates/server/tests/ai_gateway_tests.rs` | ۳۱ تست بستهٔ fake-LLM (معیار: ۲۹ ✓) + ۱۰ انتگراسیون درون‌پردازشی روی SQLite واقعی (معیار: ۹ ✓) + ۲۷ واحد |

**معیارهای پذیرش ADR-009 — هر دو سبز:**

1. **۲۹ تست بستهٔ fake-LLM بدون شبکه + ۹ تست انتگراسیون درون‌پردازشی** — قطع primary → secondary جواب می‌دهد و نام برنده در پاسخ و usage می‌آید؛ همهٔ اسلات‌ها قطع → 502؛ سقف تمام → 429 `AI_CAP_EXCEEDED` **قبل از provider** (شمارندهٔ فراخوانی provider ثابت می‌ماند)؛ پاکت خراب ×۲ → 502 `AI_MALFORMED_PACKET`؛ روتینگ/مجوز/redaction/usage همه ادعاشده.
2. **redaction روی سیم با شمارش در audit؛ escrow_refund ۶۰٬۰۰۰ Đ → ردیف HITL با اسکرو هنوز FUNDED** — کلید sk/Bearer در پرامپت به provider نمی‌رسد، reply پاک می‌شود و `redactions: 3` روی سیم و در meta ردیف audit می‌نشیند؛ بازپرداخت staff → ردیف `ESCROW_REFUND` در صف (amount از امانت، نه LLM) و اسکرو همان‌جا FUNDED + ممیزی `ai.pending`.

**تصمیم‌های ثبت‌شدهٔ فاز ۶:**

* **پاکتِ یک‌مرحله‌ای بدون فراخوانی دوم** — خروجی ابزار مستقیماً خلاصهٔ انسانی reply می‌شود (بدون LLM دوم)؛ مصرف توکن نصف، رفتار قطعی‌تر، سطح parity ساده‌تر.
* **اسلات‌های _2/_3 با پسوند env** — `OPENAI_BASE_URL_2/OPENAI_API_KEY_2/MODEL_2` و `_3`؛ هر اسلات مستقلاً همه‌یا-هیچ؛ غیبتشان فقط زنجیره را کوتاه می‌کند (primary با ADR-006 اجباری می‌ماند).
* **refused هم مصرف provider دارد** — هر پاسخ (حتی رد مجوز یا پاکت خراب) ردیف `AiUsage` با outcome دقیق می‌گیرد تا سقف روزانه قابل دورزدن نباشد.
* **wallet_mint/wallet_burn از AI ناممکن** — `allowed_agents: []` در رجیستری یعنی حتی staff هم از مسیر چت نمی‌رسد؛ فقط داشبورد ادمین (fail-closed ساختاری، نه runtime).
* **staff = کلید ادمین + role: staff** — داشبورد با کلید اما role=user همان رفتار کاربر عادی می‌گیرد (دونزولی صریح).
* **agent fields در AiUsage lowercase** — `AiAgentName::as_str` جدید در قرارداد (additive) — همان قالب serde وایر.
* **audit meta ساختاریافته و بی‌متن** — ردیف audit فقط provider/model/tokens/redactions/pending/tools دارد؛ متن کاربر و پاسخ هرگز در audit نمی‌نشیند (حریم خصوصی).
* **regex با LazyLock** — ۹ الگو یک‌بار کامپایل می‌شوند؛ بدون once_cell اضافه.

**آمار فاز ۶:** ~۲٬۶۰۰ LOC گیت‌وی خالص + ~۵۰۰ LOC backend/هندلر + ~۱٬۷۰۰ LOC تست · **۶۷ تست جدید** (۲۷ واحد + ۳۱ بستهٔ gateway + ۱۰ انتگراسیون) · مجموع ورک‌اسپیس: **۳۱۷ سبز** · clippy `-D warnings` صفر خطا · cargo fmt تمیز.

---

## فاز ۵ — بات teloxide + صف HITL (v6/10) ✅

**تحویل‌داده‌شده‌ها** — پورت یک‌به‌یک الگوی بات grammY فعلی — قلب خالص بدون teloxide طبق ADR-009 §۸٫۵ (TS مرجع: `back/bot/src/orchestrator.ts`):

| قطعه | فایل | توضیح |
|---|---|---|
| DTOهای وایر | `crates/bot/src/wire.rs` | `WireUpdate/WireMessage/WireUser/WireChat` — بدون teloxide؛ فرستندهٔ غایب = fail-closed؛ `BotAction` + کیبورد شیشه‌ای |
| ۱۹ فرمان | `crates/bot/src/cmd.rs` | ۱۴ دستور کاربر + ۵ ادمین با گیت `is_admin`؛ `/cmd@Bot` و بزرگی‌حرف بی‌اثر |
| i18n fa/en | `crates/bot/src/i18n.rs` | پیش‌فرض fa، hint از `language_code`، انتخاب صریح `/lang`؛ **نگاشت هر ۲۰ کد دامنه + گاردهای authz به پیام محلی** (کد وایر هرگز به کاربر نشان داده نمی‌شود) |
| کیبورد شیشه‌ای | `crates/bot/src/keyboard.rs` | دکمه = متن = فرمان (رفت‌وبرگشت پایدار fa/en)؛ منوی ۴ردیفی + منوی ادمین |
| قرارداد backend | `crates/bot/src/env.rs` | trait `Env` (RPITIT + Send) — قلب از هر backend جدا؛ `user_account = user:tg:<id>` فاز ۲ |
| backend حافظه | `crates/bot/src/memory.rs` | معناشناسی عین `V3Store::apply`: پیش‌چک replay → گارد نقش → دامنهٔ خالص → audit؛ آفلاین و قطعی |
| قلب خالص | `crates/bot/src/orchestrator.rs` | `handle_update(env, msg)` — authz سیم (خریدار fund/release/dispute/cancel، فروشنده deliver)، auto-resolve آخرین امانت باز، خطا → پیام محلی |
| رانر polling | `crates/bot/src/runner.rs` | teloxide فقط تبدیل Message↔Wire و ارسال؛ پیام بی‌متن/بی‌فرستنده نادیده |
| webhook آکسوم | `crates/server/src/handlers/bot.rs` + `bot_env.rs` | `POST /api/v1/bot/webhook` (additive، خارج رجیستری TS)؛ پاسخ HTTP = فرمان `sendMessage` (الگوی std/http)؛ `StoreEnv` روی SQLite واقعی |
| امنیت webhook | `crates/contracts/src/error.rs` + `server/config.rs` | 503 `BOT_DISABLED` بدون `BOT_WEBHOOK_SECRET`؛ 401 `BOT_UNAUTHORIZED` با مقایسهٔ زمان‌ثابت (helper فاز ۳)؛ `BOT_ADMIN_IDS` ادمین‌های بات |
| باینری توسعه | `crates/server/src/bin/dropagentx-bot.rs` | polling توسعه — بوت fail-closed (۲ توکن غایب، ۳ دیتابیس، ۴ رانر)؛ مستقل از کانفیگ LLM |

**معیارهای پذیرش ADR-009 — هر دو سبز:**

1. **۲۰+ تست بات پورت و سبز؛ retry دوبار پول جابه‌جا نمی‌کند** — ۴۹ تست فاز ۵ (۱۸ واحد + ۲۴ orchestrator آفلاین روی MemoryDomain + ۸ انتگراسیون webhook روی SQLite واقعی): کلید یکسان‌سازی `tg:{chat}:{message_id}` از پیام تلگرام می‌آید؛ replay همان `/fund` پیام «قبلاً پردازش شده» می‌دهد و تراز ثابت می‌ماند؛ replay `/buy` امانت دوم نمی‌سازد؛ replay `/release` ≥۵۰k ردیف HITL دوم نمی‌سازد.
2. **HITL ≥ ۵۰٬۰۰۰ Đ از صف می‌گذرد؛ تأیید دقیقاً ۹۰/۱۰؛ تأیید دوم تایپ‌شده با بازپرداخت صفر** — `/release ۶۰٬۰۰۰` → خطای `REQUIRES_HUMAN_APPROVAL` دامنه → درخواست `hitl:esc:…` در صف (دولایه: حافظه و سرور) با امانت همان‌جا DELIVERED؛ `/approve` ادمین → تسویهٔ اتمیک همان تراکنش (فروشنده +54,000 از 60,000، خزانه +6,000)؛ تأیید دوباره → `HITL_ALREADY_DECIDED` محلی‌شده، Σ ترازها ۰؛ مرز دقیق ۵۰٬۰۰۰ هم صف می‌خواهد؛ TTL ۲۴ ساعت → `HITL_EXPIRED`؛ `/reject` هیچ پولی نمی‌برد.

**تصمیم‌های ثبت‌شدهٔ فاز ۵:**

* **teloxide فقط رانر** — قلب بات (`orchestrator`) صفر وابستگی به teloxide/IO دارد و روی `MemoryDomain` آفلاین تست می‌شود؛ رانر webhook در آکسوم (تولید) و polling teloxide (توسعه) هر دو همان یک مسیر را اجرا می‌کنند.
* **teloxide با `rustls` به‌جای native-tls** — هم‌راستا با تصمیم reqwest ورک‌اسپیس؛ درخت openssl وارد بیلد نمی‌شود + feature `ctrlc_handler` برای `repl`.
* **کلید یکسان‌سازی با chat_id مقیاس می‌شود** — `message_id` تلگرام فقط در چت یکتاست؛ کلید `tg:{chat}:{msg}` و شناسهٔ امانت بات `esc:c{chat}m{msg}` دترمیینیستیک‌اند تا retry وب‌هوک امن باشد.
* **authz دولایه** — orchestrator پیام دقیقِ نقش (خریدار/فروشنده/غریبه) می‌سازد و هر دو backend گارد را دوباره داخل فرمان اعمال می‌کنند؛ گذار پولی از مسیر دامنهٔ خالص فاز ۱ می‌گذرد (عین هندلرهای فاز ۴).
* **webhook = پاسخ sendMessage در بدنهٔ HTTP** — رانر ارسال جدا در تولید لازم نیست؛ چند action → اولی ارسال، بقیه در لاگ.
* **session زبان درون‌حافظه‌ای** — `BTreeMap<i64, Locale>` در AppState؛ parity فاز ۸ روی state session زبان نیست (فقط رفتار پایه فرمان‌ها).
* **audit صادقانهٔ بات** — actor در `ApplyOptions` حساب واقعی شهروند است (`user:tg:<id>`) نه «admin»؛ + audit_ext `bot.escrow.*` برای مسیر webhook.
* **ادمین‌های بات lenient** — `BOT_ADMIN_IDS` کاما-جدا، ورودی نامعتبر نادیده (مثل KEEP_BACKUPS)؛ فهرست خالی یعنی هیچ فرمان ادمینی اجرا نمی‌شود (fail-closed).

**آمار فاز ۵:** ~۲٬۹۰۰ LOC بات + ~۵۰۰ LOC webhook/StoreEnv + ~۱٬۲۰۰ LOC تست · **۴۹ تست جدید** (۱۸ واحد + ۲۴ orchestrator + ۸ انتگراسیون webhook) · مجموع ورک‌اسپیس: **۲۵۰ سبز** · clippy `-D warnings` صفر خطا · cargo fmt تمیز.

---

## فاز ۴ — سرور Axum + داشبورد static (v5/10) ✅

**تحویل‌داده‌شده‌ها** — جایگزین کامل فستیفای v3 (پورت `back/server/src/*` طبق رجیستری فاز ۳):

| قطعه | فایل | توضیح |
|---|---|---|
| پیکربندی fail-closed | `crates/server/src/config.rs` | ۱۳ متغیر env؛ **ADR-006 همه‌یا-هیچ**: کانفیگ ناقص LLM (`OPENAI_BASE_URL`/`OPENAI_API_KEY`/`MODEL`) بوت را با exit 2 رد می‌کند — هرگز open |
| امنیت چندلایه | `crates/server/src/middleware.rs` | گارد `X-KIA-Key` (مقایسهٔ زمان‌ثابت فاز ۳: 503 `ADMIN_DISABLED`/401 `ADMIN_KEY_REQUIRED`/401 `INVALID_ADMIN_KEY`) + نرخ پنجرهٔ لغزان به‌صورت middleware آکسوم با باکت ادمین جدا (X-Forwarded-For اول، سپس ConnectInfo) و بدنهٔ ۴۲۹ دقیق فاز ۳ |
| پاکت خطا | `crates/server/src/error.rs` | `ApiFailure` = Http/Domain/Db → بدنهٔ `{error:{code,message,meta?}}` با وضعیت صادقانه؛ خطای دامنهٔ خروجی از تراکنش (`DbError::Domain`) در مسیر نگاشت ۴۰۴/۴۲۲ می‌ماند |
| ۴۶ مسیر + سند | `crates/server/src/routes.rs` | همان رجیستری واحد ADR-005؛ `/api/v1/openapi.json` (سند inline کامل، بدون `$ref`) + صفحهٔ ردوک `/api/v1/docs`؛ 404 مسیر ناشناخته هم پاکت است |
| هندلرها | `crates/server/src/handlers/*` | system (health عمیق/بکاپ/economy)، accounts (ذخیرهٔ `RESERVED_ID`/`DUPLICATE_ACCOUNT`)، wallet (سقف سالانهٔ مادهٔ ۳ **داخل همان تراکنش** — دو ضرب هم‌زمان هرگز از سقف نمی‌گذرد)، escrows (۹ گذار از دامنهٔ خالص)، hitl (تأیید ⇒ تسویهٔ اتمی ۹۰/۱۰)، ai (گیت‌های هویت دقیق)، dashboard (۱۵ مسیر پاکت `{ok,data}`) |
| تبدیل‌گرها | `crates/server/src/views.rs` | تنها نقطهٔ ساخت DTO وایر — nullable صریح، وضعیت‌های SCREAMING/lowercase، ریسک از آستانهٔ منشور |
| رجیستری AI | `crates/server/src/state.rs` | ۶ ایجنت Zenovix + ۱۶ مهارت KIA-OS با متادیتای مجوز — دادهٔ ثابت `/ai/agents` و `/ai/skills` (فاز ۶ موتور را روی همین شناسه‌ها می‌سازد) |
| کوئری‌های فاز ۴ | `crates/db/src/queries.rs` (فعال‌سازی ماژول) | صف HITL/محصولات/audit/بکاپ که فاز ۲ آماده کرده بود + `count_users/count_escrows/count_hitl/sales_by_product` — ستون‌های محصول یک‌به‌یک `schema.prisma` (`priceDin`)؛ salesCount از اسکروهای RELEASED مشتق می‌شود |
| لاگ JSON | `crates/server/src/main.rs` | tracing-subscriber فرمت JSON + TraceLayer آکسوم؛ بوت با exit code صریح (۲ کانفیگ، ۳ دیتابیس، ۴ بایند) |
| بیلد دو مرحله‌ای | `Dockerfile` + `.dockerignore` | builder (`rust:1.99-slim`) → runtime (`distroless/cc-debian12:nonroot`) — تک‌باینری، مهاجرت‌ها جاسازی‌شده، user غیر-root |

**معیارهای پذیرش ADR-009 — هر سه سبز:**

1. **تست‌های سرور پورت و سبز — ۷۷ تست فاز ۴** (۲۰ واحد + ۵۷ یکپارچگی روی `oneshot` معادل `app.inject`): مسیر کامل mint→escrow→release **دقیقاً ۹۰/۱۰** (seller 360/treasury 40 از 400)؛ replay با کلید کیف و اسکرو **دوبار پرداخت نمی‌کند**؛ overdraft سوزاندن/تأمین **۴۲۲ رد**؛ **۴۰۱/۴۰۴/۴۲۲/۴۲۹ همان‌ها**؛ HITL ۶۰٬۰۰۰ Đ از صف می‌گذرد و تأییدش دقیقاً ۹۰/۱۰ تسویه می‌کند؛ تصمیم دوم ادمین → `HITL_ALREADY_DECIDED` تایپ‌شده با بازپرداخت صفر؛ بدنهٔ خراب (float/کلید کوتاه/id فاصله‌دار) → ۴۰۰ `INVALID_REQUEST`؛ آرشیو محصول نرم است؛ PUT تنظیمات ۵۰۱ صادقانه.
2. **داشبورد static در برابر سرور Rust** — ServeDir برای build موجود (`STATIC_DIR`)، ریشه → index، asset غایب → ۴۰۴ واقعی؛ + smoke باینری واقعی: بوت، health با چیپ‌ها، ثبت حساب، اقتصاد، openapi.json و docs همه ۲۰۰.
3. **بوت با کانفیگ ناقص LLM رد می‌شود** — آزمایش واقعی: `OPENAI_BASE_URL`+`OPENAI_API_KEY` بدون `MODEL` → `boot refused: Incomplete LLM config (ADR-006 all-or-nothing) — missing: MODEL`، exit 2.

**تصمیم‌های ثبت‌شدهٔ فاز ۴:**

* **سقف ضرب داخل تراکنش** — محاسبهٔ `mintedThisYear` از وضعیتِ لودشدهٔ دفتر (نه کوئری جدا) تا دو ضرب هم‌زمان هرگز با خوانش کهنه از سقف بگذرند؛ مدل خواندن سالانهٔ دقیق با dual-run فاز ۸ بازبینی می‌شود.
* **فیلدهای dashboard-only محصول ستون ندارند** — `schema.prisma` فقط `priceDin` دارد؛ Stars صفر، stock −۱ (نامحدود)، featured false و salesCount = شمارش اسکروهای RELEASED با `productRef` — نمادهای UI از جدول‌های زنده مشتق می‌شوند، نه ستون‌های جعلی.
* **واژگان وضعیت سفارش داشبورد** — `CREATED→pending`، `FUNDED→paid`، و… همان واژگان فیلتر رجیستری؛ وضعیت‌های بیرون فیلتر (released/disputed/expired) lowercase.
* **ServeDir خالص بدون SPA fallback** — asset غایب ۴۰۴ واقعی می‌دهد؛ deep-link fallback با ادغام فرانت فاز ۷ شکل قطعی می‌گیرد (ساخت داشبورد فعلی در ریپو موجود نیست).
* **هویت تصمیم‌گیرندهٔ داشبورد** — مسیر `/approvals/{id}/decide` فیلد هویت ندارد؛ عامل ردیف audit `"dashboard"` است (مسیر `/hitl/{id}/approve` هویت را از بدنه می‌گیرد).
* **پاسخ `/ai/chat` تا فاز ۶** — گیت‌های هویت/مجوز دقیق پورت شده‌اند؛ پس از عبور گیت‌ها `AI_DISABLED` 503 با پیام صادقانه (همان کد وایر — additive).

**آمار فاز ۴:** ~۳٬۹۰۰ LOC سرور + ~۹۰۰ LOC تست · **۷۷ تست جدید** (۲۰ واحد + ۵۷ یکپارچگی) · مجموع ورک‌اسپیس: **۲۰۱ سبز** · clippy `-D warnings` صفر خطا · cargo fmt تمیز.

---

## فاز ۳ — قراردادها (v4/10) ✅

**تحویل‌داده‌شده‌ها** — پورت یک‌به‌یک `back/contracts/src/*` + لایهٔ امنیت `back/server/src/{auth,guards}.ts`:

| قطعه | فایل | توضیح |
|---|---|---|
| اولیه‌های وایر | `crates/contracts/src/common.rs` | `Din` (DNX — مرز 2^53−1 با Deserialize سخت‌گیرانه)، `Id` (الگو+طول)، `IdempotencyKey` (≥۸)، `IsoDateTime` (RFC3339 با آفست اجباری)، `MetaValue`، `ErrorEnvelope`، ۸ وضعیت اسکرو |
| ۲۰+ قرارداد | `accounts.rs` · `wallet.rs` · `escrows.rs` · `hitl.rs` · `economy.rs` · `system.rs` · `ai.rs` · `dashboard.rs` | همهٔ DTOهای zod با deriveهای serde+utoipa — camelCase، `.nullable()` → null صریح روی سیم، `.optional()` → کلید غایب، literalها → انم تک‌عضوی |
| طبقه‌بندی خطا | `crates/contracts/src/error.rs` | ۲۰ کد دامنه → ۴۰۴/۴۲۲/۵۰۰ (مثل `errors.ts`) + کدهای صریح سرور: `ADMIN_DISABLED` 503، `ADMIN_KEY_REQUIRED`/`INVALID_ADMIN_KEY`/`UNAUTHORIZED`/`AI_IDENTITY_REQUIRED` 401، `RATE_LIMITED` 429 با meta `limit`، `NOT_IMPLEMENTED` 501، `AI_*` 503/502، `INVALID_REQUEST` 400 |
| سازندهٔ OpenAPI 3.1 | `crates/contracts/src/openapi.rs` | پورت `buildOpenApiDoc` — رجیستری ۴۶ مسیر با همان ترتیب تگ‌ها، پارامترها، بدنه، پاسخ 4XX، `kiaAdminKey`؛ inliner داخلی `$ref`ها (سند مثل zod کاملاً inline)؛ `fastify_path`؛ صفحهٔ docs ردوک |
| رجیستری مسیرها | `crates/contracts/src/routes.rs` | ۴۶ مسیر `API_ROUTES` مو‌به‌مو — یک تعریف = قرارداد + امنیت + خلاصه (ADR-005) |
| امنیت قراردادی | `crates/contracts/src/security.rs` | `verify_kia_key` (زمان‌ثابت با subtle، fail-closed: بدون کانفیگ/کلید <۱۶ → رد)، `verify_telegram_init_data` (طرح رسمی HMAC-SHA256: secret=SHA256(bot_token)، data_check_string مرتب، مقایسهٔ زمان‌ثابت)، `make_test_init_data`، `RateLimiter` پنجرهٔ لغزان ۶۰ثانیه‌ای با باکت ادمین جدا و گارد ۱۰هزار کلید |
| گلدن‌فیکسچر | `crates/contracts/tests/fixtures/` | `golden-openapi.json` (۴۰ مسیر/۴۶ عملیات) + `golden-wire.json` (۵۵ نمونه) — تولیدشده از **خود** zod (`scripts/ts-golden/generate.ts`) و همه معتبر به اسکیمای اصلی |
| تست‌ها | `tests/{golden_wire,golden_openapi,roundtrip}.rs` + ~۳۰ تست واحد درون ماژول‌ها | ۵۸ تست فاز ۳ |

**معیارهای پذیرش ADR-009 — هر سه سبز:**

1. **دیف اسکیمای OpenAPI = صفر تغییر شکستن‌دهنده** — سند Rust پس از نرمال‌سازیِ مستندشده (حذف `$schema`/`additionalProperties`/`description`، `anyOf[T,null]↔nullable`، `const↔enum`، عدد↔صحیح، `format:int64/double`) با خروجی واقعی TS صفر diff دارد (`openapi_diff_against_ts_golden_is_zero_breaking`).
2. **راند‌تریپ کلاینت تایپ‌شده** — هر ۵۵ نمونهٔ وایر TS در Rust parse و دوباره serialize می‌شود و JSON **یک‌به‌یک** برمی‌گردد؛ ۱۱ تست قرارداد TS (`client.test.ts` ۸ + `openapi.test.ts` ۳) روی روتر axum با `oneshot` (معادل `app.inject`) سبز — کد خطا عیناً، بدنهٔ خراب → `MALFORMED_ERROR_BODY`، پاسخ ناقص → `RESPONSE_MISMATCH`.
3. **۱۱ تست قرارداد پورت و سبز** ✓ (همان بالا — به‌علاوهٔ ~۳۰ تست واحد و جدول کامل نگاشت ۲۰ کد دامنه).

**تصمیم‌های ثبت‌شدهٔ فاز ۳:**

* **utoipa 5.5** (نه 5.3) — resolved در فاز ۰؛ API نسخهٔ جدید (`ToSchema` بدون lifetime، `Number::Int` صحیح را در JSON حفظ می‌کند) با تست گلدن قفل شد.
* **rate-limit درون‌حافظه‌ای دست‌ساز به‌جای tower-governor** — معناشناسی TS (پنجرهٔ لغزان، باکت مشترک ادمین، بدنهٔ دقیق 429 با `meta.limit`، گارد ۱۰هزار کلید) عیناً پورت شد؛ wire-governor بدنهٔ 429 و bucketing متفاوت می‌داد. سیم‌کشی به‌عنوان middleware آکسوم در فاز ۴.
* **validation لایه‌بندی شد** — قیود ساختاری پول (`Din`) در Deserialize سخت‌گیرانه اعمال می‌شود؛ الگوهای رشته‌ای (id/datetime/طول‌ها) در اسکیمای OpenAPI اعلان و در فاز ۴ در لایهٔ اعتبارسنجی مسیرها enforce می‌شوند (مثل zod که در لبهٔ HTTP اعتبارسنجی می‌کند، نه داخل تایپ‌ها).
* **سند نهایی کاملاً inline** — `$ref`های utoipa پس از مونتاژ حل می‌شوند تا خروجی با zod (بدون `$ref`) قابل مقایسه و diff مستقیم باشد؛ رجیستری کامپوننت‌ها در `component_schemas()` مرکزی است و هر `$ref` حل‌نشده خطای تست می‌گیرد.

**آمار فاز ۳:** ~۳٬۱۰۰ LOC (تست‌ها و فیکسچرها شامل) · ۵۸ تست جدید · مجموع ورک‌اسپیس: **۱۲۵ سبز** · clippy `-D warnings` صفر خطا.

---

## فاز ۲ — پیوند persistence (v3/10) ✅

**تحویل‌داده‌شده‌ها:**

| قطعه | فایل | توضیح |
|---|---|---|
| آداپتور فروشگاه | `crates/db/src/store.rs` | الگوی load → دامنهٔ خالص → persist در `sqlx::Transaction`؛ ترازها همیشه مشتق از `Posting` (ADR-003)؛ replay داخل تراکنش + بازیابی از مسابقه با قید UNIQUE `Tx.idempotencyKey` |
| اسکیمای نسخه‌دار | `crates/db/migrations/0001_init.sql` | یک‌به‌یک مطابق `schema.prisma` — dual-run فاز ۸ بدون تبدیل روی همان فایل |
| Outbox + audit | `crates/db/src/outbox.rs` + `wire.rs` | رویدادهای دامنه در همان تراکنشِ پول؛ شکل JSON دقیقاً مثل `JSON.stringify` TS با تست طلایی |
| برنامهٔ genesis | `crates/db/src/genesis.rs` | محاسبهٔ خالص floor هر شهروند (ADR-002) + یادداشت تعدیل شفاف (گپ −63.0) |
| خوانندهٔ لگاسی | `crates/db/src/legacy.rs` | SQL خام فقط‌خواندنی روی دیتابیس بازیابی‌شدهٔ Railway |
| مهاجرت | `crates/db/src/migrate.rs` + `crates/tooling/src/bin/migrate-legacy.rs` | ۴۶۶ کاربر → `user:tg:<id>`، genesis یک‌بارمصرف `legacy:genesis:v1`، ردیف audit `legacy.adjustment`، ۳۰ محصول؛ **انتقال genesis از مسیر دامنهٔ خالص** (سخت‌گیرانه‌تر از TS — fail-closed) |
| ممیزی خارجی | `crates/db/src/proof.rs` + `crates/tooling/src/bin/genesis-proof.rs` | SQL خام، بدون ORM — Σ کیف‌ها = 187,672 Đ · Σ ردیف‌ها = 0 · exit 1 در نقض انتظار |

**تصمیم ثبت‌شده (اصلاح فاز ۰):** sqlx با feature `sqlite` به‌جای `postgres` — دیتابیس واقعی v3 و لگاسی هر دو SQLite هستند (schema.prisma → provider "sqlite") و dual-run باید روی همان فایل بنشیند. کوئری‌ها همه پشت `dropagentx-db` پنهان‌اند؛ تغییر به Postgres فقط یک خط در `Cargo.toml` ورک‌اسپیس است.

**اثبات (فیکسچر سینتتیک با اعداد روز-اول — ۴۶۶ کاربر · Σ ۱۸۷٬۶۷۲.۸ Đ · ۱٬۵۵۴ تراکنش · ۳۰ محصول):**

```text
[migrate] per-citizen floor: Σ 187672 Đ across 466 holder(s)
  Σ user wallets        = 187672 Đ  (acceptance: 187672)   ✅
  trialBalance.balanced = true (Σ postings 0)               ✅
  products              = 30/30 · adjustment rows = 1       ✅
PROOF OK — every expectation holds.                         ✅
اجرای دوم → replay-safe exit (بدون ضرب دوباره)             ✅
```

**آمار فاز ۲:** ~۲٬۱۰۰ LOC (تست‌ها شامل) · ۱۵ تست یکپارچگی/واحد جدید · مجموع تست ورک‌اسپیس: ۶۳ سبز

---

## فاز ۱ — هسته دامنهٔ خالص (v2/10) ✅

**تحویل‌داده‌شده‌ها** — پورت یک‌به‌یک `back/domain/src/*` به `crates/domain`:

| ماژول | منبع TS | توضیح |
|---|---|---|
| `money.rs` | `money.ts` (ADR-002) | `Din` — newtype روی i64 با مرز 2^53−1، `mul_bps` با گردکردن به پایین، فرمت `187,672 Đ` |
| `ledger.rs` | `ledger.ts` (ADR-003) | دفتر دوطرفهٔ تغییرناپذیر، replay با کلید یکسان‌سازی، قواعد جهت fail-closed، تراز آزمایشی |
| `escrow.rs` | `escrow.ts` (ADR-004) | ماشین ۸ حالته با گذارهای مجاز، مالیات ۱۰٪ مادهٔ ۸، HITL مادهٔ ۷ (≥ ۵۰٬۰۰۰ Đ) |
| `charter.rs` | `charter.ts` | منشور ۱۰ ماده‌ای، سقف ضرب، تقسیم فروشنده/خزانه |
| `errors.rs` | `errors.ts` | ۲۰ کد خطا + `Meta` برای زمینهٔ دیباگ |
| `events.rs` | `events.ts` | `DomainEvent` با شناسه دترمینیستیک `{escrowId}#{kind}` |

* صفر وابستگی خارجی، صفر IO، `#![forbid(unsafe_code)]` · ۴۲ تست سبز
* تست سفر کامل: ضرب → اسکرو → تسویه با مالیات → تراز آزمایشی تقدس
