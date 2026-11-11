//! ADR-003 — دفتر دوطرفه تغییرناپذیر (هسته عملیاتی خالص).
//!
//! قراردادها:
//!  - مقدار ردیف > 0 → بستانکار (ورود به حساب)، مقدار < 0 → بدهکار (خروج از حساب).
//!  - هر تراکنش حداقل ۲ ردیف دارد و مجموع‌شان **همیشه** صفر است.
//!  - قواعد جهت هر نوع حساب (fail-closed):
//!    user/escrow/treasury → تراز هرگز منفی نمی‌شود،
//!    mint → تراز هرگز مثبت نمی‌شود (فقط عرضه صادر می‌کند)،
//!    burn → تراز هرگز منفی نمی‌شود (فقط پول نابودشده می‌گیرد).
//!  - idempotencyKey: پست دوباره با همان کلید = پخش مجدد no-op و برگشت
//!    تراکنش اصلی (امن برای retryهای بات/API).
//!  - حفاظت (تقدسی): Σ تراز همه حساب‌ها = 0، همیشه.
//!
//! تغییرناپذیری: هر عملیات یک `LedgerState` **جدید** برمی‌گرداند
//! (clone-on-write) — خطا هرگز وضعیت را خراب نمی‌کند.

use crate::errors::{DomainError, DomainErrorCode, DomainResult, MetaValue};
use crate::events::EPOCH_ISO;
use crate::money::MAX_DIN;
use std::collections::BTreeMap;

/// نوع حساب — جهت‌گیری قواعد fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccountKind {
    /// حساب شهروند — هرگز منفی نمی‌شود
    User,
    /// حساب امانت — هرگز منفی نمی‌شود
    Escrow,
    /// حساب خزانه کشور — هرگز منفی نمی‌شود
    Treasury,
    /// حساب ضرب — هرگز مثبت نمی‌شود (فقط عرضه صادر می‌کند)
    Mint,
    /// حساب سوزاندن — هرگز منفی نمی‌شود (فقط پول نابود می‌گیرد)
    Burn,
}

impl AccountKind {
    /// رشته کانانیکی — یکسان با نسخه TS.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Escrow => "escrow",
            Self::Treasury => "treasury",
            Self::Mint => "mint",
            Self::Burn => "burn",
        }
    }
}

impl std::fmt::Display for AccountKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// حساب در دفتر.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// شناسه یکتا (مثل `user:buyer` یا `escrow:e1`)
    pub id: String,
    /// نوع حساب
    pub kind: AccountKind,
    /// برچسب اختیاری (مثلاً نام فارسی)
    pub label: Option<String>,
}

/// یک ردیف تراکنش — مقدار علامت‌دار صحیح.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    /// حساب هدف
    pub account_id: String,
    /// مقدار علامت‌دار (بستانکار > 0، بدهکار < 0)
    pub amount: i64,
}

impl Posting {
    /// میان‌بر ساخت ردیف.
    #[must_use]
    pub fn new(account_id: impl Into<String>, amount: i64) -> Self {
        Self { account_id: account_id.into(), amount }
    }
}

/// تراکنش ثبت‌شده.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostedTransaction {
    /// شناسه یکتا (`tx_N` یا صریح)
    pub id: String,
    /// کلید یکسان‌سازی اختیاری
    pub idempotency_key: Option<String>,
    /// ردیف‌ها (کپی مستقل)
    pub postings: Vec<Posting>,
    /// یادداشت اختیاری
    pub memo: Option<String>,
    /// زمان ISO-8601 (فراهم‌شده توسط فراخوان/آداپتور — دامنه ساعت ندارد)
    pub at: String,
}

/// وضعیت کامل دفتر — تغییرناپذیر از دید فراخوان.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerState {
    /// حساب‌ها بر اساس شناسه
    pub accounts: BTreeMap<String, Account>,
    /// ترازها بر اساس شناسه
    pub balances: BTreeMap<String, i64>,
    /// تراکنش‌های ثبت‌شده
    pub txs: BTreeMap<String, PostedTransaction>,
    /// کلید یکسان‌سازی → شناسه تراکنش
    pub idempotency: BTreeMap<String, String>,
}

/// حساب‌های سیستمی شناخته‌شده (قرارداد مشترک با آداپتورها).
pub const MINT_ACCOUNT_ID: &str = "mint:genesis";
/// حساب خزانه کشور.
pub const TREASURY_ACCOUNT_ID: &str = "treasury:nation";
/// حساب سوزاندن.
pub const BURN_ACCOUNT_ID: &str = "burn:void";

/// دفتر خالی باز می‌کند.
#[must_use]
pub fn open_ledger() -> LedgerState {
    LedgerState::default()
}

/// افزودن حساب — شناسه تکراری رد می‌شود.
///
/// # Errors
/// `DUPLICATE_ACCOUNT` اگر حساب وجود داشته باشد.
pub fn add_account(state: &LedgerState, account: Account) -> DomainResult<LedgerState> {
    if state.accounts.contains_key(&account.id) {
        return Err(DomainError::new(
            DomainErrorCode::DuplicateAccount,
            format!("Account {} already exists", account.id),
        ));
    }
    let mut accounts = state.accounts.clone();
    let id = account.id.clone();
    accounts.insert(id.clone(), account);
    let mut balances = state.balances.clone();
    balances.insert(id, 0);
    Ok(LedgerState {
        accounts,
        balances,
        txs: state.txs.clone(),
        idempotency: state.idempotency.clone(),
    })
}

/// فرمان ثبت تراکنش.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostCommand {
    /// شناسه صریح اختیاری (پیش‌فرض: `tx_N`)
    pub id: Option<String>,
    /// کلید یکسان‌سازی اختیاری
    pub idempotency_key: Option<String>,
    /// ردیف‌ها (≥ ۲ و مجموع صفر)
    pub postings: Vec<Posting>,
    /// یادداشت اختیاری
    pub memo: Option<String>,
    /// زمان ISO اختیاری — پیش‌فرض epoch دترمینیستیک
    pub at: Option<String>,
}

/// نتیجه ثبت تراکنش.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostOutcome {
    /// وضعیت جدید دفتر (در replay، همان وضعیت ورودی)
    pub state: LedgerState,
    /// تراکنش ثبت‌شده (در replay، تراکنش اصلی)
    pub tx: PostedTransaction,
    /// آیا این یک پخش مجدد (replay) بود؟
    pub replayed: bool,
}

/// نقض قاعده جهت — `mint` مثبت نمی‌شود؛ بقیه منفی نمی‌شوند.
#[must_use]
pub const fn is_overdraft_violation(kind: AccountKind, balance: i64) -> bool {
    match kind {
        AccountKind::Mint => balance > 0,
        AccountKind::Burn | AccountKind::User | AccountKind::Escrow | AccountKind::Treasury => {
            balance < 0
        }
    }
}

/// ثبت تراکنش — چهار گام: replay → شکل → توازن → قواعد جهت، سپس commit.
///
/// # Errors
/// طبق کدهای دامنه: `TOO_FEW_POSTINGS`، `NON_INTEGER_AMOUNT`،
/// `UNKNOWN_ACCOUNT`، `UNBALANCED_TRANSACTION`، `ACCOUNT_OVERDRAFT`.
pub fn post(state: &LedgerState, cmd: &PostCommand) -> DomainResult<PostOutcome> {
    // ۱) پخش مجدد یکسان‌سازی‌شده — برگرداندن تراکنش اصلی مجاز است، خطا نیست.
    if let Some(key) = &cmd.idempotency_key {
        if let Some(existing_id) = state.idempotency.get(key) {
            if let Some(existing) = state.txs.get(existing_id) {
                return Ok(PostOutcome {
                    state: state.clone(),
                    tx: existing.clone(),
                    replayed: true,
                });
            }
        }
    }

    // ۲) اعتبارسنجی شکل.
    if cmd.postings.len() < 2 {
        return Err(DomainError::new(
            DomainErrorCode::TooFewPostings,
            format!("A transaction needs at least 2 postings, got {}", cmd.postings.len()),
        ));
    }
    for p in &cmd.postings {
        // مرز امن TS: |amount| ≤ 2^53−1
        if p.amount.unsigned_abs() > MAX_DIN as u64 {
            return Err(DomainError::new(
                DomainErrorCode::NonIntegerAmount,
                format!("Posting to {} must be a safe integer, got {}", p.account_id, p.amount),
            ));
        }
        if !state.accounts.contains_key(&p.account_id) {
            return Err(DomainError::new(
                DomainErrorCode::UnknownAccount,
                format!("Unknown account {}", p.account_id),
            ));
        }
    }
    let mut sum: i64 = 0;
    for p in &cmd.postings {
        sum = match sum.checked_add(p.amount) {
            Some(s) => s,
            None => {
                return Err(DomainError::new(
                    DomainErrorCode::UnbalancedTransaction,
                    "Postings must sum to zero (overflow while summing)",
                )
                .with_meta("sum", MetaValue::Text("overflow".into())));
            }
        };
    }
    if sum != 0 {
        return Err(DomainError::new(
            DomainErrorCode::UnbalancedTransaction,
            format!("Postings must sum to zero, got {sum}"),
        )
        .with_meta("sum", sum));
    }

    // ۳) قواعد جهت (fail-closed) — روی پیش‌نویس ارزیابی؛ فقط اگر تمیز بود commit.
    let mut draft = state.balances.clone();
    for p in &cmd.postings {
        *draft.entry(p.account_id.clone()).or_insert(0) += p.amount;
    }
    for (account_id, new_balance) in &draft {
        if let Some(account) = state.accounts.get(account_id) {
            if is_overdraft_violation(account.kind, *new_balance) {
                return Err(DomainError::new(
                    DomainErrorCode::AccountOverdraft,
                    format!(
                        "Account {} ({}) cannot go to {}",
                        account_id, account.kind, new_balance
                    ),
                )
                .with_meta("accountId", account_id.clone())
                .with_meta("kind", account.kind.as_str())
                .with_meta("newBalance", *new_balance));
            }
        }
    }

    // ۴) Commit — وضعیت جدید تغییرناپذیر.
    let tx_id = cmd.id.clone().unwrap_or_else(|| format!("tx_{}", state.txs.len() + 1));
    let tx = PostedTransaction {
        id: tx_id,
        idempotency_key: cmd.idempotency_key.clone(),
        postings: cmd.postings.clone(),
        memo: cmd.memo.clone(),
        at: cmd.at.clone().unwrap_or_else(|| EPOCH_ISO.to_owned()),
    };
    let mut txs = state.txs.clone();
    txs.insert(tx.id.clone(), tx.clone());
    let mut idempotency = state.idempotency.clone();
    if let Some(key) = &cmd.idempotency_key {
        idempotency.insert(key.clone(), tx.id.clone());
    }
    Ok(PostOutcome {
        state: LedgerState { accounts: state.accounts.clone(), balances: draft, txs, idempotency },
        tx,
        replayed: false,
    })
}

/// تراز یک حساب.
///
/// # Errors
/// `UNKNOWN_ACCOUNT` اگر حساب وجود نداشته باشد.
pub fn balance_of(state: &LedgerState, account_id: &str) -> DomainResult<i64> {
    state.balances.get(account_id).copied().ok_or_else(|| {
        DomainError::new(DomainErrorCode::UnknownAccount, format!("Unknown account {account_id}"))
    })
}

/// سطر تراز آزمایشی.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalanceRow {
    /// شناسه حساب
    pub account_id: String,
    /// نوع حساب
    pub kind: AccountKind,
    /// تراز
    pub balance: i64,
}

/// تراز آزمایشی — تقدسی: `balanced == true` همیشه.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalance {
    /// سطرها (مرتب بر اساس شناسه — دترمینیستیک)
    pub rows: Vec<TrialBalanceRow>,
    /// Σ همه ترازها
    pub sum: i64,
    /// آیا Σ = 0؟
    pub balanced: bool,
}

/// محاسبه تراز آزمایشی.
#[must_use]
pub fn trial_balance(state: &LedgerState) -> TrialBalance {
    let mut rows = Vec::new();
    let mut sum: i64 = 0;
    for (account_id, balance) in &state.balances {
        if let Some(account) = state.accounts.get(account_id) {
            rows.push(TrialBalanceRow {
                account_id: account_id.clone(),
                kind: account.kind,
                balance: *balance,
            });
            sum += balance;
        }
    }
    TrialBalance { sum, balanced: sum == 0, rows }
}

/// پول در دست شهروندان (user + escrow + treasury) = صادرشده − سوزانده‌شده.
#[must_use]
pub fn circulating_supply(state: &LedgerState) -> i64 {
    let mut total: i64 = 0;
    for (account_id, balance) in &state.balances {
        if let Some(account) = state.accounts.get(account_id) {
            if matches!(
                account.kind,
                AccountKind::User | AccountKind::Escrow | AccountKind::Treasury
            ) {
                total += balance;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINT: &str = MINT_ACCOUNT_ID;
    const TREASURY: &str = TREASURY_ACCOUNT_ID;
    const BURN: &str = BURN_ACCOUNT_ID;

    fn new_ledger() -> LedgerState {
        let l = open_ledger();
        let l = add_account(&l, Account { id: MINT.into(), kind: AccountKind::Mint, label: None })
            .unwrap();
        let l = add_account(
            &l,
            Account { id: TREASURY.into(), kind: AccountKind::Treasury, label: None },
        )
        .unwrap();
        let l = add_account(&l, Account { id: BURN.into(), kind: AccountKind::Burn, label: None })
            .unwrap();
        let l = add_account(
            &l,
            Account {
                id: "user:buyer".into(),
                kind: AccountKind::User,
                label: Some("خریدار".into()),
            },
        )
        .unwrap();
        let l = add_account(
            &l,
            Account {
                id: "user:seller".into(),
                kind: AccountKind::User,
                label: Some("فروشنده".into()),
            },
        )
        .unwrap();
        add_account(&l, Account { id: "user:c".into(), kind: AccountKind::User, label: None })
            .unwrap()
    }

    fn mint_to(state: &LedgerState, account_id: &str, amount: i64) -> LedgerState {
        post(
            state,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new(MINT, -amount), Posting::new(account_id, amount)],
                memo: Some(format!("mint {amount} → {account_id}")),
                at: None,
            },
        )
        .unwrap()
        .state
    }

    #[test]
    fn opens_empty_and_keeps_accounts_unique() {
        let l = new_ledger();
        assert_eq!(trial_balance(&l).sum, 0);
        let dup =
            add_account(&l, Account { id: MINT.into(), kind: AccountKind::Mint, label: None });
        assert!(dup.is_err());
        assert_eq!(dup.unwrap_err().code, DomainErrorCode::DuplicateAccount);
    }

    #[test]
    fn mints_supply_and_keeps_conservation() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 1_000);
        assert_eq!(balance_of(&l, "user:buyer").unwrap(), 1_000);
        assert_eq!(balance_of(&l, MINT).unwrap(), -1_000);
        let tb = trial_balance(&l);
        assert!(tb.balanced);
        assert_eq!(circulating_supply(&l), 1_000);
    }

    #[test]
    fn rejects_unbalanced_transactions() {
        let l = new_ledger();
        let r = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", 10), Posting::new("user:seller", 5)],
                memo: None,
                at: None,
            },
        );
        let e = r.unwrap_err();
        assert_eq!(e.code, DomainErrorCode::UnbalancedTransaction);
        assert_eq!(e.meta.get("sum"), Some(&MetaValue::Int(15)));
    }

    #[test]
    fn rejects_single_posting_transactions() {
        let l = new_ledger();
        let r = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", 10)],
                memo: None,
                at: None,
            },
        );
        assert_eq!(r.unwrap_err().code, DomainErrorCode::TooFewPostings);
    }

    #[test]
    fn rejects_unknown_accounts_and_out_of_range_postings() {
        let l = new_ledger();
        let unknown = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -10), Posting::new("user:ghost", 10)],
                memo: None,
                at: None,
            },
        );
        assert_eq!(unknown.unwrap_err().code, DomainErrorCode::UnknownAccount);
        // در TS پست اعشاری رد می‌شود؛ در Rust همان کد برای خروج از مرز امن است
        let beyond = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![
                    Posting::new("user:buyer", -9_007_199_254_740_992),
                    Posting::new("user:seller", 9_007_199_254_740_992),
                ],
                memo: None,
                at: None,
            },
        );
        assert_eq!(beyond.unwrap_err().code, DomainErrorCode::NonIntegerAmount);
    }

    #[test]
    fn never_lets_a_user_go_negative() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 500);
        let r = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -600), Posting::new("user:seller", 600)],
                memo: None,
                at: None,
            },
        );
        let e = r.unwrap_err();
        assert_eq!(e.code, DomainErrorCode::AccountOverdraft);
        assert_eq!(e.meta.get("newBalance"), Some(&MetaValue::Int(-100)));
        // وضعیت پس از خطا دست‌نخورده
        assert_eq!(balance_of(&l, "user:buyer").unwrap(), 500);
        assert_eq!(balance_of(&l, "user:seller").unwrap(), 0);
    }

    #[test]
    fn enforces_direction_rules_mint_cannot_gain_burn_cannot_lose() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 1_000);
        assert_eq!(balance_of(&l, MINT).unwrap(), -1_000);
        // بستانکاری که mint را بالای صفر ببرد رد می‌شود
        let bad_mint = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -1_100), Posting::new(MINT, 1_100)],
                memo: None,
                at: None,
            },
        );
        let e = bad_mint.unwrap_err();
        assert_eq!(e.code, DomainErrorCode::AccountOverdraft);
        assert_eq!(e.meta.get("accountId"), Some(&MetaValue::Text(MINT.into())));
        // دریافت توسط burn مجاز است
        let burn_ok = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -100), Posting::new(BURN, 100)],
                memo: None,
                at: None,
            },
        );
        assert!(burn_ok.is_ok());
        // ماتریس قواعد
        assert!(is_overdraft_violation(AccountKind::Mint, 1));
        assert!(is_overdraft_violation(AccountKind::Burn, -1));
        assert!(is_overdraft_violation(AccountKind::User, -1));
        assert!(is_overdraft_violation(AccountKind::Escrow, -1));
        assert!(is_overdraft_violation(AccountKind::Treasury, -1));
        assert!(!is_overdraft_violation(AccountKind::Mint, -1));
        assert!(!is_overdraft_violation(AccountKind::Burn, 1));
    }

    #[test]
    fn replays_idempotently_same_key_same_tx_state_untouched() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 500);
        let cmd = PostCommand {
            id: None,
            idempotency_key: Some("order:42:settle".into()),
            postings: vec![Posting::new("user:buyer", -100), Posting::new("user:seller", 100)],
            memo: Some("pay order 42".into()),
            at: None,
        };
        let first = post(&l, &cmd).unwrap();
        assert!(!first.replayed);
        let second = post(&first.state, &cmd).unwrap();
        assert!(second.replayed);
        assert_eq!(second.tx.id, first.tx.id);
        assert_eq!(second.state, first.state); // وضعیت تغییرناپذیر یکسان
        assert_eq!(balance_of(&second.state, "user:buyer").unwrap(), 400);
        assert_eq!(balance_of(&second.state, "user:seller").unwrap(), 100);
    }

    #[test]
    fn does_not_dedupe_postings_without_idempotency_key() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 500);
        let cmd = PostCommand {
            id: None,
            idempotency_key: None,
            postings: vec![Posting::new("user:buyer", -10), Posting::new("user:seller", 10)],
            memo: None,
            at: None,
        };
        l = post(&l, &cmd).unwrap().state;
        l = post(&l, &cmd).unwrap().state;
        assert_eq!(balance_of(&l, "user:seller").unwrap(), 20);
    }

    #[test]
    fn assigns_unique_transaction_ids() {
        let mut l = new_ledger();
        l = mint_to(&l, "user:buyer", 100);
        let r1 = post(
            &l,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -1), Posting::new("user:seller", 1)],
                memo: None,
                at: None,
            },
        )
        .unwrap();
        let r2 = post(
            &r1.state,
            &PostCommand {
                id: None,
                idempotency_key: None,
                postings: vec![Posting::new("user:buyer", -1), Posting::new("user:seller", 1)],
                memo: None,
                at: None,
            },
        )
        .unwrap();
        assert_ne!(r1.tx.id, r2.tx.id);
        assert_ne!(r2.tx.id, "tx_1");
    }

    /// PRNG دترمینیستیک — پورت دقیق mulberry32 تا با نسخه TS یکسان بماند.
    fn mulberry32(seed: u32) -> impl FnMut() -> f64 {
        let mut t = seed;
        move || {
            t = t.wrapping_add(0x6d2b79f5);
            let mut x = t;
            x = (x ^ (x >> 15)).wrapping_mul(x | 1);
            x ^= x.wrapping_add((x ^ (x >> 7)).wrapping_mul(x | 61));
            f64::from(x ^ (x >> 14)) / 4294967296.0
        }
    }

    #[test]
    fn survives_300_seeded_random_operations_with_conservation_intact() {
        let mut l = new_ledger();
        for u in ["user:buyer", "user:seller", "user:c"] {
            l = mint_to(&l, u, 300);
        }
        let mut rand = mulberry32(1405);
        let users = ["user:buyer", "user:seller", "user:c"];
        let mut minted_extra: i64 = 0;
        let mut burned_total: i64 = 0;
        for i in 0..300 {
            let from = users[(rand() * users.len() as f64) as usize];
            let mut to = users[(rand() * users.len() as f64) as usize];
            if to == from {
                to = "user:seller";
            }
            let amount = 1 + (rand() * 50.0) as i64;
            let roll = rand();
            let postings = if roll < 0.6 {
                vec![Posting::new(from, -amount), Posting::new(to, amount)]
            } else if roll < 0.8 {
                vec![Posting::new(from, -amount), Posting::new(BURN, amount)]
            } else {
                vec![Posting::new(MINT, -amount), Posting::new(to, amount)]
            };
            let r = post(
                &l,
                &PostCommand {
                    id: None,
                    idempotency_key: None,
                    postings,
                    memo: Some(format!("seeded op {i}")),
                    at: None,
                },
            );
            match r {
                Ok(out) => {
                    l = out.state;
                    assert!(trial_balance(&l).balanced, "op {i} broke conservation");
                    if roll >= 0.8 {
                        minted_extra += amount;
                    }
                    if (0.6..0.8).contains(&roll) {
                        burned_total += amount;
                    }
                }
                Err(e) => {
                    // عملیات ناموفق فقط خطای سیاست است، هرگز شکاف حفاظت
                    assert!(
                        matches!(
                            e.code,
                            DomainErrorCode::AccountOverdraft
                                | DomainErrorCode::UnknownAccount
                                | DomainErrorCode::UnbalancedTransaction
                        ),
                        "op {i}: {}",
                        e.code
                    );
                }
            }
        }
        assert!(trial_balance(&l).balanced);
        assert_eq!(circulating_supply(&l), 900 + minted_extra - burned_total);
    }
}
