# Phase 7 — Step 1: Frontend Audit & Architecture Plan (v8/10)

> Status: **DRAFT — awaiting owner approval of the structure** before Step 2 begins.
> Scope discipline: the backend (Rust workspace) is **frozen** — zero route/shape/enum changes in Phase 7.
> This document is the audit + plan required by Phase 7 Step 1 ("Audit + plan, wait for structure approval").

---

## ۰. خلاصهٔ فارسی (Executive Summary)

**اصلاح شماره‌گذاری:** بک‌اند روی دیسک تا **فاز ۶ (AI Gateway) = v7/10** جلو رفته است (۳۱۷ تست سبز، clippy صفر)؛ پس طبق فرمول ثبت‌شده در PROGRESS.md («فاز ۰→v1 … فاز ۸→v9»)، تحویل فاز ۷ = **v8/10** خواهد بود — نه v7/10 (نسخهٔ v7/10 از قبل به فاز ۶ اختصاص یافته و `Phase6-v7of10.zip` موجود است).

**یافتهٔ صادقانهٔ ممیزی:** سورس TS فرانت قدیمی (`front/dashboard`، `front/webapp`) در سندباکس فعلی موجود نیست (فقط ورک‌اسپیس Rust + اسناد باقی مانده‌اند؛ هیچ `package.json` روی دیسک نیست). ممیزی بر پایهٔ سه منبع مستند انجام شده: ADR-009 §2.3 (dashboard 6,513 LOC + Mini App 3,009 LOC، React)، سند ساختار نهایی (نگاشت `front/* → apps/*`)، و مشخصات مالک (۳۳ صفحه). چون مالک بازطراحی from-scratch خواسته، این محدودیت مسیر Step 2 را نمی‌بندد؛ نقشهٔ ۳۳ صفحه به‌صورت DRAFT تنظیم شد و با ارسال دوبارهٔ `front/` توسط مالک قابل دقیق‌شدن است.

**تصمیم‌های اصلی:** تک‌کد‌بیس pnpm + Vite + React، سیستم‌طرح Aurora در `packages/ui` (RTL درجه‌یک، دارک/لایت، فونت سلف-هاست Vazirmatn/Inter)، api-client تولیدشده از OpenAPI 3.1 (openapi-typescript + openapi-fetch — بدون URL دستی)، شل راهبردی در `/` با تشخیص Telegram/نقش/زبان/دستگاه، Tauri v2 (نه Electron)، PWA + Capacitor + Tauri به‌صورت skeleton، بدون دیتابیس محلی، کلید Idempotency سمت کلاینت برای هر mutation پولی، و دیکشنری خطا با همان واژگان i18n بات (کد خام هرگز نمایش داده نمی‌شود).

**سرو static بدون هیچ تغییر بک‌اند:** ServeDir موجود فاز ۴ با `STATIC_DIR` به ریشهٔ dist ترکیبی جدید pointing می‌شود؛ SPAها hash-routing می‌گیرند (پیشینهٔ ADR-009)، سایت عمومی MPA استاتیک است — پس هیچ SPA-fallback سروری لازم نیست و `NO_FRONTEND` هم معنا پیدا می‌کند.

**درخواست:** تأیید ساختار بخش ۷ → سپس Step 2 (Aurora + api-client) شروع می‌شود.

---

## 1. State of record (backend truth, verified on disk)

| ADR-009 Phase | Scope | Delivered as | Evidence |
|---|---|---|---|
| 0 | infrastructure & rules | v1/10 | `Phase0-v1of10.zip` |
| 1 | pure domain core (money/ledger/escrow/charter/errors) | v2/10 | 42 tests |
| 2 | persistence (sqlx + Outbox + Genesis `legacy:genesis:v1`) | v3/10 | Σ wallets = 187,672 Đ proof |
| 3 | contracts — serde + utoipa (OpenAPI 3.1) | v4/10 | golden fixtures, zero-diff vs TS zod |
| 4 | Axum server + static dashboard ServeDir | v5/10 | 77 tests, 46 routes |
| 5 | teloxide bot + HITL queue (≥ 50,000 Đ) | v6/10 | 49 tests |
| 6 | AI Gateway (3-slot fallback, redaction, cap) | **v7/10** | 317 workspace tests green |
| **7** | **frontend integration + optional WASM** | **v8/10 (this phase)** | — |
| 8 | dual-run → data migration → cut-over | v9/10 | — |
| 9 | final signature | v10/10 | — |

**Correction vs. owner prompt:** the prompt says "Backend is DONE up to Phase 5 (v6/10)". Disk shows Phase 6 (AI Gateway) is also done (v7/10). **Phase 7 therefore ships as v8/10** per the version formula recorded in PROGRESS.md. Nothing else changes.

**Backend contract facts relevant to Phase 7 (all verified in source):**

- 46 operations across 40 paths under `/api/v1` (registry: `crates/contracts/src/routes.rs`, ADR-005 "one definition per endpoint"); served doc at `GET /api/v1/openapi.json` (fully inlined, no `$ref`) + Redoc page at `/api/v1/docs`; unknown API path → JSON envelope 404 `NOT_FOUND`.
- One additive route outside the TS registry: `POST /api/v1/bot/webhook` (Phase 5).
- Error envelope everywhere: `{ "error": { "code", "message", "meta?" } }`; statuses 401/404/422/429/503 (+400 INVALID_REQUEST for malformed bodies, 501 NOT_IMPLEMENTED on `PUT /settings`, 502/503 AI_*).
- Money = integer Din, hard Deserialize ceiling `2^53−1`; server formats `187,672 Đ`.
- Escrow = 8-state machine; tax 10% (seller 90 / treasury 10) applied atomically at release/approve; HITL required at ≥ 50,000 Đ (`REQUIRES_HUMAN_APPROVAL` → HITL queue row).
- Idempotency: body-level key (`IdempotencyKey` ≥ 8 chars) persisted as `Tx.n` with a UNIQUE constraint — replay returns the original transaction, never double-pay (Phase 4 acceptance).
- Rate limiting: sliding-window 60 s, separate admin bucket, 429 `RATE_LIMITED` with `meta.limit`.

## 2. Old frontend audit (`front/dashboard` + `front/webapp`)

### 2.1 Evidence base — honest limitation

The original TS monorepo (`back/`, `front/`, `packages/`) is **not present in the current sandbox** — only the Rust workspace, ADR-009, the final-structure document, and the golden fixtures survived. Verified: zero `package.json` anywhere on disk; phase zips contain only `rust/`.

The audit below is therefore built from the three authoritative documented sources:

1. **ADR-009 §2.3** — `front/dashboard` **6,513 LOC** + `front/webapp` **3,009 LOC** = 9,522 LOC, decision "🟦 React — stays, fed through the same OpenAPI; optional WASM (Phase 7)".
2. **Agent-One-Final-Structure.html** — mapping already planned: `front/dashboard · front/webapp → apps/dashboard/ + apps/webapp/`, `front/public-site → apps/site/` ("۳۰ صفحه" noted there).
3. **Owner prompt (2026-11-12)** — "front/webapp Mini App (~3009 LOC, 33 pages)".

Discrepancy: ADR structure doc says 30 pages, owner says 33. Owner prompt is authoritative; the page map in §2.4 is a **DRAFT** to be reconciled if the owner re-uploads `front/` (one-line request, listed in Risks).

Because the owner directive is a **from-scratch redesign** ("REDESIGN the frontend from scratch as a strategic, unified shell"), this limitation blocks nothing downstream — it only reduces the precision of "what to keep" below concept level.

### 2.2 KEEP — concepts carried into the new build

| Concept (old front) | Carried forward as |
|---|---|
| React SPA stack, TS | React 18 + TS strict, Vite 6 (same paradigm, modern toolchain) |
| Hash routing precedent (ADR-009: "same hash-routing") | hash router in dashboard + webapp (no server SPA fallback needed) |
| OpenAPI-fed clients | `packages/api-client` **generated** (no hand-written URLs) |
| fa (RTL) + en (LTR) bilingual UX | `packages/i18n` with first-class RTL flip |
| "Aurora" design system name | `packages/ui` Aurora: tokens, dark/light, RTL |
| Escrow lifecycle UX (buy → fund → deliver → release/dispute/cancel) | webapp flows rebuilt on the same 8-state machine |
| Admin sections behind dashboard routes | dashboard rebuilt on the 15 dashboard routes |
| Telegram Mini App integration (initData) | shell detection + `packages/api-client` identity layer |
| Static serving through `STATIC_DIR` (`front/dashboard/dist`) | combined static root (§5) — same env, new path |
| Error handling that never leaks internals | localized fa/en dictionary mirroring bot i18n wording |

### 2.3 DROP / REPLACE — what the redesign removes

| Old pattern | Replaced by | Why |
|---|---|---|
| Hand-written `fetch` calls / URLs | generated typed client (`openapi-typescript` + `openapi-fetch`) | zero drift vs frozen contract (acceptance #4) |
| Any local money state as truth | server is the single source; local cache is UI-only | shared-DB rule; no per-platform DB |
| Google Fonts CDN | self-hosted Vazirmatn (fa) + Inter (en) woff2 in `packages/ui/fonts` | performance + privacy + offline shell |
| Electron desktop (Gen2 `packages/desktop`) | **Tauri v2** | Rust-native, ~10 MB, memory-safety story aligns with backend; Electron not retained (decision D2) |
| Ad-hoc per-app i18n strings | `packages/i18n` dictionaries + error-code map | one dictionary, fa/en parity, mirrors bot wording |
| No role-aware entry point | `apps/shell` strategic index with context detection | the command-center requirement |
| Optimistic money moves | honest pending states + offline queue replay-safe via idempotency keys | double-click must never double-pay |
| Raw wire codes in UI | localized messages for every `error.code` | "never show raw wire codes" |

### 2.4 WebApp page map — DRAFT (33 pages, to reconcile with owner's original list)

Derived from the 14 bot user commands (Start, Help, Menu, Balance, History, Products, Buy, Status, Fund, Deliver, Release, Dispute, Cancel, Lang) + public API surface:

| # | Page | # | Page | # | Page |
|---|---|---|---|---|---|
| 1 | Storefront home | 12 | Escrow cancel confirm | 23 | AI agents & skills directory |
| 2 | Products list | 13 | HITL status (queue position) | 24 | Quests (read-only) |
| 3 | Product detail | 14 | Wallet (balance) | 25 | Bookings (read-only) |
| 4 | Buy flow (amount + 90/10 preview) | 15 | Transaction history | 26 | Leaderboard (read-only) |
| 5 | Escrow created (pending fund) | 16 | Transaction detail | 27 | Economy transparency |
| 6 | My escrows list | 17 | My orders | 28 | Charter (10 articles) |
| 7 | Escrow detail (8-state timeline) | 18 | Order detail | 29 | Notifications |
| 8 | Escrow fund confirm | 19 | Profile (`user:tg:*`) | 30 | Help / FAQ |
| 9 | Escrow deliver (seller) | 20 | Settings | 31 | Offline shell state |
| 10 | Escrow release confirm (HITL badge ≥ 50,000 Đ) | 21 | Language | 32 | Error / 404 |
| 11 | Escrow dispute | 22 | AI assistant chat | 33 | About / brand |

### 2.5 Dashboard section map (15 dashboard routes → 7 sections)

| Section | Routes used | Notes |
|---|---|---|
| Overview | `GET /overview`, `GET /brand` | command center tiles |
| Economy | `GET /economy` | treasury, trial balance |
| Products | `GET/POST /products`, `PATCH/DELETE /products/{id}` | archive = soft delete |
| Orders | `GET /orders`, `POST /orders/{id}/status`, `GET /users`, `GET /wallets` | order vocabulary mapping preserved |
| Approvals (HITL) | `GET /approvals`, `POST /approvals/{id}/decide`, `GET /hitl`, `GET /hitl/{id}` | queue status visible |
| Growth | `GET /quests`, `GET /bookings`, `GET /leaderboard` | read-only gen2 views |
| AI console | `GET /ai/usage` (+ public agents/skills) | usage metering view |
| Platform | `GET/PUT /settings`, `GET /security`, `POST /system/backup` | settings PUT still 501 — honest UI state |

## 3. Frozen API inventory (46 ops, security per ADR-005 registry)

Verified column-by-column from `crates/contracts/src/routes.rs`:

| Family | Operations | Security |
|---|---|---|
| System | `GET /health`, `GET /economy` | Public |
| System | `POST /system/backup` | Admin |
| Accounts | `POST /accounts`, `GET /accounts` | Admin |
| Accounts | `GET /accounts/{id}/balance` | Public |
| Wallet | `POST /wallet/mint`, `POST /wallet/burn` | Admin |
| Escrows | `POST /escrows`, `GET /escrows`, `POST /escrows/{id}/fund` · `/deliver` · `/release` · `/dispute` · `/refund` · `/cancel` · `/expire` | Admin |
| Escrows | `GET /escrows/{id}` | Public |
| HITL | `POST /hitl`, `GET /hitl`, `GET /hitl/{id}`, `POST /hitl/{id}/approve`, `POST /hitl/{id}/reject`, `POST /hitl/sweep` | Admin |
| AI | `POST /ai/chat` (identity-gated inside: staff key **or** Telegram initData; else 401 `AI_IDENTITY_REQUIRED`), `GET /ai/agents`, `GET /ai/skills` | Public (gated) |
| AI | `GET /ai/usage` | Admin |
| Bot | `POST /bot/webhook` (additive, Telegram secret) | Internal |
| Dashboard | `GET /brand`, `GET /products` | Public |
| Dashboard | `POST /products`, `PATCH/DELETE /products/{id}`, `GET /orders`, `POST /orders/{id}/status`, `GET /users`, `GET /wallets`, `GET /approvals`, `POST /approvals/{id}/decide`, `GET/PUT /settings`, `GET /security`, `GET /overview` | Admin |
| Growth | `GET /quests`, `GET /bookings`, `GET /leaderboard` | Public |
| Contract doc | `GET /openapi.json`, `GET /docs` | Public |

Design consequence: the webapp's citizen flows (escrow lifecycle) currently execute through **admin-key routes** — in Gen3 the Telegram bot orchestrates for citizens. The user-facing webapp therefore (a) renders read-only citizen views from Public routes (balance by id, escrow detail, products, economy, leaderboard), (b) uses identity-gated `POST /ai/chat` for conversational actions, and (c) deep-links "open in Telegram bot" for citizen mutations — **until the owner decides otherwise**. This keeps the API frozen and still delivers the 33-page UX. Flagged as decision D14 (needs owner awareness, not a backend change).

## 4. Auth matrix (frontend side)

| Actor | Credential | Wire behavior | Guest behavior |
|---|---|---|---|
| Admin (dashboard) | `X-KIA-Key` header | 503 `ADMIN_DISABLED` (server unconfigured) / 401 `ADMIN_KEY_REQUIRED` (missing) / 401 `INVALID_ADMIN_KEY` (wrong) | every Admin route surfaces localized 401/503 state, key entry screen |
| Citizen (webapp in Telegram) | Telegram `initData` (HMAC-SHA256 verified server-side, `user:tg:<id>`) | sent with `POST /ai/chat`; identity never faked client-side | outside Telegram → read-only guest |
| Guest | none | Public routes only; 401 envelope on anything else | honest 401 screen, no fabricated data |
| Rate limit | per-IP sliding window, admin bucket shared | 429 `RATE_LIMITED` + `meta.limit` | client backs off, shows localized message |

## 5. Serving model — zero backend change

Current server behavior (verified `crates/server/src/routes.rs`): API nested under `/api/v1`; root is `ServeDir` on `STATIC_DIR` (default `front/dashboard/dist`) with `append_index_html_on_directories(true)`; missing dir → `NO_FRONTEND` envelope; **no SPA fallback** (Phase 4 decision deferred this to Phase 7); **no CSP header emitted today** (verified — parity kept, decision D13).

New model — one combined static root produced by the monorepo build:

```
STATIC_DIR/dist/
├── index.html            # apps/shell strategic index (detected context → route)
├── manifest.webmanifest  # PWA (shell)
├── sw.js                 # service worker (offline shell)
├── icons/                # 192 / 512 + apple-touch
├── admin/                # dashboard build (hash-routed SPA, /admin/ → index.html)
├── app/                  # webapp build (hash-routed SPA, /app/ → index.html)
└── site/                 # public site (static MPA pages, SEO)
```

- `GET /` → shell. Links use trailing slashes (`/admin/`, `/app/`, `/site/…`) so ServeDir resolves directory indexes — no server change, assets still 404 honestly.
- SPA deep links inside `/admin/#…` and `/app/#…` never hit the server path router (hash), so the Phase 4 "no SPA fallback" decision stands.
- `STATIC_DIR` remains the only env knob (ops-level value change, not code).

## 6. UX rules codified (from owner spec + domain)

- **Money:** integer Din only; display `187,672 Đ` exactly like `dropagentx-domain` formatting; never float; fees/tax never hidden; **90/10 split shown before any release confirm**.
- **HITL UX:** amount ≥ 50,000 Đ → prominent "Needs human approval" state + queue status (from `GET /hitl/{id}`).
- **Touch & layout:** mobile-first; ≥ 44 px touch targets; bottom nav on mobile; sidebar on desktop.
- **RTL first-class:** fa is the default and must mirror perfectly (logical CSS properties only; fa ↔ en flip tested both ways).
- **Performance:** route-level code splitting; self-hosted fonts; bundle budgets: shell ≤ 80 KB gzip initial, webapp route chunk ≤ 120 KB, dashboard initial ≤ 200 KB, shared `ui + api-client` ≤ 60 KB; WASM lazy threshold 300 KB gzip else dropped (D10).
- **A11y:** semantic HTML, visible focus states, aria-labels on all money actions.
- **States:** every route ships empty / loading / error states; localized fa/en messages for every backend `error.code` (dictionary mirrors `crates/bot/src/i18n.rs` wording for the 20 domain codes + server wire codes); raw codes never shown.

## 7. Target architecture (new code, nothing in `rust/` touched)

```
Agent-One/
├── apps/
│   ├── shell/        # strategic index: device + Telegram + role + lang detection → smart route + role switcher
│   ├── dashboard/    # admin redesign (React SPA, hash routes, /admin/)
│   ├── webapp/       # user Mini App / PWA, 33 pages (React SPA, hash routes, /app/)
│   ├── site/         # public landing (static MPA, SEO, /site/)
│   ├── desktop/      # Tauri v2 skeleton (Win .msi/.exe, Linux AppImage/deb, macOS .dmg — config only)
│   └── mobile/       # Capacitor skeleton (Android/iOS config — no React Native rewrite)
├── packages/
│   ├── ui/           # Aurora: tokens, dark/light, RTL, components, self-hosted fonts
│   ├── api-client/   # GENERATED from OpenAPI 3.1 + typed fetch + auth layers + Idempotency-Key store
│   ├── i18n/         # fa/en dictionaries + RTL flip + error-code map
│   └── domain-wasm/  # OPTIONAL: dropagentx-domain → WASM for client-side validation (lazy, threshold-gated)
├── pnpm-workspace.yaml
├── package.json      # root scripts (dev/build/test/lint per app)
└── rust/             # UNTOUCHED backend workspace (clippy/fmt/test stay green)
```

- **Toolchain:** pnpm (via corepack 0.36) · Node 24 · Vite 6 · React 18 · TypeScript strict · Tailwind CSS 4 for Aurora tokens/utilities (logical properties for RTL) · TanStack Query 5 (server state) · zustand (session/lang/theme only) · `openapi-typescript` codegen.
- **`packages/api-client` generation source:** pinned `rust/crates/contracts/tests/fixtures/golden-openapi.json` (guarded by the contract's own golden tests) + a round-trip E2E against the live server's `/api/v1/openapi.json` (acceptance #4). A lint rule bans hand-written API URLs outside the client package.
- **No local database anywhere.** localStorage/IndexedDB = session, language, theme, UI cache, offline queue only.

## 8. Offline & idempotency design (money honesty)

1. Every money mutation gets a client-generated idempotency key (ULID), created **once per logical action**, persisted in the queue store, and **reused across retries** — double-click / reconnect replay hits backend `Tx.n` UNIQUE semantics and returns the original transaction (replay-safe, balance unchanged).
2. Offline: read-only cache of last-known public data + queued mutations rendered as explicit "pending" — never optimistic balances; on reconnect, sync and surface conflicts honestly.
3. 409/422 replay responses are treated as success-with-receipt (the backend's original tx), not errors.

## 9. Platform skeletons (Phase 7 delivers configs, Step 9 signs)

| Target | Phase 7 deliverable | Deferred to Step 9 |
|---|---|---|
| PWA | `manifest.webmanifest`, icons 192/512, service worker (offline shell), Lighthouse budget config | store-quality audit iteration |
| Android | Capacitor config + app icon + splash + signing placeholder doc | keystore, signing |
| iOS | Capacitor iOS config + apple-touch icon + safe-area insets + TestFlight notes doc | Apple Developer, provisioning |
| Windows/Linux/macOS | Tauri v2 config skeletons (`.msi/.exe`, `AppImage/deb`, `.dmg`) + auto-update placeholder | code-signing certs, notarization |

## 10. Step plan (commit after each step) → acceptance mapping

| Step | Content | Proves |
|---|---|---|
| 1 (this) | Audit + plan — **awaiting structure approval** | Deliverable #1 |
| 2 | `packages/ui` Aurora (tokens, dark/light, RTL) + `packages/i18n` + generated `packages/api-client` + contract round-trip test | typed-client criterion |
| 3 | `apps/shell` strategic index (detection + routing + role switcher) + static-root build wiring | shell criterion |
| 4 | `apps/dashboard` redesign on Aurora | admin E2E |
| 5 | `apps/webapp` redesign (33-page map) | money UX, HITL UX |
| 6 | PWA + Capacitor + Tauri skeletons | installable PWA, config builds |
| 7 | E2E (mint→escrow→release = 90/10, replay-safe, 401s), Lighthouse (Playwright Chromium — see R2), RTL flip tests, PROGRESS.md (v8/10) + README-fa.md, zip `نسخه 8 از 10` | full acceptance list |

## 11. Risks & unknowns

| # | Risk | Mitigation |
|---|---|---|
| R1 | Old `front/` sources absent from sandbox | doc-based audit (§2.1); owner may re-upload `front/` to refine §2.4 page map — nothing else blocked |
| R2 | No Chrome in sandbox for Lighthouse | attempt `npx playwright install chromium`; if blocked, ship budget config + document local run |
| R3 | Page count 33 (owner) vs 30 (structure doc) | owner prompt authoritative; DRAFT map flagged |
| R4 | Citizen mutations are admin-key routes today | read-only citizen views + AI-chat actions + Telegram deep-links; no API change (D14) |
| R5 | Signing / store submission | explicitly out of Phase 7; Step 9 checklist documented per platform |

## 12. Decision register (ADR-009 deltas)

| # | Decision |
|---|---|
| D1 | Owner directive (2026-11-12) supersedes ADR-009 §2.3 "React stays untouched" and the §3 non-goal "frontend rewrite" → unified shell redesign with the API frozen. |
| D2 | Tauri v2 replaces Electron (Gen2 `packages/desktop` not retained); justification: Rust-native, ~10 MB, memory-safety narrative aligns with the backend story. |
| D3 | One combined static root served by the existing ServeDir; only `STATIC_DIR` value changes (ops), zero backend code changes. |
| D4 | Hash routing for the two SPAs (ADR-009 precedent); public site is a static MPA — no server SPA fallback required. |
| D5 | api-client generated from pinned `golden-openapi.json` + live-server round-trip test; hand-written API URLs banned by lint. |
| D6 | TanStack Query 5 for server state; zustand only for session/lang/theme; money state never local. |
| D7 | Client-generated idempotency keys, stable per logical action, reused on retry (replay-safe per `Tx.n` UNIQUE). |
| D8 | Error dictionary mirrors `crates/bot/src/i18n.rs` fa/en wording for the 20 domain codes + all wire codes. |
| D9 | Fonts self-hosted (Vazirmatn fa / Inter en); no Google Fonts CDN. |
| D10 | `domain-wasm` optional: lazy-loaded, 300 KB-gzip threshold, cleanly dropped if over (ADR-009 risk table allows). |
| D11 | Phase 7 ships as **v8/10** (formula فاز N → v(N+1)); prompt's "v7/10" corrected — v7/10 is already Phase 6. |
| D12 | pnpm via corepack; Node 24; Vite 6; React 18; Tailwind 4; TS strict. |
| D13 | Server emits no CSP header today — parity kept (no backend change); frontend does not depend on server CSP. |
| D14 | Citizen mutation path in the webapp = read-only public views + identity-gated `/ai/chat` + Telegram bot deep-links until the owner decides otherwise (API stays frozen). |

---

**END OF STEP 1 — awaiting owner approval of the structure (§7) to begin Step 2.**
