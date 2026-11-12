# Android — امضا و انتشار (placeholder فاز ۷، Step 9 انجام می‌شود)

> این سند همان «signing placeholder doc» است که Step 6 (بخش ۹ ممیزی) می‌خواهد.
> خودِ امضا و انتشار در محیط بدون Android Studio انجام نمی‌شود — صادقانه به
> Step 9 سپرده شده است.

## آنچه Step 6 تحویل داد

- `capacitor.config.ts` — `com.dropagentx.app`، webDir `www`، scheme https،
  رنگ‌های Aurora تیره؛ `allowMixedContent=false` (اپ پولی — بدون downgrade).
- `scripts/sync-www.mjs` — بازسازی webapp با `--base=./` و پرکردن `www/`.
- آیکون: `front/apps/shell/public/icons/*` و اسپلش: `front/apps/mobile/splash/splash-{2732,2048,1284}.png`
  (همان منبع Aurora؛ پس از نصب SDK با `npx @capacitor/assets generate` به رزولوشن‌های اندروید تولید شود).

## آنچه به Step 9 موکول است (چک‌لیست)

1. **keystore** — تولید release keystore بیرون ریپو (هرگز commit نشود):
   ```bash
   keytool -genkey -v -keystore dropagentx-release.keystore -alias dropagentx \
           -keyalg RSA -keysize 2048 -validity 10000
   ```
2. `android/variables.gradle` + `keystore.properties` محلی؛ `signingConfig`
   release فقط از همان فایل محلی.
3. `npx cap add android` پس از نصب SDK؛ `npx cap sync android`.
4. آیکون‌ها/اسپلش با `@capacitor/assets` از همان منبع Aurora.
5. Play Console — bundle signing، privacy policy، data-safety.

## خواندن API

WebView به مبدأ مطلق سرور نیاز دارد: پیش از `sync:www` متغیر
`VITE_API_BASE=https://<origin>` را ست کنید (داخل کلاینت تولیدی از طریق
`optionsFromEnv` می‌نشیند — هیچ URL دستی نوشته نمی‌شود، D5).
