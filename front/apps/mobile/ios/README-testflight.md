# iOS — TestFlight و انتشار (placeholder فاز ۷، Step 9 انجام می‌شود)

> «TestFlight notes doc»ی که Step 6 (بخش ۹ ممیزی) می‌خواهد. Provisioning و
> امضا بدون Apple Developer در این محیط ممکن نیست — صادقانه به Step 9 سپرده
> شده است.

## آنچه Step 6 تحویل داد

- `capacitor.config.ts` — `com.dropagentx.app`، `contentInset: 'always'`؛
  safe-area داخل خود webapp مدیریت می‌شود (`viewport-fit=cover` +
  `env(safe-area-inset-*)` در `.aur-bottomnav` و padding انتهای صفحه) —
  وب‌ویو اینست اضافه نمی‌گذارد تا دوباره‌کاری نشود.
- apple-touch-icon ۱۸۰×۱۸۰ در static-root (`/icons/apple-touch-icon.png`) —
  همان منبع Aurora، بدون شفافیت (iOS خودش گوشه‌ها را ماسک می‌کند).
- `scripts/sync-www.mjs` — webapp با `--base=./` داخل `www/`.

## آنچه به Step 9 موکول است (چک‌لیست)

1. Apple Developer Program + App ID `com.dropagentx.app` با Associated Domains
   (اگر universal link به بات لازم شد).
2. `npx cap add ios` پس از نصب Xcode؛ `npx cap sync ios`.
3. آیکون AppIcon.appiconset و اسپلش از همان منبع Aurora (`@capacitor/assets`؛ منبع اسپلش:
   `front/apps/mobile/splash/splash-{2732,2048,1284}.png`).
4. Archive با distribution certificate + provisioning profile؛ upload با
   Transporter یا `xcrun altool`.
5. TestFlight: گروه داخلی، ملاحظات review، این‌نِبِل بودن مودال پولی برای
   App Store Review (امانت‌گذاری روی سرور خودمان — خرید in-app نیست؛ برای
   تصمیم نهایی مالک در Step 9).

## خواندن API

پیش از `sync:www`: `VITE_API_BASE=https://<origin>` (کلاینت تولیدی +
`optionsFromEnv` — بدون URL دستی، D5). ATS: فقط https با TLS معتبر.
