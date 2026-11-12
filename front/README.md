# Agent-One — مونوریپو فرانت (فاز ۷، Step 2)

> این درخت **جدا از `rust/`** است و بک‌اند منجمد را دست نمی‌زند — صفر تغییر
> مسیر/شکل/enum. ریشهٔ workspace pnpm به‌صورت زیرپوشهٔ `front/` است تا از
> ریشهٔ cargo workspace جدا بماند (تصمیم D15 در ثبت‌شدهٔ Step 2؛
> `rust/` هیچ ارجاعی به این درخت ندارد).

## ساختار (per STEP1-AUDIT §7)

```
front/
├── apps/
│   ├── shell/        # Step 3 — شل راهبردی: تشخیص Telegram/نقش/زبان/دستگاه + تعویض نقش
│   │                 # Step 6 — PWA: manifest.webmanifest + sw.js + icons/ + favicon در public/
│   ├── dashboard/    # Step 4 — داشبورد ادمین روی Aurora (هشت بخش §2.5، /admin/)
│   ├── webapp/       # Step 5 — اپ شهروندان: نقشهٔ ۳۳ صفحه §2.4 (مسیرهای عمومی + /ai/chat، /app/)
│   ├── mobile/       # Step 6 — اسکلت Capacitor (config + sync-www + سندهای امضا/TestFlight)
│   └── desktop/      # Step 6 — اسکلت Tauri v2 (src-tauri + targets سه‌پلتفرم + UPDATER.md)
├── packages/
│   ├── ui/           # Aurora — توکن‌ها، دارک/لایت، RTL، فونت سلف-هاست، money/theme/dir
│   ├── i18n/         # fa/en — دیکشنری + نگاشت کد خطا (آینهٔ بوت i18n)
│   └── api-client/   # تولیدی از golden-openapi.json + کلید idempotency + optionsFromEnv
├── scripts/
│   ├── check-no-raw-urls.mjs    # لینت D5 — ممنوعیت URL دستی بیرون api-client
│   ├── build-static-root.mjs    # Step 3+ — مونتاژ static-root ترکیبی + build-manifest
│   ├── round-trip-live.mjs      # round-trip زنده در برابر سرور Rust
│   ├── e2e-money.mjs            # Step 7 — E2E پولی روی باینری واقعی (mint→escrow→release ۹۰/۱۰ + HITL)
│   └── gen-icons.py             # Step 6 — مولد آیکون قطعی (Aurora + Đ)
├── lighthouse/budgets.json + lighthouserc.cjs   # Step 6+7 — بودجهٔ Lighthouse + گیت PWA (§6)
├── pnpm-workspace.yaml
└── package.json
```

`apps/site` در Stepهای بعدی اضافه می‌شود؛ glob آن از الان در workspace است.

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
pnpm test                        # vitest — ۲۱۰ تست آفلاین (شامل RTL flip و گارد CSS)
pnpm lint:urls                   # D5 — هیچ URL دستی بیرون packages/api-client
pnpm --filter @agent-one/ui run build          # پل Tailwind 4 (@theme)
pnpm --filter @agent-one/api-client run generate   # بازتولید schema.d.ts از گلدن
BASE_URL=http://127.0.0.1:8080 node scripts/round-trip-live.mjs   # در برابر سرور زنده
node scripts/e2e-money.mjs       # Step 7 — E2E پولی؛ باینری را خودش بوت می‌کند
npx @lhci/cli autorun --config=lighthouserc.cjs   # Lighthouse روی dist ترکیبی (محلی — R2)
python3 scripts/gen-icons.py      # بازتولید قطعی آیکون‌ها (Pillow)
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

## معیارهای پذیرش Step 4 — همه سبز (داشبورد ادمین)

1. **هشت بخش §2.5 روی کلاینت تولیدی** — `GET /overview` / اقتصاد / محصولات / سفارش‌ها / تأییدها + HITL / رشد / کنسول AI / پلتفرم؛ کلید ادمین فقط در مرورگر و راستی‌آزمایی‌شده با مسیر ادمین واقعی.
2. **صداقت پولی** — پیش‌نمایش ۹۰/۱۰ در هر ردیف آزادسازیِ در انتظار؛ mint/burn با کلید idempotency پایدار-به-ازای-اقدام (پاس‌گم‌شده → کلید حفظ می‌شود)؛ double-click با pending قفل می‌شود.
3. **static-root روی سرور واقعی** — `GET /admin/` → ۲۰۰ (base '/admin/')، manifest `admin:true` صادقانه.
4. **دولیتی و بودجه** — ۱۶۵ کلید تازه fa/en با پاریتی؛ initial 78.3 KB gzip زیر سقف ۲۰۰KB.

## معیارهای پذیرش Step 5 — همه سبز (اپ شهروندان — نقشهٔ ۳۳ صفحه)

1. **۳۳ صفحه §2.4 روی قرارداد منجمد** — ۳۲ مسیر هش‌روتر + 404 صادقانه؛ باتم‌ناو موبایل ۵گانه + کشوی «همهٔ صفحه‌ها»؛ همهٔ داده از کلاینت تولیدی (D5) — صفر تغییر بک‌اند.
2. **نردبان اقدام D14** — staff (کلید اختیاری) → مستقیم با کلید idempotency؛ شهروند تلگرام → مهارت‌های واقعی چت (create/deliver/dispute — آینهٔ رجیستری AI) یا فرمان دقیق بات (`/fund …` + لینک t.me)؛ مهمان → تحویل صادقانه به بات؛ هیچ دکمهٔ جعلی و هیچ دادهٔ جعلی (فهرست امانت/سفارش/صف HITL = نمای کارکنان؛ شهروند = جستجوی عمومی شناسه + دستیار).
3. **صداقت پولی** — پیش‌نمایش ۹۰/۱۰ همیشه پیش از آزادسازی؛ بج مادهٔ ۷ روی ≥ ۵۰٬۰۰۰ Đ؛ خط زمانی ۸ حالته آینهٔ `escrow_transitions`؛ رسید اقدام با بج replay.
4. **گفتگوی AI** — POST /ai/chat فقط با initData خام (تأیید HMAC سمت سرور)؛ تاریخچه فقط حافظهٔ صفحه؛ رد ابزارها با بج «نیاز به تأیید ادمین».
5. **static-root روی سرور واقعی** — `GET /app/` → ۲۰۰ (dir=rtl، base '/app/')، manifest `app:true` صادقانه، asset غایب → ۴۰۴ واقعی.
6. **دولیتی و بودجه** — ۱۶۷ کلید تازه fa/en با پاریتی؛ ۱۰ مادهٔ منشور عین `charter.rs`؛ webapp initial **84.7 KB gzip** و بدترین chunk **78.6 KB** زیر سقف ۱۲۰KB §6.

## معیارهای پذیرش Step 6 — همه سبز (PWA + Capacitor + Tauri skeletons)

1. **PWA نصب‌پذیر در static-root** — `manifest.webmanifest` (fa/rtl، scope `/`، standalone، توکن‌های Aurora)، آیکون‌های ۱۹۲/۵۱۲ در هر دو purpose + apple-touch ۱۸۰ + favicon svg/ico از مولد قطعی `scripts/gen-icons.py`؛ روی سرور واقعی: `/sw.js` → text/javascript، manifest → application/manifest+json، آیکون‌ها → ۲۰۰، asset غایب → ۴۰۴ واقعی.
2. **سرویس‌ورکر پولی-صادق** — cache نسخه‌دار `ao-shell-v1`؛ **`/api` هرگز intercept نمی‌شود** (guard پیش از respondWith، هیچ non-GET)؛ document ها network-first با fallback به کش ناحیه و سپس شلِ کش‌شده؛ asset هش‌دار cache-first؛ ثبت فقط در PROD با گارد تزریق‌پذیر (`pwa.ts`) — ۳۷ تست تازه، مجموع **۲۰۵ سبز**.
3. **اسکلت‌های موبایل/دسکتاپ بدون راز** — Capacitor: `com.dropagentx.app`، https scheme، `--base=./` بازسازی در sync-www، سندهای placeholder امضا/TestFlight (Step 9)؛ Tauri v2: frontendDist = static-root ترکیبی، targets msi/nsis/appimage/deb/dmg/app، capability فقط core:default، `[workspace]` مستقل، UPDATER.md به‌جای endpoint جعلی؛ **D16** (`VITE_API_BASE` تنها کلید مبدأ API بسته‌ها، از `optionsFromEnv`) و **D17–D19** ثبت شدند.
4. **بودجه‌ها** — داشبورد 83.6 ≤ 200 و webapp 84.8 ≤ 120؛ شل 80.7 KiB (۰٫۷ بالای سقف — رگرسیون دیکشنری مشترک Stepهای ۴/۵، ثبت صادقانه؛ ترمیم ناحیه‌ایِ دیکشنری به Step 7).
