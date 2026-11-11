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
| 5 | بات teloxide + صف HITL (آستانه ۵۰,۰۰۰ Đ) | ⬜ | — |
| 6 | AI Gateway — ۶ ایجنت، ۱۶ مهارت، سقف و audit | ⬜ | — |
| 7 | ادغام فرانت + WASM اختیاری | ⬜ | — |
| 8 | Dual-run → مهاجرت داده → Cut-over | ⬜ | — |
| 9 | امضای نهایی — تحویل نسخه ۱۰ از ۱۰ | ⬜ | — |

**وضعیت فعلی: v5/10 — سرور Axum کامل (۴۶ مسیر رجیستری واحد + امنیت چندلایه + OpenAPI/ردوک + ServeDir) — کل ورک‌اسپیس ۲۰۱ تست سبز، clippy صفر خطا.**

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
