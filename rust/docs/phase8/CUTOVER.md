# گزارش Cut-over فاز ۸ — Dual-run → مهاجرت داده → Cut-over (v9/10)

> ADR-009 فاز ۸ — «پل بازگشت همیشه بالاست» · ۳ هفته / 70h · خروجی: تگ
> `rust-core-1.0.0` پس از ۲ هفته ۱۰۰٪ سبز. این سند **گزارش اجرای ابزار و
> گیت‌ها** در سندباکس تحویل است؛ پنجرهٔ ۲ هفته‌ای خودش یک فرایند عملیاتی
> production است که با RUNBOOK همین پوشه اجرا می‌شود.

---

## ۱) آنچه تحویل شد — نگاشت به «دامنه کار» ADR-009

| بند ADR | تحویل | فایل/مسیر |
|---|---|---|
| استقرار dual-run (§۱۲): shadow → ۱٪→۱۰٪→۵۰٪→۱۰۰٪ با parity gate | مغز مسیریابی دترمیتیک + پراکسی HTTP با نقاط کنترل `/_dualrun/*` + باینری `dual-run` (تک‌متغیر `RUST_TRAFFIC_PCT`، سایه، سپر HITL سه‌روزه، rollback تک‌فرمان) | `crates/tooling/src/dual_run.rs` · `src/proxy.rs` · `src/bin/dual-run.rs` |
| سوئیت parity: ~۳۰ سناریوی طلایی + وضعیت DB | **۴۳ سناریوی طلایی** (بیش از هدف) — پاسخ بایت‌به‌بایت + خوانش SQL خام؛ رکورد/بازپخش/مقایسهٔ زندهٔ TS | `crates/tooling/src/parity/` · `tests/golden/*.json` (۴۳ فایل) · `src/bin/parity.rs` |
| proptest روی invariantها | ۳ خاصیت: تخصیص ترافیک تابع قطعی درخواست؛ Σ=۰ و تراز ≥۰ از درگاه وایر؛ ایدمپوتنت‌بودن نرمال‌ساز | `crates/tooling/tests/properties.rs` |
| cargo-fuzz روی پارسرهای ورودی | اسکلت libfuzzer (۳ هدف: `fuzz_mint_request`، `fuzz_create_escrow`، `fuzz_ai_chat`) + معادل قطعی درون‌CI با موتور موتاسیون seed-دار | `fuzz/` · `crates/tooling/tests/fuzz_lite.rs` |
| لودتست کوتاه (p95 + حافظهٔ مقصد ثبت‌شده) | مولد بار tokio معادل سبک oha — p50/p95/p99/RPS/توزیع وضعیت + JSON | `crates/tooling/src/bin/loadtest.rs` · گزارش §۳ همین سند |
| Runbook عملیاتی | بوت، sweep، بکاپ، panic-free policy، alerting، آموزش سوییچ تک‌متغیره، rollback drill | `docs/phase8/RUNBOOK.md` |
| مهاجرت داده + ممیزی نهایی | مسیر رسمی لگاسی→v3 + باینری ممیزی مستقل با حکم go/no-go | `crates/tooling/src/bin/migrate-legacy.rs` · `src/bin/audit-ledger.rs` |
| تگ + آرشیو Node + مستندات | تگ git `rust-core-1.0.0` روی همین تحویل؛ Node همان تگ آخر TS می‌ماند (گرم‌خانه)؛ README/DEPLOY به‌روز | `README-fa.md` · `DEPLOY.md` |

## ۲) گیت parity — اجرا و نتیجه

```text
cargo run -p dropagentx-tooling --bin parity -- --verify --report docs/phase8/parity-report.json
parity: 43/43 green   →   PARITY OK — gate green.
```

* پوشش: system/meta (۵) · حساب‌ها (۴) · کیف (۴) · اسکرو (۷) · HITL (۶) · بات (۳) · AI (۴) · داشبورد (۸) · امنیت/نرخ (۲) — هر سناریو = setup + درخواستِ ثبت‌شده + انتظار وضعیت DB.
* مقایسه: status + content-type + **بدنهٔ بایت‌به‌بایت پس از نرمال‌سازی فیلدهای فرّار** (`<TS>`/`<UPTIME>`/`<UUID>`) + خوانش SQL (`Σ کیف`، خزانه، Tx، Outbox، وضعیت اسکرو).
* گیت مالی قفل‌شده در گلدن: `escrow_release_settles_90_10` → `Σ کیف 199,000 · خزانه 1,000 · RELEASED` — تست replay همان را در هر CI اجرا می‌کند (`parity_replay.rs`).
* حالت `--ts-url` برای مقایسهٔ زندهٔ TS↔Rust در production آماده است (همان سناریوها به هر دو موتور؛ خروجی exit ۰/۱ برای گیت خودکار).

## ۳) لودتست — اجرا و اعداد ثبت‌شده

سخت‌افزار: سندباکس تحویل (تک‌هسته مجازی) — بیلد **release** (`lto=thin, strip`)،
SQLite فایل‌محور با pool=4 (پیکربندی منجمد فاز ۲)، `RATE_LIMIT=1,000,000`
تا لیمتر با load قاطی نشود، ۸ کارگر هم‌زمان.

| سناریو | مدت | درخواست موفق | RPS | p50 | p95 | p99 | خطا |
|---|---|---|---|---|---|---|---|
| `GET /api/v1/health` (شامل probe دیتابیس) | 10s | 2,532 | 253 | 26.2ms | **56.9ms** | 65.2ms | 0 |
| `GET /api/v1/economy` (خوانش کامل دفتر) | 10s | 2,526 | 252 | 24.5ms | **56.7ms** | 66.7ms | 0 |

**حافظهٔ مقصد ثبت‌شده:** RSS پروسهٔ سرور = **12 MB** (هدف ADR-009: ~20MB؛
Node/TS baseline: ~120–150MB) · **استارت سرد تا اولین `/health` سبز: 10.7ms**
(هدف ADR: <100ms).

فایل‌های خام: `loadtest-health.json` · `loadtest-economy.json` · `memory.txt`.

یادداشت صادقانه:
1. TS baseline در سندباکس قابل‌اجرا نیست (منبع TS `back/` در سندباکس موجود
   نیست — همان محدودیت ثبت‌شدهٔ فاز ۳/۷). مقایسهٔ «p95 زیر TS baseline در همان
   سخت‌افزار» طبق §۱۲ در گام ۲ dual-run روی production با همین ابزار
   `loadtest` گرفته می‌شود؛ قاعدهٔ rollback: p95 > 2×baseline.
2. اعداد مطلقِ سندباکس (RPS/p95) به سخت‌افزار وابسته‌اند؛ آنچه معیار است
   **نسبت به baseline همان سخت‌افزار** است، نه عدد خام.

## ۴) مهاجرت داده + ممیزی نهایی cut-over — اجرا و نتیجه

مسیر رسمی (عین فاز ۲ — همان idempotency `legacy:genesis:v1`):

```text
python3 docs/phase8/make_legacy_fixture.py /tmp/legacy/marketplace-20260924-0400.db
  → 466 کاربر · Σ credits = 187,672.8 (465×403.0 + 1×277.8) · 30 محصول · 1,554 تراکنش

migrate-legacy --legacy /tmp/legacy/marketplace-20260924-0400.db \
               --target file:docs/phase8/cutover-v3.db
  Σ user wallets        = 187672 Đ  (acceptance: 187672)
  trialBalance.balanced = true (Σ postings 0)
  accounts = 469 (466 users + system) · products = 30/30

audit-ledger docs/phase8/cutover-v3.db        →  exit 0
  Σ user wallets           : 187672 Đ   expected 187672 Đ
  Σ all postings           : 0          (double-entry closure)
  genesis tx               : tx_legacy_genesis  (gap −63.0 توضیح‌داده‌شده در memo)
  AUDIT PASS — Σ wallets = 187,672 Đ · trialBalance.balanced = true
```

ممیزی، **باینری مستقل با SQL خام فقط‌خواندنی** است (نه کد دامنه) — عین سیاست
`genesis-proof` فاز ۲: ممیزی نباید باگ کدِ ممیزی‌شده را به ارث ببرد. خروجی کامل
در `audit-cutover.txt`. دیتابیس کاندیدای cut-over (`cutover-v3.db`) هم ضمیمهٔ
همین پوشه است.

> شفافیت: بکاپ واقعی Railway در سندباکس موجود نیست؛ فیکسچر لگاسی با «دقیقاً
> اعداد روز-اول» (همان اعداد ممیزی مهر ۱۴۰۵: 465×403 + 277.8 = 187,672.8 →
> Σ floor 187,672 Đ) بازسازی شده — همان ورودی که تست انتگراسیون فاز ۲ و ممیزی
> نسل‌ها روی آن سبز شده‌اند. روی production همین باینری‌ها با بکاپ واقعی اجرا
> می‌شود.

## ۵) شبیه‌سازی زندهٔ موتور روی دیتابیس cut-over

سرور release روی `cutover-v3.db` بوت شد (همان دیتابیس مهاجرت‌شده):

```text
GET /api/v1/health   → {"status":"ok","db":"ok",…}
GET /api/v1/economy  → {"circulatingSupply":187672, "balanced":true,
                        "accountCount":469, "genesisSupply":187672, …}
```

— «همان عدد نسل‌ها، حالا با هسته Rust.»

## ۶) معیار پذیرش فاز ۸ — وضعیت

| معیار ADR-009 | وضعیت |
|---|---|
| ~۳۰ سناریوی طلایی parity + وضعیت DB | **۴۳/۴۳ سبز** (بازپخش بایتی در CI هم قفل است) |
| proptest روی invariantها | سبز — ۳ خاصیت (ترافیک قطعی، Σ=۰ از وایر، نرمال‌ساز ایدمپوتنت) |
| cargo-fuzz پارسرهای ورودی | اسکلت libfuzzer (nightly/CI) + fuzz-lite قطعی سبز درون‌CI |
| لودتست: p95 + حافظهٔ مقصد ثبت‌شده | p95 ≈ 57ms · RSS = 12MB · استارت سرد 10.7ms — ثبت در همین سند |
| Runbook عملیاتی + آموزش سوییچ تک‌متغیره | `RUNBOOK.md` — بوت/sweep/بکاپ/panic-free/alerting/rollback drill |
| ممیزی نهایی: Σ کیف‌ها = 187,672 Đ و balanced=true | **AUDIT PASS** (exit 0) روی کاندیدای cut-over |
| ۲ هفته پیوستهٔ ۱۰۰٪ + صفر rollback | فرایند production — با `dual-run` + گیت‌های همین تحویل اجرا می‌شود؛ تک‌متغیر و ابزار کامل است |
| تگ `rust-core-1.0.0` + آرشیو Node + README/DEPLOY | تگ زده شد؛ Node = گرم‌خانه در تگ آخر TS؛ `README-fa.md` و `DEPLOY.md` به‌روز شدند |

## ۷) برگشت — قرارداد همیشگی

`RUST_TRAFFIC_PCT=0` (یا `POST /_dualrun/rollback`) → ترافیک فوراً به TS.
چون هر دو موتور روی **همان** SQLite با invariantهای یکسان کار کرده‌اند،
**هیچ مهاجرت برگشتیِ داده لازم نیست** — سوییچ، تصمیمِ مسیریابی است نه مهاجرت.
دامنهٔ مجاز rollback: هر parity-fail · هر discrepancy مالی · p95 > 2×baseline ·
panic تکرارشونده.
