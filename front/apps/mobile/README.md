# @agent-one/mobile — اسکلت Capacitor (فاز ۷، Step 6)

پیکربندی و ابزارِ sync فقط — بدون بازنویسی React Native (بخش ۷ ممیزی). پروژه‌های
native، keystore و provisioning عمداً به Step 9 موکول شده‌اند (بخش ۹: «امضا»).

## چیست

| فایل | نقش |
|---|---|
| `capacitor.config.ts` | `com.dropagentx.app` · webDir `www` · scheme https · رنگ تیرهٔ Aurora · `allowMixedContent=false` |
| `capacitor-config.types.ts` | آینهٔ محلی تایپ `CapacitorConfig` — بعد از نصب SDK با تایپ واقعی `@capacitor/cli` عوضش کنید |
| `scripts/sync-www.mjs` | بازسازی webapp با `--base=./` (مسیر asset در WebView) → کپی به `www/` |
| `splash/splash-{2732,2048,1284}.png` | منبع اسپلش Aurora (Đ + وردمارک) برای `@capacitor/assets` |
| `android/README-signing.md` | سند placeholder امضای اندروید (Step 9) |
| `ios/README-testflight.md` | سند placeholder TestFlight/iOS + safe-area (Step 9) |

## چرا sync کپیِ کور نیست؟

داشبورد/webapp برای static-root با base مطلق `/app/` بیلد می‌شوند (D3/D4).
WebView کاپسیتور از `https://localhost` سرو می‌کند؛ آن پیشوند مطلق آنجا ۴۰۴
می‌شود. بنابراین `sync-www.mjs` با `vite build --base=./` **بازسازی** می‌کند تا
مسیرها relative شوند. هش‌روتر هم یعنی هیچ SPA-fallback سروری لازم نیست.

## اجرا (روی ماشین مالک — SDK اینجا نصب نیست)

```bash
# یک‌بار — نصب SDK (خارج از Step 6، عمداً)
corepack pnpm --filter @agent-one/mobile add @capacitor/core
corepack pnpm --filter @agent-one/mobile add -D @capacitor/cli @capacitor/android @capacitor/ios

# مبدأ API + پر کردن www/
VITE_API_BASE=https://api.example.example corepack pnpm --filter @agent-one/mobile run sync:www

# پروژه‌های native + همگام‌سازی
corepack pnpm --filter @agent-one/mobile exec cap add android
corepack pnpm --filter @agent-one/mobile exec cap add ios
corepack pnpm --filter @agent-one/mobile exec cap sync
corepack pnpm --filter @agent-one/mobile exec cap open android   # یا ios
```

پس از `cap add`، تایپ کانفیگ را به `import type { CapacitorConfig } from '@capacitor/cli'`
سوئیچ کنید و `capacitor-config.types.ts` را حذف کنید (شکل فیلدها یکسان است).

## اصل دست‌نخورده

پول حقیقتش سمت سرور است — WebView فقط همان webapp قرارداد-محور را بار می‌کند؛
هویت تلگرام در WebView بیرونی وجود ندارد و کلاینت هم هویت جعل نمی‌کند (D14).
`www/` و پروژه‌های native هرگز commit نمی‌شوند (`.gitignore` همین پوشه).
