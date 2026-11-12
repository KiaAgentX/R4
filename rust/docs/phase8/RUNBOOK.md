# RUNBOOK عملیاتی — DropAgentX v3 (هستهٔ Rust)

> فاز ۸ ADR-009 — «Runbook عملیاتی: بوت، sweep، بکاپ، panic-free policy،
> alerting؛ آموزش سوییچ تک‌متغیره.» این سند همان چیزی است که اپراتور روی
>production می‌خواند؛ فرض‌ها: تک‌باینری `dropagentx-server`، SQLite فایل‌محور،
> دیپلوی Railway/Docker (`Dockerfile` ورک‌اسپیس).

---

## ۰) متغیرهای محیطی ضروری (fail-closed)

| متغیر | الزام | نقش |
|---|---|---|
| `KIA_ADMIN_KEY` | الزامی (≥۱۶ کاراکتر) | کلید ادمین `X-KIA-Key` — نبودنش = بوت رد |
| `DATABASE_URL` | الزامی | `file:/data/v3.db` — فایل دیتابیس نسخه‌دار |
| `PORT` | پیش‌فرض 3000 | پورت HTTP |
| `RATE_LIMIT` | پیش‌فرض 120 | درخواست/دقیقه بر کلید (پنجرهٔ لغزان) |
| `BRAND_NAME` / `BRAND_TAGLINE` | اختیاری | شل وایت‌لیبل داشبورد |
| `TELEGRAM_BOT_TOKEN` + `BOT_WEBHOOK_SECRET` + `BOT_ADMIN_IDS` | اختیاری | بات؛ بدون توکن، webhook با 503 «disabled» پاسخ می‌دهد (هیچ‌وقت 500) |
| `OPENAI_BASE_URL` + `OPENAI_API_KEY` + `MODEL` | اختیاری | سه‌تایی AI — همه‌یا-هیچ؛ ناقص = 503 fail-closed |

قاعدهٔ طلایی: **پیکربندی ناقص هرگز panic نمی‌کند؛ مسیر را با پاکت خطای
تایپ‌دار 503 رد می‌کند.**

## ۱) بوت

```bash
# تصویر یا باینری — تک‌پروسه، استارت سرد اندازه‌گیری‌شده: ~11ms
KIA_ADMIN_KEY=… DATABASE_URL=file:/data/v3.db PORT=3000 ./dropagentx-server
```

چک‌لیست پس از بوت (به‌ترتیب):

1. `GET /api/v1/health` → `{"status":"ok","db":"ok",…}` — اگر `db` غیر `ok` است، دیتابیس را audit کنید (§۴).
2. `GET /api/v1/economy` → `balanced: true` و `trialBalanceSum: 0` — اگر نه، **فوراً rollback** (§۶) و ممیزی (§۴).
3. `GET /api/v1/openapi.json` → همان OpenAPI 3.1 تگ `rust-core-1.0.0` (diff صفر).
4. لاگ JSON روی stdout — هر خط `WARN/ERROR` باید قابل‌توضیح باشد؛ لاگ بی‌صدا = سلامت.

## ۲) sweep — کارهای دوره‌ای (بوت یا کرون بیرونی)

| کار | فراخوانی | توضیح |
|---|---|---|
| انقضای HITL overdue | `POST /api/v1/hitl/sweep` (ادمین) | درخواست‌های PENDING گذشته از TTL ۲۴ساعت → `EXPIRED`؛ بات/کرون هر ۵ دقیقه صدا بزند |
| انقضای اسکرو TTL | `POST /api/v1/escrows/{id}/expire` (ادمین) | مادهٔ ۹ منشور — اسکرو FUNDED گذشته از TTL → بازپرداخت خودکار |
| بکاپ سازگار | `POST /api/v1/system/backup` (ادمین) | `VACUUM INTO` → `db/backups/`؛ هرگز روی دیتابیس زنده copy نبرید |

هر سه idempotent‌اند — صدا زدن دوباره، خروجی دوباره نمی‌سازد.

## ۳) بکاپ و بازیابی

* **گرفتن:** `POST /api/v1/system/backup` — اسنپ‌شات سازگار SQLite؛ نگهداری
  `KEEP_BACKUPS` نسخهٔ آخر (بقیه هرس می‌شوند).
* **بازیابی:** پروسه را بندید → فایل بکاپ را جای `v3.db` بگذارید → بوت (§۱) →
  `audit-ledger` (§۴) → فقط بعد از `AUDIT PASS` ترافیک را بازگردانید.
* **آزمون ماهانه:** بازیابی را روی محیط کپی تمرین کنید؛ بکاپِ تست‌نشده بکاپ نیست.

## ۴) ممیزی مالی — باینری `audit-ledger`

```bash
audit-ledger /data/v3.db
```

دو شرط پذیرش ADR-009 را چاپ می‌کند: `Σ کیف‌ها = 187,672 Đ` و
`trialBalance.balanced == true` (Σ پستینگ‌ها = ۰). خروجی‌ها:

* `AUDIT PASS` (exit 0) — ادامهٔ عملیات عادی.
* `AUDIT FAILED` (exit 1) — **هرگز با مغایرت مالی سوییچ/ادامه ندهید**؛
  `RUST_TRAFFIC_PCT=0`، ترافیک به TS، بکاپ آخرین وضعیت سبز را بازیابی کنید و
  از گزارش `parity --verify` برای پیدا کردن نقطهٔ واگرایی استفاده کنید.

## ۵) panic-free policy — چرا و چگونه

* ورک‌اسپیس `#![forbid(unsafe_code)]` سراسری + deny `unwrap/expect` خارج از
  تست دارد؛ مرز HTTP هیچ panic پخش‌شده‌ای ندارد — هر خطای دامنه پاکت
  `{error:{code,message}}` تایپ‌دار است.
* **سیاست عملیاتی:** panic در لاگ = حادثهٔ P1 حتی اگر درخواست 200 برگردانده
  باشد. فرایند: لاگ را ببرید به `cargo run -p dropagentx-tooling --bin parity`
  و `fuzz_lite` را روی همان ورودی بازاجرا کنید؛ اگر بازتولید نشد، memory
  dump/ورودی خام را پیوست issue کنید.
* در dual-run، آپ‌استریم مرده هرگز اپراتور را غافلگیر نمی‌کند: پراکسی 502
  `UPSTREAM_DOWN` fail-closed می‌دهد (تست‌شده در `dual_run_proxy.rs`).

## ۶) alerting — چه چیزی را هشدار بدهیم

| سیگنال | آستانه | اقدام |
|---|---|---|
| `/health` ناموفق | ۳ بار پشت‌سرهم | ری‌استارت + بازبینی لاگ |
| `economy.balanced` ≠ true | هر بار | **P0** — rollback + audit-ledger |
| p95 هر مسیر | > 2× baseline ثبت‌شده (LOADTEST این فاز) | بازبینی؛ در dual-run یعنی پله بعدی ممنوع |
| 5xx rate | > 0.1٪ در ۵ دقیقه | بازبینی لاگ؛ parity مجدد |
| `parityDiff` شمارندهٔ پراکسی | > 0 در پنجرهٔ ۲۴ساعت | **P0** — rollback؛ گزارش parity ضمیمهٔ incident |
| دیسک `db/backups` | > ۸۰٪ | نگهداری/هرس بکاپ‌ها |
| مصرف RAM پروسه | > 100 MB (هدف ADR: ~20MB) | بررسی نشت؛ مقایسه با baseline |

## ۷) آموزش سوییچ تک‌متغیره — dual-run (ADR-009 §۱۲)

تک‌متغیرِ کل گذار **فقط** `RUST_TRAFFIC_PCT` است؛ هیچ چیز دیگری بین موتورها
مشترک تغییر نمی‌کند (هر دو روی **همان** SQLite با همان کلیدهای idempotency).

```bash
# پراکسی را کنار هر دو سرور بالا بیاورید (بوت با کانفیگ متناقض رد می‌شود)
TS_URL=http://127.0.0.1:3000 RUST_URL=http://127.0.0.1:3002 \
RUST_TRAFFIC_PCT=0 SHADOW=1 PORT=3001 ./dual-run

# وضعیت / برنامهٔ پله‌ها / شمارنده‌ها
curl http://127.0.0.1:3001/_dualrun/status

# ارتقای پله — فقط بعد از سبزی گیت parity همان پله
curl -X POST http://127.0.0.1:3001/_dualrun/pct -d '{"pct":1}'
# … 48 ساعت پایداری + parity + p95 … سپس 10 → 50 → 100

# برگشت اضطراری — هر لحظه، تک‌فرمان، بدون مهاجرت برگشتیِ داده
curl -X POST http://127.0.0.1:3001/_dualrun/rollback
```

**گیت هر پله** (همه سبز وگرنه نمان سر همان پله یا برگشت):

```bash
cargo run -p dropagentx-tooling --bin parity -- --verify          # 43/43 طلایی
cargo run -p dropagentx-tooling --bin parity -- --ts-url http://127.0.0.1:3000  # مقایسهٔ زندهٔ TS↔Rust
./loadtest --url http://127.0.0.1:3002/api/v1/health --duration 10  # p95 ≤ 2× baseline
```

**سپر انسانی (مادهٔ ۷):** تراکنش‌های ≥ ۵۰٬۰۰۰ Đ (مسیرهای `/hitl*` و
`/escrows/{id}/release|refund`) تا `STABILITY_DAYS=3` حتی در ۱۰۰٪ هم به TS
می‌روند — «آخرین لایه‌ای که سوییچ می‌شود، اولین چیزی که آدیت می‌شود.» پراکسی
این را خودش اعمال می‌کند (`hitl_shielded`)؛ اپراتور فقط `STABILITY_DAYS` را
درست نگه می‌دارد و بعد از ۳ روز پایداری کامل افزایش می‌دهد.

**دامنهٔ مجاز rollback:** هر parity-fail، هر discrepancy مالی، p95 بدتر از
2×baseline، هر panic تکرارشونده. بعد از rollback: parity مجدد، علت‌یابی، و
فقط بعد از ریشه‌کنی، از shadow دوباره شروع کنید.

## ۸) تثبیت و بستن dual-run

2 هفتهٔ پیوستهٔ ۱۰۰٪ سبز (صفر parity-fail، صفر rollback) →

1. `git tag rust-core-1.0.0` — تگ منتشر شود.
2. TS در همان تگ آخری به‌عنوان گرم‌خانه باقی می‌ماند (Node خاموش در production؛
   آرشیو runtime در artifact store).
3. README، DEPLOY.md و این RUNBOOK به‌روزرسانی و ضمیمهٔ تحویل می‌شوند.
