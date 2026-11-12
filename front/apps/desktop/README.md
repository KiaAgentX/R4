# @agent-one/desktop — اسکلت Tauri v2 (فاز ۷، Step 6)

جایگزین Electron (تصمیم D2): Rust-native، ~۱۰MB، همان روایت ایمنی حافظهٔ
بک‌اند. طبق بخش ۹ ممیزی، Step 6 فقط «کانفیگ» می‌دهد؛ امضای کد و notarization
به Step 9 موکول است.

## معماری — یک آرتیفکت، همه‌جا (D3)

پنجره همان static-root ترکیبی (`front/dist`) را در WebView سیستم بار می‌کند —
همان آرتیفکتی که ServeDir سرور و PWA سرو می‌کنند:

```
src-tauri/
├── tauri.conf.json      # frontendDist ← ../../../dist (front/dist ترکیبی)
├── Cargo.toml           # [workspace] مستقل — نه rust/ منجمد، نه ورک‌اسپیس کهنهٔ ریشهٔ ریپو
├── build.rs             # tauri_build::build()
├── src/main.rs          # Builder پیش‌فرض — هیچ فرمان سفارشی (viewer خالص، fail-closed)
├── capabilities/default.json   # فقط core:default
├── icons/               # 32/128/256/512 از همان مولد آیکون Aurora
└── docs/UPDATER.md      # placeholder آپدیت خودکار (Step 9)
```

## اجرا (روی ماشین مالک — کریت‌ها از crates.io)

```bash
corepack pnpm install                     # یک‌بار در front/
corepack pnpm --filter @agent-one/desktop add -D @tauri-apps/cli
# mdev اولیه — رزولوشن مسیرها را همین‌جا می‌بینی:
corepack pnpm --filter @agent-one/desktop exec tauri build
```

- `beforeBuildCommand` = `pnpm -C ../.. build:root` (از CWD اجرای CLI یعنی
  `apps/desktop` دو سطح بالا می‌رود به `front/`). اگر نسخهٔ CLI تو hookها را
  از CWD متفاوتی اجرا کرد، کافی است پیش از build یک‌بار
  `pnpm -C front build:root` بزنی — کانفیگ بدون hook هم معتبر است.
- `frontendDist: "../../../dist"` نسبی به پوشهٔ `src-tauri` است (قرارداد v2،
  مثل قالب رسمی `../dist`). اگر خروجی به `apps/desktop/dist` کپی شود،
  مقدار را به `../../dist` تغییر بده — فقط یک خط.
- **مبدأ API**: بستهٔ دسکتاپ از `https://localhost` سرو می‌کند؛ بنابراین هنگام
  `build:root` متغیر `VITE_API_BASE=https://<origin>` را ست کن — کلاینت تولیدی
  از طریق `optionsFromEnv` همان را مصرف می‌کند (بدون URL دستی، D5). در وبِ عادی
  این متغیر unset است و پایهٔ طلایی `/api/v1` می‌ماند.

## اهداف بسته‌بندی

| سیستم‌عامل | targets |
|---|---|
| Windows | `msi`, `nsis` (webview2 downloadBootstrapper) |
| Linux | `appimage`, `deb` |
| macOS | `dmg`, `app` (حداقل 10.15) |

امضا/notarization/آپدیت خودکار: `docs/UPDATER.md` (Step 9).
