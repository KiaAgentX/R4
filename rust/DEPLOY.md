# DEPLOY — DropAgentX v3 (هستهٔ Rust، تک‌باینری)

> فاز ۸ ADR-009 — به‌روزرسانی مستند دیپلوی پس از cut-over. معماری دیپلوی
> تغییر نکرده (تک‌پروسه · SQLite فایل‌محور · distroless)؛ چیزِ جدید این فاز،
> **پراکسی dual-run** و **دروازهٔ ممیزی** است که بخش ۴ را به‌روزرسانی کرده‌اند.

## ۱) دیپلوی استاندارد (Docker/Railway)

> **برای production کامل (بک‌اند + استاتیک ترکیبی فاز ۷ در یک ایمیج) و راهنمای
> گام‌به‌گام Railway، از ریشهٔ ریپو استفاده کنید: `Dockerfile` + `railway.toml`
> + `DEPLOY-RAILWAY.md` — شواهد اعتبارسنجی: `docs/final/PRODUCTION-READINESS.md`.**
> Dockerfile زیر (rust/) فقط باینری سرور را می‌سازد و برای دیپلوی بدون فرانت
> یا بیلدهای dev مناسب است.

```bash
docker build -t dropagentx-server .
docker run -d --name v3 \
  -p 3000:3000 \
  -e KIA_ADMIN_KEY="$KIA_ADMIN_KEY" \
  -e DATABASE_URL=file:/data/v3.db \
  -v agent-data:/data \
  dropagentx-server
```

* تصویر: `rust:1.99-slim` (builder) → `gcr.io/distroless/cc-debian12:nonroot`
  (runtime) — تک‌باینری، کاربر غیر-root، بدون شل.
* مهاجرت‌های sqlx در کامپایل جاسازی‌اند؛ بوت خودش اسکیما را نسخه‌دار اجرا
  می‌کند — مرحلهٔ «migrate» جدا وجود ندارد و لازم هم نیست.
* `STATIC_DIR` (پیش‌فرض `front/dashboard/dist`) دارایی‌های داشبورد/Mini App
  فاز ۷ را سرو می‌کند.

## ۲) چک سلامت پس از دیپلوی (به‌ترتیب)

1. `GET /api/v1/health` → `"status":"ok" && "db":"ok"`
2. `GET /api/v1/economy` → `"balanced":true && "trialBalanceSum":0`
3. `GET /api/v1/openapi.json` → diff صفر نسبت به تگ `rust-core-1.0.0`
4. ممیزی دوره‌ای: `audit-ledger /data/v3.db` → `AUDIT PASS` (exit 0)

## ۳) متغیرهای محیطی (خلاصه — شرح کامل در RUNBOOK §۰)

`KIA_ADMIN_KEY` (الزامی) · `DATABASE_URL` (الزامی) · `PORT` · `RATE_LIMIT` ·
`STATIC_DIR` · `KEEP_BACKUPS` · `BRAND_NAME`/`BRAND_TAGLINE` ·
`TELEGRAM_BOT_TOKEN`+`BOT_WEBHOOK_SECRET`+`BOT_ADMIN_IDS` ·
`OPENAI_BASE_URL`+`OPENAI_API_KEY`+`MODEL` (سه‌تایی همه‌یا-هیچ).

## ۴) گذار dual-run روی production (فاز ۸ — ADR-009 §۱۲)

هر دو موتور روی **همان** فایل SQLite، پشت یک پراکسی؛ تک‌متغیرِ گذار
`RUST_TRAFFIC_PCT`:

```bash
# 1) Rust را کنار TS بالا بیاورید (بوت مستقل، دیتابیس همان فایل)
docker run -d --name v3-rust -p 3002:3000 … dropagentx-server

# 2) پراکسی — سایه (پاسخ فقط از TS، Rust ثبت/مقایسه)
TS_URL=http://127.0.0.1:3000 RUST_URL=http://127.0.0.1:3002 \
  RUST_TRAFFIC_PCT=0 SHADOW=1 PORT=3001 ./dual-run

# 3) ۷ روز parity سبز → ارتقای پله: 1 → 10 → 50 → 100 (هر پله: 48h + parity + p95)
curl -X POST http://127.0.0.1:3001/_dualrun/pct -d '{"pct":1}'

# 4) برگشت اضطراری — هر لحظه، تک‌فرمان، بدون مهاجرت داده
curl -X POST http://127.0.0.1:3001/_dualrun/rollback
```

* گیت هر پله: `parity --verify` (43/43) + `parity --ts-url http://…:3000`
  (مقایسهٔ زندهٔ TS↔Rust) + `loadtest` (p95 ≤ 2×baseline).
* سپر انسانی: مسیرهای ≥۵۰,۰۰۰ Đ (`/hitl*`، `/escrows/{id}/release|refund`) تا
  `STABILITY_DAYS=3` حتی در ۱۰۰٪ به TS می‌روند — پراکسی خودش اعمال می‌کند.
* تثبیت: ۲ هفتهٔ پیوستهٔ ۱۰۰٪ سبز → تگ `rust-core-1.0.0` → خاموشی Node در
  production؛ TS در همان تگ آخری گرم‌خانه می‌ماند.

## ۵) بکاپ / بازیابی / ممیزی

* بکاپ سازگار: `POST /api/v1/system/backup` (ادمین) — `VACUUM INTO` به
  `db/backups/`، هرس تا `KEEP_BACKUPS` نسخه.
* بازیابی: توقف پروسه → جای‌گذاری فایل بکاپ → بوت → `audit-ledger` →
  فقط بعد از `AUDIT PASS` ترافیک.
* شرح عملیاتی کامل (sweep، alerting، panic-free policy): `docs/phase8/RUNBOOK.md`.

## ۶) اهداف ثبت‌شدهٔ فاز ۸ (سندباکس، بیلد release)

| متریک | مقدار | هدف ADR |
|---|---|---|
| RSS پروسه | **12 MB** | ~20MB (TS: ~120–150MB) |
| استارت سرد تا `/health` سبز | **10.7 ms** | <100ms |
| p95 (`/health`, `/economy`) | ≈ 57ms | ≤ 2× TS baseline روی همان سخت‌افزار |
