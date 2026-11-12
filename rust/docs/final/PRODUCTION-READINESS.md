# آمادگی production — اعتبارسنجی بستهٔ Railway (پس از امضای v10/10)

> تاریخ: ۲۱ آبان ۱۴۰۵ — ۱۲ نوامبر ۲۰۲۶ · باینری: `target/release` (بهینه، LTO thin، strip)
> دیتابیس: کپی `docs/phase8/cutover-v3.db` (همان کاندیدای cut-over ممیزی‌شده)

## ۱) بیلد release

```text
cargo build --release -p dropagentx-server -p dropagentx-tooling
  Finished `release` profile [optimized] target(s) in 6m 42s
```

## ۲) Smoke ۱۲/۱۲ — سرور release با env فارغالتحصیل production

| # | بررسی | نتیجه |
|:---:|---|:---:|
| 1 | `GET /api/v1/health` — status ok + db ok | ✅ 200 |
| 2 | `GET /api/v1/economy` — balanced | ✅ 200 |
| 3 | `GET /api/v1/openapi.json` | ✅ 200 |
| 4 | `GET /api/v1/docs` | ✅ 200 |
| 5 | `GET /` — شل راهبردی (static-root) | ✅ 200 |
| 6 | `GET /admin/` — داشبورد ادمین | ✅ 200 |
| 7 | `GET /app/` — اپ شهروندان | ✅ 200 |
| 8 | `GET /build-manifest.json` — صادقانه (shell+admin+app) | ✅ 200 |
| 9 | asset استاتیک شل | ✅ 200 |
| 10 | گیت ادمین: `overview` بدون کلید | ✅ 401 |
| 11 | گیت ادمین: `overview` با `X-KIA-Key` | ✅ 200 |
| 12 | ممیزی مستقل روی دیتابیس همین بوت | ✅ AUDIT PASS — Σ = ۱۸۷٬۶۷۲ Đ · balanced=true |

اقتصاد زنده: `circulatingSupply=187672 · balanced=true · accountCount=469 · genesisSupply=187672`

## ۳) لودتست روی باینری release (۱۰ ثانیه × ۲، هم‌زمانی ۸)

| مسیر | ok | خطا | p95 | منبع |
|---|---:|---:|---:|---|
| `/api/v1/health` | 564,069 | 0 | **0.2 ms** | `docs/final/loadtest-health.json` |
| `/api/v1/economy` | 565,541 | 0 | **0.2 ms** | `docs/final/loadtest-economy.json` |

**RSS پروسهٔ سرور = 10 MB** (هدف ADR-009: ~۲۰MB — TS: ~۱۲۰–۱۵۰MB).
یادداشت: اعداد فاز ۸ (p95≈57ms) روی ایمیج dev سندباکس ثبت شده بود؛ این اجرا
روی باینری release بهینه است — هر دو در `memory.txt`/همین سند شفاف‌اند.

## ۴) بودجهٔ استاتیک (بیلد تازهٔ امروز، static-root ترکیبی)

| اپ | بودجه اولیه | سقف §6 |
|---|---:|---:|
| shell | 71.3 KB gzip | ≤ 80 KB |
| dashboard | 78.0 KB gzip | ≤ 200 KB |
| webapp | 82.5 KB gzip | ≤ 120 KB |

manifest: `{"shell":true,"admin":true,"app":true,"site":false}`

## ۵) مصنوعات استقرار Railway — همه داخل بسته

| فایل | نقش |
|---|---|
| `Dockerfile` (ریشه) | تصویر production سه‌مرحله‌ای: استاتیک (node:24 + pnpm + build:root) → رست (rust:1.99-slim) → distroless cc nonroot — استاتیک ترکیبی داخل ایمیج |
| `.dockerignore` (ریشه) | context تمیز (بدون target/node_modules/dist/.git) |
| `railway.toml` | builder=DOCKERFILE · healthcheck=`/api/v1/health` · restart ON_FAILURE |
| `DEPLOY-RAILWAY.md` | راهنمای گام‌به‌گام فارسی: دو مسیر (GitHub/CLI)، Volume `/data`، مهاجرت واقعی، چک ۴ گامی، برگشت |

## ۶) آنچه در سندباکس انجام نشد (شفافیت)

- `docker build` محلی — داکردایمون در سندباکس نیست؛ بیلد ایمیج روی Railway انجام می‌شود (همان Dockerfile).
- دیپلوی/تست روی حساب واقعی Railway — نیازمند توکن مالک؛ مسیر کامل در `DEPLOY-RAILWAY.md`.
- به‌جای آن، **همان نقطهٔ ورود production** (باینری release + env الزامی + استاتیک ترکیبی + دیتابیس cut-over) در سندباکس کامل اجرا و ۱۲/۱۲ سبز شد.
