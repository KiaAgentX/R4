# DEPLOY-RAILWAY — استقرار production DropAgentX v3 روی Railway

> پیوست پس از امضای v10/10 — این سند مسیر دقیق رساندن «نسخهٔ production» به
> Railway را گام‌به‌گام می‌دهد. تصویر production ریشه (`Dockerfile`) برخلاف
> `rust/Dockerfile`، **استاتیک ترکیبی فاز ۷ را هم داخل ایمیج می‌سازد** —
> شل `/`، داشبورد `/admin/` و اپ شهروندان `/app/` همه از یک کانتینر سرو می‌شوند.
>
> شواهد اعتبارسنجی همین بسته (بیلد release + ۱۲/۱۲ smoke + لودتست):
> `rust/docs/final/PRODUCTION-READINESS.md`

## ۰) پیش‌نیازها

- حساب Railway (پلن Hobby کافی است — یک سرویس + یک Volume + یک دامنه).
- ریپو روی GitHub (محتوای بستهٔ `Agent-One-Production-v10of10.zip` را push کنید — ریشهٔ ریپو = ریشهٔ بسته).
- بکاپ واقعی دیتابیس فعلی (Railway قدیمی یا سرور فعلی) برای مهاجرت — بخش ۴.

## ۱) متغیرهای محیطی

| متغیر | الزامی | مقدار پیشنهادی |
|---|:---:|---|
| `KIA_ADMIN_KEY` | ✅ | کلید قوی تصادفی (`openssl rand -hex 32`) |
| `DATABASE_URL` | ✅ | `file:/data/v3.db` (روی Volume) |
| `PORT` | — | پیش‌فرض ۳۰۰۰ (Railway خودش تزریق می‌کند؛ دست نزنید) |
| `STATIC_DIR` | — | از پیش `/app/dashboard` در ایمیج تنظیم شده |
| `RATE_LIMIT` | — | پیش‌فرض کد |
| `KEEP_BACKUPS` | — | مثلاً `7` |
| `TELEGRAM_BOT_TOKEN` + `BOT_WEBHOOK_SECRET` + `BOT_ADMIN_IDS` | — | برای بات؛ هر سه با هم |
| `OPENAI_BASE_URL` + `OPENAI_API_KEY` + `MODEL` | — | قاعدهٔ همه‌یا-هیچ ADR-006: ناقص = بوت رد می‌شود |

> fail-closed: بدون `KIA_ADMIN_KEY` یا `DATABASE_URL` سرور بالا نمی‌آید — این عمدی است.

## ۲) مسیر A — GitHub → Railway (توصیه‌شده)

1. **New Project → Deploy from GitHub repo** → ریپو را انتخاب کنید.
2. Railway با وجود `railway.toml` خودش تشخیص می‌دهد: builder = `DOCKERFILE`، مسیر = `Dockerfile` ریشه. **Root Directory را دست نزنید** (ریشهٔ ریپو).
3. صبر برای بیلد (~۵–۸ دقیقه؛ stage رست ~۴ دقیقه، stage استاتیک ~۲ دقیقه).
4. **Variables** → مقادیر جدول بالا را set کنید → سرویس ری‌دپلوی می‌شود.
5. **Volume**: سرویس → `Settings → Volumes → Create` → mount path = **`/data`** — SQLite بدون Volume با هر ری‌دپلوی پاک می‌شود؛ این گام حیاتی است.
6. **Domain**: `Settings → Networking → Generate Domain` — آدرس `*.up.railway.app` آمادهٔ Telegram Mini App و داشبورد است.

## ۳) مسیر B — Railway CLI

```bash
npm i -g @railway/cli
railway login
railway init                      # در ریشهٔ ریپو
railway up                        # بیلد و دیپلوی از Dockerfile ریشه
railway volume add --mount-path /data
railway variables --set "KIA_ADMIN_KEY=$(openssl rand -hex 32)" \
                       --set "DATABASE_URL=file:/data/v3.db" \
                       --set "KEEP_BACKUPS=7"
railway domain                    # دامنهٔ عمومی
```

## ۴) مهاجرت دادهٔ واقعی (یک‌بار، پیش از بازکردن ترافیک)

```bash
# روی سیستم محلی — بکاپ واقعی را به فیکسچر استاندارد تبدیل نکنید؛ مستقیم:
docker run --rm -v "$PWD":/work -w /work dropagentx-builder \
  migrate-legacy --legacy /work/marketplace-<تاریخ>.db \
                 --target file:/work/v3.db
#  خروجی موردانتظار: Σ user wallets = 187672 Đ · trialBalance.balanced = true
audit-ledger v3.db          # → AUDIT PASS (exit 0) — در غیر این صورت ترافیک باز نشود
```

سپس `v3.db` را روی Volume بگذارید (سرویس را stop → فایل را با `railway run`/پنل جای‌گذاری → start).
مهاجرت **replay-safe** است (`legacy:genesis:v1`) — اجرای دوباره صفر تغییر می‌دهد.

## ۵) چک سلامت پس از دیپلوی (DEPLOY.md §2 — به‌ترتیب)

```bash
BASE=https://<app>.up.railway.app
curl -s $BASE/api/v1/health   # "status":"ok" && "db":"ok"
curl -s $BASE/api/v1/economy  # "balanced":true && "trialBalanceSum":0
curl -s $BASE/api/v1/openapi.json > new.json   # diff صفر نسبت به تگ rust-core-1.0.0
curl -s -H "X-KIA-Key: $KIA_ADMIN_KEY" $BASE/api/v1/overview | head -c 200
# استاتیک: $BASE/ (شل) · $BASE/admin/ (داشبورد) · $BASE/app/ (مینی‌اپ)
```

## ۶) به‌روزرسانی و برگشت

- **آپدیت**: push به برنچ پیش‌فرض → Railway خودکار می‌سازد؛ یا `railway up`.
- **برگشت اپلیکیشن**: Railway → Deployments → نسخهٔ قبلی → `Redeploy` (چون SQLite روی Volume است، برگشت کد = بدون مهاجرت داده).
- **برگشت گذار dual-run**: طبق `rust/DEPLOY.md` §4 — `POST /_dualrun/rollback` یا `RUST_TRAFFIC_PCT=0`؛ هر دو موتور روی همان فایل SQLite کار می‌کنند و هیچ مهاجرت برگشتی لازم نیست.
- **بکاپ دوره‌ای**: `POST /api/v1/system/backup` (ادمین) → `VACUUM INTO` روی `db/backups/` با هرس `KEEP_BACKUPS` — و دانلود دوره‌ای از Volume (Railway snapshot جایگزین بکاپ نیست).

## ۷) شفافیت

دیپلوی واقعی روی Railway نیازمند حساب/توکن مالک است و از سندباکس انجام نمی‌شود؛
اما همهٔ مصنوعات لازم (Dockerfile production با استاتیک، railway.toml، سند،
ابزار مهاجرت و ممیزی) **داخل همین بسته** آماده و با بیلد release محلی
اعتبارسنجی شده‌اند: ۱۲/۱۲ smoke سبز (شامل استاتیک ترکیبی، گیت ادمین، اقتصاد
زندهٔ ۱۸۷٬۶۷۲ Đ و AUDIT PASS) + لودتست p95 = ۰٫۲ms با RSS = ۱۰MB.
