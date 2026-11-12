//! قراردادهای شفافیت اقتصاد — مادهٔ ۶ منشور — پورت `economy.ts`.
//! فقط‌خواندنی و عمومی: ملت همیشه می‌تواند پول خودش را ممیزی کند.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::Din;

/// واحد پول — literal روی وایر.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum Currency {
    /// `"DNX"`
    #[serde(rename = "DNX")]
    Dnx,
}

/// نماد پول — literal روی وایر.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DinSymbol {
    /// `"Đ"`
    #[serde(rename = "Đ")]
    Dinar,
}

/// `economySummarySchema` — خلاصهٔ اقتصاد ملی.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EconomySummary {
    /// واحد پول (همیشه `"DNX"`)
    pub currency: Currency,
    /// نماد (همیشه `"Đ"`)
    pub symbol: DinSymbol,
    /// عرضهٔ در گردش
    pub circulating_supply: Din,
    /// موجودی خزانه
    pub treasury_balance: Din,
    /// قفل‌شده در اسکروها
    pub escrow_locked: Din,
    /// کل ضرب‌شده از آغاز
    pub minted_all_time: Din,
    /// کل سوزانده‌شده از آغاز
    pub burned_all_time: Din,
    /// ضرب امسال
    pub minted_this_year: Din,
    /// سقف ضرب سالانه (مادهٔ ۳)
    pub yearly_mint_cap: Din,
    /// عرضهٔ جنسیس (ADR-002)
    pub genesis_supply: Din,
    /// مجموع تراز آزمایشی (باید ۰ باشد)
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub trial_balance_sum: i64,
    /// آیا دفتر متوازن است؟
    pub balanced: bool,
    /// تعداد حساب‌ها
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub account_count: i64,
    /// اسکروهای باز
    #[schema(minimum = -9_007_199_254_740_991_i64, maximum = 9_007_199_254_740_991_i64)]
    pub open_escrows: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn literals_render_exactly_like_zod() {
        let json = r#"{"currency":"DNX","symbol":"Đ","circulatingSupply":187672,"treasuryBalance":127672,"escrowLocked":60000,"mintedAllTime":247672,"burnedAllTime":60000,"mintedThisYear":247672,"yearlyMintCap":10000000,"genesisSupply":187672,"trialBalanceSum":0,"balanced":true,"accountCount":468,"openEscrows":2}"#;
        let v: EconomySummary = from_str(json).unwrap();
        assert_eq!(v.currency, Currency::Dnx);
        assert_eq!(v.symbol, DinSymbol::Dinar);
        assert_eq!(to_string(&v).unwrap(), json);
    }
}
