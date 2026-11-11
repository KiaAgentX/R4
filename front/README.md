# Agent-One — مونوریپو فرانت (فاز ۷، Step 2)

> این درخت **جدا از `rust/`** است و بک‌اند منجمد را دست نمی‌زند — صفر تغییر
> مسیر/شکل/enum. ریشهٔ workspace pnpm به‌صورت زیرپوشهٔ `front/` است تا از
> ریشهٔ cargo workspace جدا بماند (تصمیم D15 در ثبت‌شدهٔ Step 2؛
> `rust/` هیچ ارجاعی به این درخت ندارد).

## ساختار (per STEP1-AUDIT §7)

```
front/
├── apps/
│   └── shell/        # Step 3 — شل راهبردی: تشخیص Telegram/نقش/زبان/دستگاه + تعویض نقش
├── packages/
│   ├── ui/           # Aurora — توکن‌ها، دارک/لایت، RTL، فونت سلف-هاست، money/theme/dir
│   ├── i18n/         # fa/en — دیکشنری + نگاشت کد خطا (آینهٔ بوت i18n)
│   └── api-client/   # تولیدی از golden-openapi.json + کلید idempotency سمت کلاینت
├── scripts/
│   ├── check-no-raw-urls.mjs    # لینت D5 — ممنوعیت URL دستی بیرون api-client
│   ├── build-static-root.mjs    # Step 3 — مونتاژ static-root ترکیبی + build-manifest
│   └── round-trip-live.mjs      # round-trip زنده در برابر سرور Rust
├── pnpm-workspace.yaml
└── package.json
```

`apps/dashboard|webapp|site` در Stepهای ۴ به بعد اضافه می‌شوند؛ glob آن‌ها از
الان در workspace است.

## static-root ترکیبی — بدون هیچ تغییر بک‌اند (D3/D4)

```bash
pnpm build:root   # shell را می‌بندد → dist/ می‌سازد → build-manifest.json صادقانه می‌نویسد
```

```
dist/
├── index.html            ← شل راهبردی (base '/')
├── assets/…              ← دارایی‌های شل (۶۶KB gzip — زیر سقف ۸۰KB)
├── build-manifest.json   ← {shell:true, admin:false, app:false, site:false}
└── admin/ · app/ · site/ ← فقط وقتی بیلدشان موجود باشد کپی می‌شوند (Stepهای ۴+)
```

سرور Rust با همان ServeDir فاز ۴ و فقط تغییر مقدار `STATIC_DIR` سرو می‌کند؛
منوی شل برای ناحیه‌های ساخته‌نشده حالت صادقانهٔ «در گام‌های بعدی» می‌دهد — هرگز
صفحهٔ جعلی نیست و asset غایب ۴۰۴ واقعی می‌ماند (تصمیم فاز ۴ حفظ شده).

## دستورها

```bash
pnpm install                     # corepack/pnpm 10، Node 24
pnpm typecheck                   # tsc سخت‌گیر (strict + noUncheckedIndexedAccess + exactOptionalPropertyTypes)
pnpm test                        # vitest — ۵۳ تست آفلاین
pnpm lint:urls                   # D5 — هیچ URL دستی بیرون packages/api-client
pnpm --filter @agent-one/ui run build          # پل Tailwind 4 (@theme)
pnpm --filter @agent-one/api-client run generate   # بازتولید schema.d.ts از گلدن
BASE_URL=http://127.0.0.1:8080 node scripts/round-trip-live.mjs   # در برابر سرور زنده
```

## معیارهای پذیرش Step 2 — همه سبز

1. **api-client تولیدی از قرارداد منجمد** — `schema.d.ts` (۳٬۳۰۰+ خط) با
   openapi-typescript از `rust/crates/contracts/tests/fixtures/golden-openapi.json`
   تولید می‌شود؛ تست determinism تولید دوباره را با فایل کامیت‌شده بایت‌به‌بایت
   می‌سنجد؛ شکل قرارداد (۴۰ مسیر / ۴۶ عملیات) قفل است.
2. **round-trip زنده** — اسکریپت `round-trip-live.mjs` همان نرمال‌سازی
   «صفر تغییر شکستن‌دهنده» تست گلدن فاز ۳ را پورت می‌کند و در برابر سرور Rust
   واقعی اجرا شد: **live == golden، ۴۰ مسیر / ۴۶ عملیات**.
3. **کیبورد پولی امن (D7)** — ۱۰ عملیات پولیِ وایر (mint/burn/escrows + ۷ گذار)
   کلید یکسان‌سازی ULID سمت کلاینت می‌گیرند: یک‌بار به‌ازای هر عمل منطقی،
   ذخیره در store، استفادهٔ مجدد در retry → دوبار-کلیک/اتصال مجدد هرگز دوبار
   پرداخت نمی‌کند.
4. **دیگر کد وایر دست‌نوشته نیست** — لینت D5 در `check` گیت است.
5. **Aurora** — توکن‌ها در `tokens.css` (لایت/دارک با `[data-theme]` +
   prefers-color-scheme)، فونت سلف-هاست Vazirmatn/Inter (بدون CDN — D9)،
   فقط logical properties → RTL آینهٔ کامل، پل Tailwind 4 از همان توکن‌ها
   (`@theme`)، فرمت پول آینهٔ `format_din` دامنه (تست `187,672 Đ`).
6. **دیکشنری خطا (D8)** — هر ۲۰ کد دامنه با متن عین `crates/bot/src/i18n.rs`
   + همهٔ کدهای وایر سرور + ۳ کد سمت کلاینت؛ کد خام هرگز به کاربر نمی‌رسد.

## معیارهای پذیرش Step 3 — همه سبز (شل راهبردی)

1. **تشخیص زمینه** — `detect.ts` خالص و آفلاین: تلگرام فقط با `initData`
   غیروفارغ (هویت جعل نمی‌شود)، نقش = کلید ادمین ذخیره‌شده > شهروند تلگرام >
   مهمان، زبان = انتخاب ذخیره‌شده > hint تلگرام > navigator > fa (قانون بات)،
   دستگاه با coarse pointer/عرض. ۱۹ تست ماتریس تشخیص.
2. **تعویض نقش + مسیرهای واقعی** — سه ناحیهٔ `/admin/` `/app/` `/site/` با
   slash انتهایی (D3/D4)؛ در دسترس‌بودن از `build-manifest.json` صادقانهٔ
   اسکریپت مونتاژ؛ ناحیهٔ غایب = «در گام‌های بعدی» — هرگز صفحهٔ جعلی.
3. **static-root ترکیبی روی سرور واقعی** — بوت باینری Rust با فقط تغییر
   `STATIC_DIR`: `GET /` → ۲۰۰ شل (dir=rtl)، asset → ۲۰۰، `/admin/` ساخته‌نشده
   → ۴۰۴ واقعی، `/build-manifest.json` → ۲۰۰ صادقانه، `/api/v1/health` → ۲۰۰.
   صفر تغییر کد بک‌اند.
4. **بودجهٔ عملکرد** — شل ۶۶KB gzip (سقف §6: ۸۰KB) با Vite 6 + React 18 +
   TanStack Query 5؛ چیپ سلامت سرور از کلاینت تولیدی می‌خواند — نه fetch دستی.

## آمار Step 2

- ~۱٬۹۰۰ LOC TS + ~۹۰۰ LOC CSS + ~۹۵۰ LOC تست · **۵۳ تست** (۵۳ سبز، آفلاین)
- typecheck strict پاک · lint:urls پاک · بیلد Tailwind ۴ سبز
- round-trip زنده در برابر باینری واقعی سرور سبز
- وابستگی‌های runtime: فقط `openapi-fetch`؛ فونت‌ها داخل بسته (۲۴۰KB، OFL)
