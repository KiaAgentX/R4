//! کیبورد شیشه‌ای — منوهای fa/en (ADR-009 فاز ۵).
//!
//! «شیشه‌ای» یعنی دکمه‌ها متن می‌فرستند و orchestrator همان برچسب را به فرمان
//! resolve می‌کند — هیچ مسیر callbackQuery جداگانه‌ای لازم نیست؛ همین الگو
//! رانر را ساده نگه می‌دارد (webhook و polling فقط متن می‌بینند).

use crate::cmd::Cmd;
use crate::i18n::{cmd_label, Locale};
use crate::wire::ReplyKeyboard;

/// کیبورد اصلی کاربر — ۴ ردیف.
#[must_use]
pub fn menu(locale: Locale) -> ReplyKeyboard {
    let l = |c| cmd_label(locale, c);
    ReplyKeyboard::new(&[
        &[l(Cmd::Balance), l(Cmd::History), l(Cmd::Products)],
        &[l(Cmd::Buy), l(Cmd::Status), l(Cmd::Fund)],
        &[l(Cmd::Deliver), l(Cmd::Release), l(Cmd::Dispute)],
        &[l(Cmd::Cancel), l(Cmd::Lang), l(Cmd::Help)],
    ])
}

/// کیبورد ادمین — صف/تصمیم/گزارش‌ها.
#[must_use]
pub fn admin(locale: Locale) -> ReplyKeyboard {
    let l = |c| cmd_label(locale, c);
    ReplyKeyboard::new(&[
        &[l(Cmd::Queue), l(Cmd::Economy)],
        &[l(Cmd::Balance), l(Cmd::History), l(Cmd::Help)],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_has_glass_keyboard_shape_for_both_locales() {
        for locale in [Locale::Fa, Locale::En] {
            let kb = menu(locale);
            assert!(kb.resize_keyboard);
            assert_eq!(kb.keyboard.len(), 4);
            assert_eq!(kb.keyboard[0].len(), 3);
            // هر برچسب دکمه باید به فرمان برسد (رفت‌وبرگشت)
            for row in &kb.keyboard {
                for label in row {
                    assert!(
                        crate::i18n::label_to_cmd(label).is_some(),
                        "برچسب «{label}» به فرمان نمی‌رسد"
                    );
                }
            }
        }
    }

    #[test]
    fn admin_keyboard_lists_queue_and_economy() {
        let kb = admin(Locale::Fa);
        let flat: Vec<&String> = kb.keyboard.iter().flatten().collect();
        assert!(flat.contains(&&"🗂 صف HITL".to_string()));
        assert!(flat.contains(&&"📊 اقتصاد".to_string()));
    }
}
