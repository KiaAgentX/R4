# آپدیت خودکار — placeholder (فاز ۷، Step 6؛ فعال‌سازی در Step 9)

طبق بخش ۹ ممیزی، Phase 7 «تنظیمات Tauri v2 + placeholder آپدیت خودکار» را
می‌دهد؛ خودِ آپدیت‌کننده به امضای کد نیاز دارد و در Step 9 فعال می‌شود.

## آنچه الان هست

- `tauri.conf.json` بدون `createUpdaterArtifacts` و بدون `updater` endpoint —
  کانفیگ عمداً buildable است و به کلید امضا وابسته نیست.
- `bundle.targets` پوشش کامل: Windows (`msi`, `nsis`) · Linux (`appimage`,
  `deb`) · macOS (`dmg`, `app`).

## فعال‌سازی در Step 9 (چک‌لیست صادقانه)

1. تولید کلید امضا (یک‌بار، بیرون ریپو):
   ```bash
   npx @tauri-apps/cli signer generate -w ~/.tauri/dropagentx.key
   ```
   کلید عمومی → env `TAURI_SIGNING_PUBLIC_KEY`؛ کلید خصوصی هرگز commit نمی‌شود.
2. در `tauri.conf.json`:
   - `"bundle": { "createUpdaterArtifacts": true }`
   - `"plugins": { "updater": { "endpoints": ["https://<origin>/desktop/latest.json"], "pubkey": "<TAURI_SIGNING_PUBLIC_KEY>" } }`
   - وابستگی `tauri-plugin-updater = "2"` به Cargo.toml + مجوزش در
     `capabilities/default.json`.
3. سرور استاتیک ما فقط `ServeDir` است — فایل `latest.json` + artifactهای امضا‌شده
   را همان‌جا می‌گذاریم (بدون تغییر بک‌اند؛ مسیر CDI خودت را در Step 9 قطعی کن).
4. Windows signing (`certificateThumbprint`) و macOS notarization
   (` APPLE_ID`/`APPLE_TEAM_ID`) — هر دو فقط با گواهی واقعی؛ placeholder همین سند.

## چرا الان فعال نشد؟

آپدیت‌کننده بدون کلید = صرفاً endpoint جعلی. اسکلت باید بدون هیچ راز قابل
build باشد؛ هر ادعای نیمه‌کاره اینجا جعل بود، نه مهندسی.
