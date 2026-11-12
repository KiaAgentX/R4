/**
 * Golden fixture generator — Phase 3 (ADR-009).
 *
 * Imports the REAL TS contracts package (zod schemas + buildOpenApiDoc) and
 * the REAL route table (transcribed verbatim from back/server/src/api.ts),
 * then emits:
 *   1. golden-openapi.json — the authoritative OpenAPI 3.1 wire doc
 *   2. golden-wire.json    — sample request/response objects, validated by
 *                            the zod schemas themselves before emitting
 *
 * The Rust contracts crate must reproduce BOTH byte-semantically
 * (JSON value equality, key order aside).
 */
import * as C from '/home/z/my-project/Agent-One/back/contracts/src/index.ts';
import { z } from 'zod';
import { writeFileSync } from 'node:fs';

// ── route table — VERBATIM from back/server/src/api.ts (metadata only) ──────
type R = C.ApiRouteDef;
const ROUTES: R[] = [
  { method: 'get', path: '/health', operationId: 'health', summary: 'Liveness probe of the v3 server', tag: 'system', security: 'public', response: C.healthResponseSchema },
  { method: 'get', path: '/economy', operationId: 'getEconomy', summary: 'Nation economy summary — public transparency (charter Article 6)', tag: 'economy', security: 'public', response: C.economySummarySchema },
  { method: 'post', path: '/accounts', operationId: 'createAccount', summary: 'Register a user account (balance starts at 0)', tag: 'accounts', security: 'admin', body: C.createAccountRequestSchema, response: C.accountViewSchema, status: 201 },
  { method: 'get', path: '/accounts', operationId: 'listAccounts', summary: 'List all accounts with derived balances', tag: 'accounts', security: 'admin', response: C.accountListResponseSchema },
  { method: 'get', path: '/accounts/{id}/balance', operationId: 'getBalance', summary: 'Derived balance of one account (public transparency)', tag: 'accounts', security: 'public', params: z.object({ id: C.idSchema }), response: C.balanceResponseSchema },
  { method: 'post', path: '/wallet/mint', operationId: 'mint', summary: 'Mint dinar into a user account — yearly cap enforced (charter Article 3)', tag: 'wallet', security: 'admin', body: C.mintRequestSchema, response: C.walletMoveResponseSchema },
  { method: 'post', path: '/wallet/burn', operationId: 'burn', summary: 'Burn dinar from a user account — overdraft-impossible (charter Article 4)', tag: 'wallet', security: 'admin', body: C.burnRequestSchema, response: C.walletMoveResponseSchema },
  { method: 'post', path: '/escrows', operationId: 'createEscrow', summary: 'Create an escrow (CREATED; money not moved yet)', tag: 'escrows', security: 'admin', body: C.createEscrowRequestSchema, response: C.escrowActionResponseSchema, status: 201 },
  { method: 'get', path: '/escrows', operationId: 'listEscrows', summary: 'List escrows (optionally filtered by status)', tag: 'escrows', security: 'admin', query: z.object({ status: C.escrowStatusSchema.optional() }), response: C.escrowListResponseSchema },
  { method: 'get', path: '/escrows/{id}', operationId: 'getEscrow', summary: 'Read one escrow (public transparency)', tag: 'escrows', security: 'public', params: z.object({ id: C.idSchema }), response: C.escrowViewSchema },
  { method: 'post', path: '/escrows/{id}/fund', operationId: 'fundEscrow', summary: 'Lock buyer money into the escrow account', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.fundEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/deliver', operationId: 'deliverEscrow', summary: 'Mark the product as delivered (money still locked)', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.deliverEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/release', operationId: 'releaseEscrow', summary: 'Settle: seller gets net (90%), treasury gets tax (10%) — HITL gate ≥ 50,000 Đ', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.releaseEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/dispute', operationId: 'disputeEscrow', summary: 'Open a dispute (escrow stays locked until arbitration)', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.disputeEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/refund', operationId: 'refundEscrow', summary: 'Arbitration in favor of the buyer — full refund, no tax', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.refundEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/cancel', operationId: 'cancelEscrow', summary: 'Cancel a CREATED escrow — no money ever moved', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.cancelEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/escrows/{id}/expire', operationId: 'expireEscrow', summary: 'TTL sweep — auto-refund a FUNDED escrow (charter Article 9)', tag: 'escrows', security: 'admin', params: z.object({ id: C.idSchema }), body: C.expireEscrowRequestSchema, response: C.escrowActionResponseSchema },
  { method: 'post', path: '/hitl', operationId: 'createHitlRequest', summary: 'Queue a human-approval request (amount copied from the escrow; duplicate id → replay)', tag: 'hitl', security: 'admin', body: C.createHitlRequestSchema, response: C.hitlViewSchema, status: 201 },
  { method: 'get', path: '/hitl', operationId: 'listHitlRequests', summary: 'List HITL requests (optionally filtered by status)', tag: 'hitl', security: 'admin', query: z.object({ status: C.hitlStatusSchema.optional() }), response: C.hitlListResponseSchema },
  { method: 'get', path: '/hitl/{id}', operationId: 'getHitlRequest', summary: 'Read one HITL request', tag: 'hitl', security: 'admin', params: z.object({ id: C.idSchema }), response: C.hitlViewSchema },
  { method: 'post', path: '/hitl/{id}/approve', operationId: 'approveHitlRequest', summary: 'Approve → ESCROW_RELEASE settles in the same transaction (seller 90% + treasury 10%)', tag: 'hitl', security: 'admin', params: z.object({ id: C.idSchema }), body: C.hitlDecisionRequestSchema, response: C.hitlActionResponseSchema },
  { method: 'post', path: '/hitl/{id}/reject', operationId: 'rejectHitlRequest', summary: 'Reject — no money moves; the escrow stays locked until arbitration decides otherwise', tag: 'hitl', security: 'admin', params: z.object({ id: C.idSchema }), body: C.hitlDecisionRequestSchema, response: C.hitlActionResponseSchema },
  { method: 'post', path: '/hitl/sweep', operationId: 'sweepHitl', summary: 'TTL sweep — expire overdue PENDING requests (bots call this on an interval)', tag: 'hitl', security: 'admin', response: C.hitlSweepResponseSchema },
  { method: 'post', path: '/system/backup', operationId: 'createBackup', summary: 'Consistent SQLite snapshot (VACUUM INTO) into db/backups — prunes to the newest KEEP_BACKUPS files. Admin-gated.', tag: 'system', security: 'admin', response: C.backupCreatedSchema },
  { method: 'post', path: '/ai/chat', operationId: 'aiChat', summary: 'Ask the Zenovix agent team — routing, skills, metering, redaction and audit run server-side', tag: 'ai', security: 'public', body: C.aiChatRequestSchema, response: C.aiChatResponseSchema },
  { method: 'get', path: '/ai/agents', operationId: 'aiAgents', summary: 'The 6 Zenovix agents with the skills each one may call', tag: 'ai', security: 'public', response: C.aiAgentsListResponseSchema },
  { method: 'get', path: '/ai/skills', operationId: 'aiSkills', summary: 'The 16 KIA-OS skills with authorization metadata', tag: 'ai', security: 'public', response: C.aiSkillsListResponseSchema },
  { method: 'get', path: '/ai/usage', operationId: 'aiUsage', summary: 'Per-user token usage against the daily cap (admin key required)', tag: 'ai', security: 'admin', query: z.object({ userId: z.string().min(1) }), response: C.aiUsageViewSchema },
  { method: 'get', path: '/brand', operationId: 'getBrand', summary: 'White-label shell branding (BRAND_NAME / BRAND_TAGLINE env → defaults)', tag: 'dashboard', security: 'public', response: C.brandResponseSchema },
  { method: 'get', path: '/overview', operationId: 'getOverview', summary: '24h KPIs + 14-day series + top products + recent orders, derived from live tables', tag: 'dashboard', security: 'admin', response: C.overviewResponseSchema },
  { method: 'get', path: '/products', operationId: 'listDashProducts', summary: 'Market listings for the command center (public read, q search, limit/offset)', tag: 'dashboard', security: 'public', query: z.object({ limit: z.coerce.number().int().min(1).max(100).default(12), offset: z.coerce.number().int().min(0).default(0), q: z.string().max(80).optional() }), response: C.productListResponseSchema },
  { method: 'post', path: '/products', operationId: 'createDashProduct', summary: 'Create a house listing (seller = treasury); Stars are not a ledger rail — credits only', tag: 'dashboard', security: 'admin', body: C.createProductRequestSchema, response: C.productMutationResponseSchema, status: 201 },
  { method: 'patch', path: '/products/{id}', operationId: 'updateDashProduct', summary: 'Edit a listing (title/category/description/price/active; stock & stars are dashboard-only views)', tag: 'dashboard', security: 'admin', params: z.object({ id: C.idSchema }), body: C.updateProductRequestSchema, response: C.productMutationResponseSchema },
  { method: 'delete', path: '/products/{id}', operationId: 'archiveDashProduct', summary: 'Archive a listing (soft — the row stays for ledger/escrow references)', tag: 'dashboard', security: 'admin', params: z.object({ id: C.idSchema }), response: C.productMutationResponseSchema },
  { method: 'get', path: '/orders', operationId: 'listDashOrders', summary: 'Escrow marketplace orders in dashboard shapes (status filter + paging)', tag: 'dashboard', security: 'admin', query: z.object({ limit: z.coerce.number().int().min(1).max(100).default(20), offset: z.coerce.number().int().min(0).default(0), status: z.enum(['all', 'paid', 'pending', 'delivered', 'refunded', 'cancelled']).default('all') }), response: C.orderListResponseSchema },
  { method: 'post', path: '/orders/{id}/status', operationId: 'dashOrderAction', summary: 'deliver → DELIVERED (from FUNDED) · cancel → CANCELLED (from CREATED) · refund → REFUNDED (from DISPUTED)', tag: 'dashboard', security: 'admin', params: z.object({ id: C.idSchema }), body: C.orderActionRequestSchema, response: C.orderActionResponseSchema },
  { method: 'get', path: '/users', operationId: 'listDashUsers', summary: 'User accounts (kind=user) for the wallets→users subtab', tag: 'dashboard', security: 'admin', query: z.object({ limit: z.coerce.number().int().min(1).max(200).default(100) }), response: C.dashUserListResponseSchema },
  { method: 'get', path: '/wallets', operationId: 'listDashWallets', summary: 'Derived balances + escrow-locked amounts per user account', tag: 'dashboard', security: 'admin', query: z.object({ limit: z.coerce.number().int().min(1).max(200).default(100) }), response: C.dashWalletListResponseSchema },
  { method: 'get', path: '/quests', operationId: 'listDashQuests', summary: 'Quest rail — empty until the quest engine ships (renders a clean empty state)', tag: 'dashboard', security: 'public', response: C.dashListResponseSchema },
  { method: 'get', path: '/bookings', operationId: 'listDashBookings', summary: 'Bookings rail — empty until the booking engine ships (renders a clean empty state)', tag: 'dashboard', security: 'public', response: C.dashListResponseSchema },
  { method: 'get', path: '/leaderboard', operationId: 'dashLeaderboard', summary: 'sales = settled escrows per seller (real); xp/ref rails ship with the quest engine', tag: 'dashboard', security: 'public', query: C.leaderboardQuerySchema, response: C.leaderboardResponseSchema },
  { method: 'get', path: '/approvals', operationId: 'listDashApprovals', summary: 'HITL queue in dashboard shapes (risk level derives from the charter threshold)', tag: 'dashboard', security: 'admin', query: C.approvalListQuerySchema, response: C.approvalListResponseSchema },
  { method: 'post', path: '/approvals/{id}/decide', operationId: 'dashApprovalDecision', summary: 'Approve settles the escrow in the same transaction; reject locks it for arbitration', tag: 'dashboard', security: 'admin', params: z.object({ id: C.idSchema }), body: C.approvalDecisionRequestSchema, response: C.approvalDecisionResponseSchema },
  { method: 'get', path: '/settings', operationId: 'listDashSettings', summary: 'Runtime settings — empty until the KV model ships (brand lives in env today)', tag: 'dashboard', security: 'admin', response: C.settingsListResponseSchema },
  { method: 'put', path: '/settings', operationId: 'putDashSetting', summary: '501 until the KV model ships — honest refusal beats a silent no-op', tag: 'dashboard', security: 'admin', body: C.settingsPutRequestSchema, response: C.settingsListResponseSchema },
  { method: 'get', path: '/security', operationId: 'dashSecurity', summary: 'Live defense-in-depth layers (real config flags) + the audit trail tail', tag: 'dashboard', security: 'admin', response: C.securityResponseSchema },
];

const INFO = {
  title: 'DropAgentX v3 — Unified Core API',
  version: '4.4.3',
  description:
    'Admin service API over the pure domain core: integer money, immutable double-entry ledger, escrow state machine, 10-article monetary charter. Errors follow { error: { code, message } } — codes are additive.',
};

if (ROUTES.length !== 46) throw new Error(`route transcription drift: ${ROUTES.length} ≠ 46`);

// ── 1. golden OpenAPI doc ────────────────────────────────────────────────────
const doc = C.buildOpenApiDoc(ROUTES, INFO);
writeFileSync('/tmp/ts-golden/golden-openapi.json', JSON.stringify(doc, null, 1));

// ── 2. golden wire samples (each VERIFIED by its zod schema first) ──────────
const samples: Record<string, unknown> = {
  // responses
  'health': { status: 'ok', service: 'dropagentx-v3-server', version: '4.4.3', llmConfigured: true, llmModel: 'gpt-4o-mini', db: 'ok', bot: 'ok', ai: 'ok', uptime: 1234.5 },
  'economy': { currency: 'DNX', symbol: 'Đ', circulatingSupply: 187672, treasuryBalance: 127672, escrowLocked: 60000, mintedAllTime: 247672, burnedAllTime: 60000, mintedThisYear: 247672, yearlyMintCap: 10000000, genesisSupply: 187672, trialBalanceSum: 0, balanced: true, accountCount: 468, openEscrows: 2 },
  'accountView': { id: 'user:tg:123', kind: 'user', label: 'Ali', balance: 500, createdAt: '2026-01-15T10:30:00.000Z' },
  'accountList': { items: [{ id: 'user:tg:123', kind: 'user', label: null, balance: 0, createdAt: '2026-01-15T10:30:00.000Z' }] },
  'balance': { accountId: 'user:tg:123', balance: 7 },
  'walletMove': { txId: 'tx:01', accountId: 'user:tg:123', balance: 500, replayed: false },
  'escrowView': { id: 'escrow:01', buyerId: 'user:tg:1', sellerId: 'user:tg:2', amount: 60000, productRef: 'prod:1', status: 'FUNDED', createdAt: '2026-02-01T00:00:00.000Z', fundedTxId: 'tx:02', settleTxId: null, disputeReason: null, deliveryRef: null, expiresAt: '2026-03-01T00:00:00.000Z' },
  'escrowList': { items: [] },
  'escrowAction': { escrow: { id: 'escrow:01', buyerId: 'user:tg:1', sellerId: 'user:tg:2', amount: 60000, productRef: null, status: 'RELEASED', createdAt: '2026-02-01T00:00:00.000Z', fundedTxId: 'tx:02', settleTxId: 'tx:03', disputeReason: null, deliveryRef: 'ref-9', expiresAt: null }, txId: 'tx:03', replayed: false },
  'hitlView': { id: 'hitl:01', kind: 'ESCROW_RELEASE', escrowId: 'escrow:01', amount: 60000, requesterId: 'user:tg:1', note: 'big order', status: 'PENDING', createdAt: '2026-02-01T00:00:00.000Z', expiresAt: '2026-02-02T00:00:00.000Z', decidedBy: null, decidedAt: null, decisionNote: null },
  'hitlList': { items: [] },
  'hitlSweep': { expired: 3 },
  'hitlAction': { request: { id: 'hitl:01', kind: 'ESCROW_RELEASE', escrowId: 'escrow:01', amount: 60000, requesterId: 'user:tg:1', note: null, status: 'APPROVED', createdAt: '2026-02-01T00:00:00.000Z', expiresAt: '2026-02-02T00:00:00.000Z', decidedBy: 'admin', decidedAt: '2026-02-01T05:00:00.000Z', decisionNote: 'ok' }, escrow: null, txId: null, replayed: true },
  'backupCreated': { file: '/vol/db/backups/db-2026.sqlite', sizeBytes: 1048576, kept: 7, createdAt: '2026-02-01T00:00:00.000Z' },
  'aiChat': { reply: 'سلام!', agent: 'receptionist', via: 'llm', provider: 'openai', toolTraces: [{ skill: 'wallet.balance', ok: true, summary: 'read balance', pendingApproval: false, code: null }], usage: { provider: 'openai', model: 'gpt-4o-mini', promptTokens: 120, completionTokens: 45, totalTokens: 165 }, pendingApproval: false, redactions: 0 },
  'aiAgents': { items: [{ name: 'orchestrator', emoji: '🎯', titleFa: 'هماهنگ‌کننده', titleEn: 'Orchestrator', description: 'routes', skills: ['wallet.balance'] }] },
  'aiSkills': { items: [{ id: 'wallet.balance', kind: 'read', titleFa: 'موجودی', titleEn: 'Balance', description: 'read balance', allowedAgents: ['orchestrator'], staffOnly: false }] },
  'aiUsage': { userId: 'user:tg:123', usedToday: 500, cap: 50000, remainingToday: 49500 },
  'brand': { ok: true, data: { name: 'DropAgentX', tagline: 'Empire' } },
  'overview': { ok: true, data: { users: 466, products: 30, orders: 12, revenueStars: 0, revenueCredits: 4500, pendingApprovals: 1, aiCalls: 99, series: [{ date: '2026-02-01', revenue: 100, orders: 2 }], topProducts: [{ title: 'VIP', salesCount: 7 }], recentOrders: [{ id: 'escrow:9', title: 'VIP', buyer: { name: 'Ali', username: 'ali' }, totalStars: 0, totalCredits: 1500, status: 'delivered', createdAt: '2026-02-01T00:00:00.000Z' }] } },
  'productView': { id: 'prod:1', sellerId: 'treasury', title: 'VIP', description: 'desc', category: 'digital', priceStars: 0, priceCredits: 1500, stock: -1, salesCount: 7, featured: true, status: 'active', createdAt: '2026-02-01T00:00:00.000Z' },
  'productList': { ok: true, data: { items: [], total: 0, limit: 12, offset: 0 } },
  'productMutation': { ok: true, data: { id: 'prod:1', sellerId: 'treasury', title: 'VIP', description: 'd', category: 'c', priceStars: 0, priceCredits: 1500, stock: 5, salesCount: 0, featured: false, status: 'active', createdAt: '2026-02-01T00:00:00.000Z' } },
  'orderView': { id: 'escrow:9', product: 'VIP', buyer: { name: null, username: null }, totalStars: 0, totalCredits: 1500, status: 'paid', createdAt: '2026-02-01T00:00:00.000Z' },
  'orderList': { ok: true, data: { items: [], total: 0 } },
  'orderAction': { ok: true, data: { id: 'escrow:9', status: 'delivered' } },
  'dashUserList': { ok: true, data: { items: [{ id: 'user:tg:1', name: 'Ali', username: 'ali', role: 'user', language: 'fa', createdAt: '2026-01-01T00:00:00.000Z' }] } },
  'dashWalletList': { ok: true, data: { items: [{ name: 'Ali', username: null, role: 'user', balance: 500, locked: 0, credits: 1500 }] } },
  'dashList': { ok: true, data: { items: [{ anything: 1, nested: { x: 'y' } }] } },
  'leaderboard': { ok: true, data: { items: [{ rank: 1, name: 'Ali', username: null, xp: 0, sales: 12, refs: 0 }] } },
  'approvalList': { ok: true, data: { items: [{ id: 'hitl:01', type: 'ESCROW_RELEASE', riskLevel: 'high', summary: 'Release 60000 Đ — escrow escrow:01', note: null, amount: 60000, status: 'pending', createdAt: '2026-02-01T00:00:00.000Z', expiresAt: '2026-02-02T00:00:00.000Z' }] } },
  'approvalDecision': { ok: true, data: { id: 'hitl:01', status: 'approved', escrowId: 'escrow:01', txId: 'tx:03' } },
  'settingsList': { ok: true, data: { items: [{ key: 'k', value: 'v' }] } },
  'security': { ok: true, data: { layers: [{ id: 'admin-key', name: 'Admin key', description: 'X-KIA-Key', active: true }], audit: [{ id: 'a:1', actor: 'admin', action: 'account.create', target: 'user:tg:1', status: 'ok', createdAt: '2026-02-01T00:00:00.000Z' }] } },
  // requests (what a TS client SENDS)
  'req:createAccount': { id: 'user:tg:123', label: 'Ali' },
  'req:mint': { accountId: 'user:tg:123', amount: 500, memo: 'welcome', idempotencyKey: 'mint-ali-1' },
  'req:burn': { accountId: 'user:tg:123', amount: 250 },
  'req:createEscrow': { id: 'escrow:01', buyerId: 'user:tg:1', sellerId: 'user:tg:2', amount: 60000, productRef: 'prod:1', ttlHours: 48, idempotencyKey: 'escrow-01' },
  'req:fund': {},
  'req:deliver': { deliveryRef: 'ref-9' },
  'req:release': { approvedByAdmin: true },
  'req:dispute': { reason: 'never arrived' },
  'req:refund': {},
  'req:cancel': {},
  'req:expire': {},
  'req:createHitl': { id: 'hitl:01', kind: 'ESCROW_RELEASE', escrowId: 'escrow:01', requesterId: 'user:tg:1', note: 'big', ttlHours: 24 },
  'req:hitlDecision': { decidedBy: 'user:admin:1', note: 'approved by council' },
  'req:aiChat': { userId: 'user:tg:123', initData: 'user=%7B%22id%22%3A123%7D&auth_date=1700000000&hash=ab', role: 'user', text: 'سلام', language: 'fa', history: [{ role: 'user', content: 'hi' }], knowledge: 'kb' },
  'req:createProduct': { title: 'VIP', category: 'digital', priceStars: 0, priceCredits: 1500, stock: 5, featured: true, description: 'd' },
  'req:updateProduct': { title: 'VIP2', active: false },
  'req:orderAction': { action: 'deliver' },
  'req:approvalDecision': { approve: true, note: 'go' },
  'req:settingsPut': { key: 'theme', value: 'dark' },
  // wire error envelope
  'wireError': { error: { code: 'ACCOUNT_OVERDRAFT', message: 'no funds', meta: { needed: 90 } } },
  'wireErrorNoMeta': { error: { code: 'ESCROW_NOT_FOUND', message: 'missing' } },
};

const SCHEMAS: Record<string, z.ZodType> = {
  'health': C.healthResponseSchema, 'economy': C.economySummarySchema,
  'accountView': C.accountViewSchema, 'accountList': C.accountListResponseSchema, 'balance': C.balanceResponseSchema,
  'walletMove': C.walletMoveResponseSchema,
  'escrowView': C.escrowViewSchema, 'escrowList': C.escrowListResponseSchema, 'escrowAction': C.escrowActionResponseSchema,
  'hitlView': C.hitlViewSchema, 'hitlList': C.hitlListResponseSchema, 'hitlSweep': C.hitlSweepResponseSchema, 'hitlAction': C.hitlActionResponseSchema,
  'backupCreated': C.backupCreatedSchema,
  'aiChat': C.aiChatResponseSchema, 'aiAgents': C.aiAgentsListResponseSchema, 'aiSkills': C.aiSkillsListResponseSchema, 'aiUsage': C.aiUsageViewSchema,
  'brand': C.brandResponseSchema, 'overview': C.overviewResponseSchema,
  'productView': C.productViewSchema, 'productList': C.productListResponseSchema, 'productMutation': C.productMutationResponseSchema,
  'orderView': C.orderViewSchema, 'orderList': C.orderListResponseSchema, 'orderAction': C.orderActionResponseSchema,
  'dashUserList': C.dashUserListResponseSchema, 'dashWalletList': C.dashWalletListResponseSchema, 'dashList': C.dashListResponseSchema,
  'leaderboard': C.leaderboardResponseSchema,
  'approvalList': C.approvalListResponseSchema, 'approvalDecision': C.approvalDecisionResponseSchema,
  'settingsList': C.settingsListResponseSchema, 'security': C.securityResponseSchema,
  'req:createAccount': C.createAccountRequestSchema, 'req:mint': C.mintRequestSchema, 'req:burn': C.burnRequestSchema,
  'req:createEscrow': C.createEscrowRequestSchema, 'req:fund': C.fundEscrowRequestSchema, 'req:deliver': C.deliverEscrowRequestSchema,
  'req:release': C.releaseEscrowRequestSchema, 'req:dispute': C.disputeEscrowRequestSchema, 'req:refund': C.refundEscrowRequestSchema,
  'req:cancel': C.cancelEscrowRequestSchema, 'req:expire': C.expireEscrowRequestSchema,
  'req:createHitl': C.createHitlRequestSchema, 'req:hitlDecision': C.hitlDecisionRequestSchema,
  'req:aiChat': C.aiChatRequestSchema,
  'req:createProduct': C.createProductRequestSchema, 'req:updateProduct': C.updateProductRequestSchema,
  'req:orderAction': C.orderActionRequestSchema, 'req:approvalDecision': C.approvalDecisionRequestSchema, 'req:settingsPut': C.settingsPutRequestSchema,
  'wireError': z.object({ error: z.object({ code: z.string(), message: z.string(), meta: z.record(z.string(), z.union([z.string(), z.number()])).optional() }) }),
  'wireErrorNoMeta': z.object({ error: z.object({ code: z.string(), message: z.string(), meta: z.record(z.string(), z.union([z.string(), z.number()])).optional() }) }),
};

for (const [name, schema] of Object.entries(SCHEMAS)) {
  const parsed = schema.safeParse(samples[name]);
  if (!parsed.success) {
    console.error(`SAMPLE INVALID: ${name}`);
    console.error(JSON.stringify(parsed.error.issues, null, 1).slice(0, 600));
    process.exit(1);
  }
}
console.log(`all ${Object.keys(SCHEMAS).length} samples valid per zod`);
writeFileSync('/tmp/ts-golden/golden-wire.json', JSON.stringify(samples, null, 1));

// quick stats for the report
const paths = Object.keys((doc as { paths: Record<string, unknown> }).paths);
console.log(`golden openapi: ${paths.length} paths, ${ROUTES.length} operations`);
console.log('done');
