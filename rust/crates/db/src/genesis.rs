//! برنامهٔ genesis — محاسبهٔ «خالص» مهاجرت لگاسی (بدون IO) + ثابت‌های قرارداد.
//!
//! اعداد روز-اول (آدیت genesis، مهر ۱۴۰۵ — هر وقت خواستی دوباره راستی‌آزمایی کن):
//!   ۴۶۶ کاربر · ۱٬۵۵۴ تراکنش · Σ ترازهای ذخیره‌شده 187,672.8 Đ
//!   دفتر لگاسی 187,735.8 را تلمیح می‌کرد → گپ −63.0 صددرصد توضیح‌داده‌شده:
//!   ۹ خرید ۵۰-کردیتی بدون ردیف تراکنش (−450) + یک شارژ دستی مالک (+387).
//!
//! بازنمایی v3 (ADR-002/003 — پول صحیح، دوطرفه):
//!  - یک حساب برای هر کاربر لگاسی: `user:tg:<id>` (هر ۴۶۶ نفر، ban شده یا نه)
//!  - فقط «یک» تراکنش genesis با کلید یکسان‌سازی `legacy:genesis:v1`:
//!    mint:genesis  −Σ(floor(credits))     (ضرب هرگز مثبت نمی‌شود)
//!    user_i        +floor(credits_i)      (گردکردن صریح هر شهروند)
//!    یادداشت genesis خودش ردیف تعدیل شفاف است (تصمیم #۵): دقیقاً می‌گوید
//!    چرا دفتر از ۱۸۷٬۶۷۲ شروع می‌شود نه ۱۸۷٬۷۳۵.

use dropagentx_domain::Din;

/// کلید یکسان‌سازی genesis — مصرف‌یک‌بار با قید UNIQUE.
pub const GENESIS_KEY: &str = "legacy:genesis:v1";

/// شناسه ثابت تراکنش genesis — برای گزارش‌ها و ممیزی.
pub const GENESIS_TX_ID: &str = "tx_legacy_genesis";

/// عرضهٔ مورد انتظار پس از مهاجرت — عدد پذیرش فاز ۲ (ADR-009).
pub const EXPECTED_SUPPLY: i64 = 187_672;

/// یک کاربر جدول `users` دیتابیس لگاسی (خام، بدون تفسیر).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyUserRow {
    /// شناسه TEXT — شناسه‌های لگاسی از Int32 بزرگ‌ترند (مثل 5010703739)
    pub user_id: String,
    /// نام کاربری اختیاری
    pub username: Option<String>,
    /// نام اختیاری
    pub first_name: Option<String>,
    /// اعتبار اعشاری لگاسی (REAL)
    pub credits: f64,
    /// ۱ = ban شده
    pub is_banned: i64,
}

/// یک محصول جدول `products` دیتابیس لگاسی (خام).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyProductRow {
    /// شناسه لگاسی (TEXT)
    pub id: String,
    /// فروشنده (TEXT)
    pub creator_id: String,
    /// عنوان
    pub title: String,
    /// توضیح اختیاری
    pub description: Option<String>,
    /// قیمت اعشاری لگاسی
    pub price_credits: f64,
    /// دسته اختیاری
    pub category: Option<String>,
    /// لینک تحویل — تنها محتوای تحویل که از Railway نجات آمد
    pub link: Option<String>,
    /// ۱ = فعال
    pub is_active: i64,
}

/// یک شهروند در برنامهٔ genesis — حساب + گردکردن صریح ADR-002.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisCitizen {
    /// شناسه حساب — `user:tg:<id>`
    pub account_id: String,
    /// برچسب — username ?? first_name ?? `tg:<id>`
    pub label: String,
    /// مبلغ گردشده به پایین (Din)
    pub din: i64,
}

/// برنامهٔ کامل genesis — خروجی محاسبهٔ خالص؛ اجرایش با `migrate_legacy`.
#[derive(Debug, Clone, PartialEq)]
pub struct GenesisPlan {
    /// همهٔ شهروندان (۴۶۶ — حتی صفرها؛ فقط holders ردیف می‌گیرند)
    pub citizens: Vec<GenesisCitizen>,
    /// Σ اعتبارهای اعشاری لگاسی (مثل 187,672.8)
    pub sum_credits: f64,
    /// Σ floor هر شهروند = عرضهٔ ضرب‌شده (۱۸۷٬۶۷۲)
    pub sum_din: i64,
    /// تعداد کیف‌های دارای موجودی
    pub holders: usize,
    /// تعداد کاربران ban‌شده (باز هم حساب می‌گیرند — تاریخچه مال خودشان است)
    pub banned: usize,
    /// یادداشت genesis — خودش ردیف تعدیل شفاف است (تصمیم #۵)
    pub memo: String,
}

/// ساخت برنامهٔ genesis از ردیف‌های خام لگاسی — تابع خالص، بدون ساعت و IO.
///
/// گردکردن ADR-002: `floor(credits)` برای هر شهروند؛ کسور جمعی (۰.۸ Đ)
/// در سطح جمع حذف می‌شوند نه در سطح فرد — و همین در یادداشت مستند است.
#[must_use]
pub fn plan_genesis(users: &[LegacyUserRow]) -> GenesisPlan {
    let sum_credits: f64 = users.iter().map(|u| u.credits).sum();
    let citizens: Vec<GenesisCitizen> = users
        .iter()
        .map(|u| GenesisCitizen {
            account_id: format!("user:tg:{}", u.user_id),
            label: u
                .username
                .clone()
                .or_else(|| u.first_name.clone())
                .unwrap_or_else(|| format!("tg:{}", u.user_id)),
            din: u.credits.floor() as i64,
        })
        .collect();
    let sum_din: i64 = citizens.iter().map(|c| c.din).sum();
    let holders = citizens.iter().filter(|c| c.din > 0).count();
    let banned = users.iter().filter(|u| u.is_banned == 1).count();
    let memo = format!(
        "legacy genesis — Σ held {sum_din} Đ (floor per citizen from {sum_credits:.1}) — \
         legacy tx log implied 187,735.8; stored balances 187,672.8; gap −63.0 resolved: \
         9×50 purchases without tx rows (−450) + manual owner credit (+387) — genesis audit ۱۴۰۵"
    );
    GenesisPlan { citizens, sum_credits, sum_din, holders, banned, memo }
}

/// Din امن برای یک مبلغ برنامهٔ genesis — helper برای تست و گزارش.
///
/// # Errors
/// اگر مقدار از مرز امن 2^53−1 بگذرد (در عمل برای genesis لگاسی غیرممکن).
pub fn citizen_din(c: &GenesisCitizen) -> dropagentx_domain::DomainResult<Din> {
    Din::new(c.din)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: &str, credits: f64, banned: i64) -> LegacyUserRow {
        LegacyUserRow {
            user_id: id.into(),
            username: None,
            first_name: None,
            credits,
            is_banned: banned,
        }
    }

    #[test]
    fn floors_per_citizen_and_sums_to_expected_supply() {
        // ۴۶۵ کاربر ۴۰۳.۰ + یک کاربر ۲۷۷.۸ = Σ ۱۸۷٬۶۷۲.۸ → Σ floor = ۱۸۷٬۶۷۲
        let mut users: Vec<LegacyUserRow> =
            (0..465).map(|i| user(&format!("u{i}"), 403.0, 0)).collect();
        users.push(user("last", 277.8, 0));
        let plan = plan_genesis(&users);
        assert_eq!(users.len(), 466);
        assert!((plan.sum_credits - 187_672.8).abs() < 1e-9);
        assert_eq!(plan.sum_din, EXPECTED_SUPPLY);
        assert_eq!(plan.holders, 466);
        assert_eq!(plan.banned, 0);
        assert!(plan.memo.contains("Σ held 187672 Đ"));
        assert!(plan.memo.contains("187672.8"));
        assert!(plan.memo.contains("gap −63.0"));
        assert!(plan.memo.contains("۱۴۰۵"));
    }

    #[test]
    fn labels_follow_username_then_first_name_then_tg() {
        let users = vec![
            LegacyUserRow {
                user_id: "1".into(),
                username: Some("kia".into()),
                first_name: Some("کیا".into()),
                credits: 10.9,
                is_banned: 0,
            },
            LegacyUserRow {
                user_id: "2".into(),
                username: None,
                first_name: Some("آدا".into()),
                credits: 0.0,
                is_banned: 1,
            },
            user("3", 5.0, 0),
        ];
        let plan = plan_genesis(&users);
        assert_eq!(plan.citizens[0].label, "kia");
        assert_eq!(plan.citizens[1].label, "آدا");
        assert_eq!(plan.citizens[2].label, "tg:3");
        assert_eq!(plan.citizens[0].account_id, "user:tg:1");
        // floor(10.9) = 10 — کسر حذف‌شده در سطح فرد
        assert_eq!(plan.citizens[0].din, 10);
        assert_eq!(plan.holders, 2); // شهروند صفر hodlr نیست
        assert_eq!(plan.banned, 1);
    }
}
