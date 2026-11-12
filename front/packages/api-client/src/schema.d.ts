/**
 * GENERATED FILE — DO NOT EDIT.
 * Source of truth: rust/crates/contracts/tests/fixtures/golden-openapi.json
 * (pinned golden fixture, guarded by the contract's own golden tests).
 * Regenerate: pnpm --filter @agent-one/api-client run generate
 * Hand-written API URLs are banned elsewhere (D5 — pnpm lint:urls).
 */

export interface paths {
    "/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Liveness probe of the v3 server */
        get: operations["health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/economy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Nation economy summary — public transparency (charter Article 6) */
        get: operations["getEconomy"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all accounts with derived balances */
        get: operations["listAccounts"];
        put?: never;
        /** Register a user account (balance starts at 0) */
        post: operations["createAccount"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/accounts/{id}/balance": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Derived balance of one account (public transparency) */
        get: operations["getBalance"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/wallet/mint": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Mint dinar into a user account — yearly cap enforced (charter Article 3) */
        post: operations["mint"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/wallet/burn": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Burn dinar from a user account — overdraft-impossible (charter Article 4) */
        post: operations["burn"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List escrows (optionally filtered by status) */
        get: operations["listEscrows"];
        put?: never;
        /** Create an escrow (CREATED; money not moved yet) */
        post: operations["createEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Read one escrow (public transparency) */
        get: operations["getEscrow"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/fund": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Lock buyer money into the escrow account */
        post: operations["fundEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/deliver": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Mark the product as delivered (money still locked) */
        post: operations["deliverEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/release": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Settle: seller gets net (90%), treasury gets tax (10%) — HITL gate ≥ 50,000 Đ */
        post: operations["releaseEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/dispute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Open a dispute (escrow stays locked until arbitration) */
        post: operations["disputeEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/refund": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Arbitration in favor of the buyer — full refund, no tax */
        post: operations["refundEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Cancel a CREATED escrow — no money ever moved */
        post: operations["cancelEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/escrows/{id}/expire": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** TTL sweep — auto-refund a FUNDED escrow (charter Article 9) */
        post: operations["expireEscrow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/hitl": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List HITL requests (optionally filtered by status) */
        get: operations["listHitlRequests"];
        put?: never;
        /** Queue a human-approval request (amount copied from the escrow; duplicate id → replay) */
        post: operations["createHitlRequest"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/hitl/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Read one HITL request */
        get: operations["getHitlRequest"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/hitl/{id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Approve → ESCROW_RELEASE settles in the same transaction (seller 90% + treasury 10%) */
        post: operations["approveHitlRequest"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/hitl/{id}/reject": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Reject — no money moves; the escrow stays locked until arbitration decides otherwise */
        post: operations["rejectHitlRequest"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/hitl/sweep": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** TTL sweep — expire overdue PENDING requests (bots call this on an interval) */
        post: operations["sweepHitl"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/system/backup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Consistent SQLite snapshot (VACUUM INTO) into db/backups — prunes to the newest KEEP_BACKUPS files. Admin-gated. */
        post: operations["createBackup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ai/chat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Ask the Zenovix agent team — routing, skills, metering, redaction and audit run server-side */
        post: operations["aiChat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ai/agents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The 6 Zenovix agents with the skills each one may call */
        get: operations["aiAgents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ai/skills": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The 16 KIA-OS skills with authorization metadata */
        get: operations["aiSkills"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ai/usage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Per-user token usage against the daily cap (admin key required) */
        get: operations["aiUsage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/brand": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** White-label shell branding (BRAND_NAME / BRAND_TAGLINE env → defaults) */
        get: operations["getBrand"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/overview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** 24h KPIs + 14-day series + top products + recent orders, derived from live tables */
        get: operations["getOverview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/products": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Market listings for the command center (public read, q search, limit/offset) */
        get: operations["listDashProducts"];
        put?: never;
        /** Create a house listing (seller = treasury); Stars are not a ledger rail — credits only */
        post: operations["createDashProduct"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/products/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Archive a listing (soft — the row stays for ledger/escrow references) */
        delete: operations["archiveDashProduct"];
        options?: never;
        head?: never;
        /** Edit a listing (title/category/description/price/active; stock & stars are dashboard-only views) */
        patch: operations["updateDashProduct"];
        trace?: never;
    };
    "/orders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Escrow marketplace orders in dashboard shapes (status filter + paging) */
        get: operations["listDashOrders"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/orders/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** deliver → DELIVERED (from FUNDED) · cancel → CANCELLED (from CREATED) · refund → REFUNDED (from DISPUTED) */
        post: operations["dashOrderAction"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** User accounts (kind=user) for the wallets→users subtab */
        get: operations["listDashUsers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/wallets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Derived balances + escrow-locked amounts per user account */
        get: operations["listDashWallets"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/quests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Quest rail — empty until the quest engine ships (renders a clean empty state) */
        get: operations["listDashQuests"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/bookings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Bookings rail — empty until the booking engine ships (renders a clean empty state) */
        get: operations["listDashBookings"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/leaderboard": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** sales = settled escrows per seller (real); xp/ref rails ship with the quest engine */
        get: operations["dashLeaderboard"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/approvals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** HITL queue in dashboard shapes (risk level derives from the charter threshold) */
        get: operations["listDashApprovals"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/approvals/{id}/decide": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Approve settles the escrow in the same transaction; reject locks it for arbitration */
        post: operations["dashApprovalDecision"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Runtime settings — empty until the KV model ships (brand lives in env today) */
        get: operations["listDashSettings"];
        /** 501 until the KV model ships — honest refusal beats a silent no-op */
        put: operations["putDashSetting"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/security": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Live defense-in-depth layers (real config flags) + the audit trail tail */
        get: operations["dashSecurity"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: never;
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    health: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        status: "ok";
                        /** @constant */
                        service: "dropagentx-v3-server";
                        version: string;
                        llmConfigured: boolean;
                        llmModel: string | null;
                        db?: string;
                        bot?: string;
                        ai?: string;
                        uptime?: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getEconomy: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        currency: "DNX";
                        /** @constant */
                        symbol: "Đ";
                        circulatingSupply: number;
                        treasuryBalance: number;
                        escrowLocked: number;
                        mintedAllTime: number;
                        burnedAllTime: number;
                        mintedThisYear: number;
                        yearlyMintCap: number;
                        genesisSupply: number;
                        trialBalanceSum: number;
                        balanced: boolean;
                        accountCount: number;
                        openEscrows: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listAccounts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        items: {
                            id: string;
                            kind: string;
                            label: string | null;
                            balance: number;
                            createdAt: string;
                        }[];
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    createAccount: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    id: string;
                    label?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        id: string;
                        kind: string;
                        label: string | null;
                        balance: number;
                        createdAt: string;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getBalance: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        accountId: string;
                        balance: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    mint: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    accountId: string;
                    amount: number;
                    memo?: string;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        txId: string;
                        accountId: string;
                        balance: number;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    burn: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    accountId: string;
                    amount: number;
                    memo?: string;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        txId: string;
                        accountId: string;
                        balance: number;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listEscrows: {
        parameters: {
            query?: {
                status?: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        items: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        }[];
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    createEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    id?: string;
                    buyerId: string;
                    sellerId: string;
                    amount: number;
                    productRef?: string;
                    ttlHours?: number;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        id: string;
                        buyerId: string;
                        sellerId: string;
                        amount: number;
                        productRef: string | null;
                        /** @enum {string} */
                        status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                        createdAt: string;
                        fundedTxId: string | null;
                        settleTxId: string | null;
                        disputeReason: string | null;
                        deliveryRef: string | null;
                        expiresAt: string | null;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    fundEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    deliverEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    deliveryRef?: string;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    releaseEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    approvedByAdmin?: boolean;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    disputeEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    reason: string;
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    refundEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    cancelEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    expireEscrow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    idempotencyKey?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        };
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listHitlRequests: {
        parameters: {
            query?: {
                status?: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        items: {
                            id: string;
                            /** @enum {string} */
                            kind: "ESCROW_RELEASE";
                            escrowId: string;
                            amount: number;
                            requesterId: string;
                            note: string | null;
                            /** @enum {string} */
                            status: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
                            createdAt: string;
                            expiresAt: string;
                            decidedBy: string | null;
                            decidedAt: string | null;
                            decisionNote: string | null;
                        }[];
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    createHitlRequest: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    id?: string;
                    /** @enum {string} */
                    kind: "ESCROW_RELEASE";
                    escrowId: string;
                    requesterId: string;
                    note?: string;
                    ttlHours?: number;
                };
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        id: string;
                        /** @enum {string} */
                        kind: "ESCROW_RELEASE";
                        escrowId: string;
                        amount: number;
                        requesterId: string;
                        note: string | null;
                        /** @enum {string} */
                        status: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
                        createdAt: string;
                        expiresAt: string;
                        decidedBy: string | null;
                        decidedAt: string | null;
                        decisionNote: string | null;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getHitlRequest: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        id: string;
                        /** @enum {string} */
                        kind: "ESCROW_RELEASE";
                        escrowId: string;
                        amount: number;
                        requesterId: string;
                        note: string | null;
                        /** @enum {string} */
                        status: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
                        createdAt: string;
                        expiresAt: string;
                        decidedBy: string | null;
                        decidedAt: string | null;
                        decisionNote: string | null;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    approveHitlRequest: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    decidedBy: string;
                    note?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        request: {
                            id: string;
                            /** @enum {string} */
                            kind: "ESCROW_RELEASE";
                            escrowId: string;
                            amount: number;
                            requesterId: string;
                            note: string | null;
                            /** @enum {string} */
                            status: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
                            createdAt: string;
                            expiresAt: string;
                            decidedBy: string | null;
                            decidedAt: string | null;
                            decisionNote: string | null;
                        };
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        } | null;
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    rejectHitlRequest: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    decidedBy: string;
                    note?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        request: {
                            id: string;
                            /** @enum {string} */
                            kind: "ESCROW_RELEASE";
                            escrowId: string;
                            amount: number;
                            requesterId: string;
                            note: string | null;
                            /** @enum {string} */
                            status: "PENDING" | "APPROVED" | "REJECTED" | "EXPIRED";
                            createdAt: string;
                            expiresAt: string;
                            decidedBy: string | null;
                            decidedAt: string | null;
                            decisionNote: string | null;
                        };
                        escrow: {
                            id: string;
                            buyerId: string;
                            sellerId: string;
                            amount: number;
                            productRef: string | null;
                            /** @enum {string} */
                            status: "CREATED" | "FUNDED" | "DELIVERED" | "RELEASED" | "DISPUTED" | "REFUNDED" | "CANCELLED" | "EXPIRED";
                            createdAt: string;
                            fundedTxId: string | null;
                            settleTxId: string | null;
                            disputeReason: string | null;
                            deliveryRef: string | null;
                            expiresAt: string | null;
                        } | null;
                        txId: string | null;
                        replayed: boolean;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    sweepHitl: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        expired: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    createBackup: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        file: string;
                        sizeBytes: number;
                        kept: number;
                        createdAt: string;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    aiChat: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    userId: string;
                    initData?: string;
                    /**
                     * @default user
                     * @enum {string}
                     */
                    role?: "user" | "staff";
                    text: string;
                    /** @default fa */
                    language?: string;
                    /** @enum {string} */
                    directAgent?: "orchestrator" | "receptionist" | "sales" | "support" | "analyst" | "content";
                    history?: {
                        /** @enum {string} */
                        role: "user" | "assistant";
                        content: string;
                    }[];
                    knowledge?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        reply: string;
                        /** @enum {string} */
                        agent: "orchestrator" | "receptionist" | "sales" | "support" | "analyst" | "content";
                        /** @enum {string} */
                        via: "direct" | "prefilter" | "llm";
                        provider: string;
                        toolTraces: {
                            skill: string;
                            ok: boolean;
                            summary: string;
                            pendingApproval: boolean;
                            code: string | null;
                        }[];
                        usage: {
                            provider: string | null;
                            model: string | null;
                            promptTokens: number;
                            completionTokens: number;
                            totalTokens: number;
                        };
                        pendingApproval: boolean;
                        redactions: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    aiAgents: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        items: {
                            /** @enum {string} */
                            name: "orchestrator" | "receptionist" | "sales" | "support" | "analyst" | "content";
                            emoji: string;
                            titleFa: string;
                            titleEn: string;
                            description: string;
                            skills: string[];
                        }[];
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    aiSkills: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        items: {
                            id: string;
                            /** @enum {string} */
                            kind: "read" | "write" | "money" | "staff";
                            titleFa: string;
                            titleEn: string;
                            description: string;
                            allowedAgents: ("orchestrator" | "receptionist" | "sales" | "support" | "analyst" | "content")[];
                            staffOnly: boolean;
                        }[];
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    aiUsage: {
        parameters: {
            query?: {
                userId?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        userId: string;
                        usedToday: number;
                        cap: number;
                        remainingToday: number;
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getBrand: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            name: string;
                            tagline: string;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    getOverview: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            users: number;
                            products: number;
                            orders: number;
                            revenueStars: number;
                            revenueCredits: number;
                            pendingApprovals: number;
                            aiCalls: number;
                            series: {
                                date: string;
                                revenue: number;
                                orders: number;
                            }[];
                            topProducts: {
                                title: string;
                                salesCount: number;
                            }[];
                            recentOrders: {
                                id: string;
                                title: string | null;
                                buyer: {
                                    name: string | null;
                                    username: string | null;
                                };
                                totalStars: number;
                                totalCredits: number;
                                status: string;
                                /** Format: date-time */
                                createdAt: string;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashProducts: {
        parameters: {
            query?: {
                limit?: number;
                offset?: number;
                q?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                id: string;
                                sellerId: string;
                                title: string;
                                description: string;
                                category: string;
                                priceStars: number;
                                priceCredits: number;
                                stock: number;
                                salesCount: number;
                                featured: boolean;
                                /** @enum {string} */
                                status: "active" | "archived";
                                /** Format: date-time */
                                createdAt: string;
                            }[];
                            total: number;
                            limit: number;
                            offset: number;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    createDashProduct: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    title: string;
                    category?: string;
                    priceStars?: number;
                    priceCredits?: number;
                    stock?: number;
                    featured?: boolean;
                    description?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            id: string;
                            sellerId: string;
                            title: string;
                            description: string;
                            category: string;
                            priceStars: number;
                            priceCredits: number;
                            stock: number;
                            salesCount: number;
                            featured: boolean;
                            /** @enum {string} */
                            status: "active" | "archived";
                            /** Format: date-time */
                            createdAt: string;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    archiveDashProduct: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            id: string;
                            sellerId: string;
                            title: string;
                            description: string;
                            category: string;
                            priceStars: number;
                            priceCredits: number;
                            stock: number;
                            salesCount: number;
                            featured: boolean;
                            /** @enum {string} */
                            status: "active" | "archived";
                            /** Format: date-time */
                            createdAt: string;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    updateDashProduct: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    title?: string;
                    category?: string;
                    priceStars?: number;
                    priceCredits?: number;
                    stock?: number;
                    featured?: boolean;
                    description?: string;
                    active?: boolean;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            id: string;
                            sellerId: string;
                            title: string;
                            description: string;
                            category: string;
                            priceStars: number;
                            priceCredits: number;
                            stock: number;
                            salesCount: number;
                            featured: boolean;
                            /** @enum {string} */
                            status: "active" | "archived";
                            /** Format: date-time */
                            createdAt: string;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashOrders: {
        parameters: {
            query?: {
                limit?: number;
                offset?: number;
                status?: "all" | "paid" | "pending" | "delivered" | "refunded" | "cancelled";
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                id: string;
                                product: string;
                                buyer: {
                                    name: string | null;
                                    username: string | null;
                                };
                                totalStars: number;
                                totalCredits: number;
                                status: string;
                                /** Format: date-time */
                                createdAt: string;
                            }[];
                            total: number;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    dashOrderAction: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    /** @enum {string} */
                    action: "deliver" | "cancel" | "refund";
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            id: string;
                            status: string;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashUsers: {
        parameters: {
            query?: {
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                id: string;
                                name: string;
                                username: string | null;
                                role: string;
                                language: string | null;
                                /** Format: date-time */
                                createdAt: string;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashWallets: {
        parameters: {
            query?: {
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                name: string;
                                username: string | null;
                                role: string;
                                balance: number;
                                locked: number;
                                credits: number;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashQuests: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                [key: string]: unknown;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashBookings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                [key: string]: unknown;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    dashLeaderboard: {
        parameters: {
            query?: {
                type?: "xp" | "sales" | "ref";
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                rank: number;
                                name: string;
                                username: string | null;
                                xp: number;
                                sales: number;
                                refs: number;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashApprovals: {
        parameters: {
            query?: {
                status?: "pending" | "approved" | "rejected" | "expired" | "all";
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                id: string;
                                type: string;
                                /** @enum {string} */
                                riskLevel: "low" | "medium" | "high";
                                summary: string;
                                note: string | null;
                                amount: number;
                                status: string;
                                /** Format: date-time */
                                createdAt: string;
                                expiresAt: string | null;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    dashApprovalDecision: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    approve: boolean;
                    note?: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            id: string;
                            status: string;
                            escrowId: string | null;
                            txId: string | null;
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    listDashSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                key: string;
                                value: string;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    putDashSetting: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    key: string;
                    value: string;
                };
            };
        };
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            items: {
                                key: string;
                                value: string;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
    dashSecurity: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Success */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @constant */
                        ok: true;
                        data: {
                            layers: {
                                id: string;
                                name: string;
                                description?: string;
                                active: boolean;
                            }[];
                            audit: {
                                id: string;
                                actor: string;
                                action: string;
                                target: string | null;
                                status: string;
                                /** Format: date-time */
                                createdAt: string;
                            }[];
                        };
                    };
                };
            };
            /** @description Domain or validation failure */
            "4XX": {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        error: {
                            code: string;
                            message: string;
                            meta?: {
                                [key: string]: string | number;
                            };
                        };
                    };
                };
            };
        };
    };
}
