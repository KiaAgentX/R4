//! تست‌های orchestrator روی backend درون‌حافظه‌ای — پورت معیارهای پذیرش
//! فاز ۵ ADR-009:
//!
//! ۱) retry با همان message_id دوبار پول جابه‌جا نمی‌کند (سیم کلید
//!    یکسان‌سازی از پیام تلگرام می‌گیرد)؛
//! ۲) HITL ≥ ۵۰٬۰۰۰ Đ از صف می‌گذرد و تأییدش دقیقاً ۹۰/۱۰ تسویه می‌کند؛
//!    ادمین دیگر دوباره تأیید کند → خطای تایپ‌شده با بازپرداخت صفر؛
//! به‌علاوهٔ authz fail-closed (خریدار/فروشنده)، i18n fa/en و نگاشت خطاها.

use dropagentx_bot::{
    handle_update, BotAction, Env, Locale, MemoryDomain, WireChat, WireMessage, WireUser,
};

const NOW: &str = "2026-11-11T00:00:00.000Z";
const BUYER: i64 = 1001;
const SELLER: i64 = 2002;
const ADMIN: i64 = 42;
const CHAT: i64 = 1;

fn msg(msg_id: i64, user: i64, text: &str) -> WireMessage {
    WireMessage {
        message_id: msg_id,
        from: Some(WireUser {
            id: user,
            first_name: Some("Kia".into()),
            username: None,
            language_code: Some("fa".into()),
        }),
        chat: WireChat { id: CHAT, chat_type: "private".into() },
        text: Some(text.to_owned()),
    }
}

fn text_of(actions: &[BotAction]) -> String {
    match &actions[0] {
        BotAction::Send { text, .. } => text.clone(),
    }
}

fn has_keyboard(actions: &[BotAction]) -> bool {
    matches!(&actions[0], BotAction::Send { keyboard: Some(_), .. })
}

/// کشور آمادهٔ تست: خریدار ۶۰٬۰۰۰ Đ، فروشنده و ادمین.
async fn fresh() -> MemoryDomain {
    let mut env = MemoryDomain::new(NOW);
    env.with_admins(&[ADMIN]);
    env.seed_user(BUYER, 60_000);
    env.seed_user(SELLER, 1_000);
    env
}

/// خرید → تأمین → تحویل؛ امانت DELIVERED برای تست‌های release.
/// شناسهٔ امانت: `esc:c1m{10+seq}` — از شمارهٔ پیام خرید می‌آید.
async fn delivered(env: &mut MemoryDomain, amount: i64, seq: i64) {
    handle_update(env, &msg(10 + seq, BUYER, &format!("/buy {SELLER} {amount}"))).await;
    handle_update(env, &msg(20 + seq, BUYER, "/fund")).await;
    handle_update(env, &msg(30 + seq, SELLER, "/deliver")).await;
}

// ── معیار ۱: replay امن با همان message_id ────────────────────────────────

#[tokio::test]
async fn fund_replay_with_same_message_id_moves_money_once() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;

    let buyer_before = env.balance(BUYER).await.unwrap();
    let escrow_amount = env.escrow("esc:c1m10").await.unwrap().expect("escrow exists").amount;

    // اولین /fund — پول قفل می‌شود
    let _first = handle_update(&mut env, &msg(11, BUYER, "/fund")).await;
    assert_eq!(env.balance(BUYER).await.unwrap(), buyer_before - 40_000);

    // retry همان update (وب‌هوک تکراری تلگرام) — همان message_id
    let replay = handle_update(&mut env, &msg(11, BUYER, "/fund")).await;
    assert!(
        text_of(&replay).contains("قبلاً پردازش شده") || text_of(&replay).contains("FUNDED"),
        "پیام replay باید صادقانه باشد: {}",
        text_of(&replay)
    );
    assert_eq!(env.balance(BUYER).await.unwrap(), buyer_before - 40_000, "پول دوبار حرکت نکرد");
    let escrow = env.escrow("esc:c1m10").await.unwrap().expect("escrow");
    assert_eq!(escrow.amount, escrow_amount);
    assert_eq!(escrow.status, "FUNDED");
}

#[tokio::test]
async fn release_replay_after_hitl_queue_creates_no_second_request() {
    let mut env = fresh().await;
    delivered(&mut env, 60_000, 0).await;

    let first = handle_update(&mut env, &msg(41, BUYER, "/release")).await;
    assert!(
        text_of(&first).contains("HITL") || text_of(&first).contains("صف"),
        "{}",
        text_of(&first)
    );
    assert_eq!(env.hitl_len(), 1, "یک درخواست HITL");

    // retry همان پیام — درخواست دوم نباید ساخته شود
    let _replay = handle_update(&mut env, &msg(41, BUYER, "/release")).await;
    assert_eq!(env.hitl_len(), 1, "replay درخواست تکراری نمی‌سازد");
}

#[tokio::test]
async fn buy_replay_with_same_message_id_creates_no_second_escrow() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;
    let again = handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;
    assert_eq!(env.open_escrows_of(BUYER).await.unwrap().len(), 1);
    assert!(text_of(&again).contains("esc:c1m10"), "همان امانت نمایش داده می‌شود");
}

// ── معیار ۲: صف HITL — تسویهٔ دقیق ۹۰/۱۰، تصمیم دوم تایپ‌شده ─────────────

#[tokio::test]
async fn hitl_above_threshold_queues_and_approve_settles_exactly_90_10() {
    let mut env = fresh().await;
    delivered(&mut env, 60_000, 0).await;
    let escrow_id = "esc:c1m10";
    assert_eq!(env.escrow(escrow_id).await.unwrap().unwrap().status, "DELIVERED");

    // ۱) /release ۶۰٬۰۰۰ Đ → صف HITL، امانت همان‌جا می‌ماند
    let queued = handle_update(&mut env, &msg(41, BUYER, "/release")).await;
    assert!(
        text_of(&queued).contains("مادهٔ ۷") && text_of(&queued).contains("۵۰٬۰۰۰"),
        "پیام صف HITL: {}",
        text_of(&queued)
    );
    assert_eq!(
        env.escrow(escrow_id).await.unwrap().unwrap().status,
        "DELIVERED",
        "امانت هنوز تحویل‌شده"
    );
    assert_eq!(env.balance(SELLER).await.unwrap(), 1_000, "هیچ پولی تسویه نشده");
    let queue = env.hitl_queue().await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, format!("hitl:{escrow_id}"));
    assert_eq!(queue[0].amount, 60_000);
    assert_eq!(queue[0].status, "PENDING");

    // ۲) /approve ادمین → تسویهٔ اتمیک دقیقاً ۹۰/۱۰
    let approved = handle_update(&mut env, &msg(50, ADMIN, "/approve hitl:esc:c1m10")).await;
    let seller_now = env.balance(SELLER).await.unwrap();
    assert_eq!(seller_now, 1_000 + 54_000, "سهم فروشنده دقیقاً ۹۰٪ (54,000)");
    assert!(
        text_of(&approved).contains("54,000") && text_of(&approved).contains("6,000"),
        "پیام تسویه باید تقسیم را نشان دهد: {}",
        text_of(&approved)
    );
    assert_eq!(env.escrow(escrow_id).await.unwrap().unwrap().status, "RELEASED");
    // تراز آزمایشی تقدس — همان ADR-003
    assert!(env.economy().await.unwrap().balanced);
}

#[tokio::test]
async fn hitl_second_approve_by_another_admin_is_typed_error_with_zero_refund() {
    let mut env = fresh().await;
    env.with_admins(&[ADMIN, 43]);
    delivered(&mut env, 60_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/release")).await;

    let first = env.hitl_approve(ADMIN, "hitl:esc:c1m10", None).await.unwrap();
    assert_eq!(first.status, "APPROVED");
    assert_eq!(first.seller_net, Some(54_000));
    let seller_after_first = env.balance(SELLER).await.unwrap();
    assert_eq!(seller_after_first, 55_000);

    // ادمین دیگر دوباره تأیید می‌کند → خطای تایپ‌شده، بازپرداخت صفر
    let second = handle_update(&mut env, &msg(51, 43, "/approve hitl:esc:c1m10")).await;
    assert!(text_of(&second).contains("قبلاً تصمیم‌گیری شده"), "خطای تصمیم دوم: {}", text_of(&second));
    assert_eq!(env.balance(SELLER).await.unwrap(), seller_after_first, "بازپرداخت صفر — تراز ثابت");
    assert_eq!(env.balance(BUYER).await.unwrap(), 0, "خریدار هم دست‌نخورده");
}

#[tokio::test]
async fn hitl_threshold_boundary_exactly_50_000_requires_approval() {
    let mut env = fresh().await;
    env.seed_user(BUYER, 0); // افزایش موجودی برای ۵۰k
    delivered(&mut env, 50_000, 0).await;
    let queued = handle_update(&mut env, &msg(41, BUYER, "/release")).await;
    assert!(
        text_of(&queued).contains("صف") || text_of(&queued).contains("HITL"),
        "مرز دقیق آستانه هم HITL می‌خواهد: {}",
        text_of(&queued)
    );
    assert_eq!(env.hitl_len(), 1);
}

#[tokio::test]
async fn hitl_reject_moves_no_money() {
    let mut env = fresh().await;
    delivered(&mut env, 60_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/release")).await;

    let rejected = handle_update(&mut env, &msg(52, ADMIN, "/reject hitl:esc:c1m10 because")).await;
    assert!(text_of(&rejected).contains("هیچ پولی"), "{}", text_of(&rejected));
    assert_eq!(env.balance(SELLER).await.unwrap(), 1_000);
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "DELIVERED");
    // صف دیگر PENDING ندارد
    assert!(env.hitl_queue().await.unwrap().is_empty());
}

#[tokio::test]
async fn hitl_expired_request_is_typed_error() {
    let mut env = fresh().await;
    delivered(&mut env, 60_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/release")).await;

    // ۲۵ ساعت بعد — TTL ۲۴ ساعت منقضی شده
    env.set_now("2026-11-12T01:00:00.000Z");
    let late = handle_update(&mut env, &msg(53, ADMIN, "/approve hitl:esc:c1m10")).await;
    assert!(
        text_of(&late).contains("منقضی") || text_of(&late).contains("TTL"),
        "خطای انقضا: {}",
        text_of(&late)
    );
    assert_eq!(env.balance(SELLER).await.unwrap(), 1_000, "تسویه‌ای رخ نداد");
}

#[tokio::test]
async fn queue_and_approve_are_admin_only_fail_closed() {
    let mut env = fresh().await;
    delivered(&mut env, 60_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/release")).await;

    let queue_denied = handle_update(&mut env, &msg(60, BUYER, "/queue")).await;
    assert!(text_of(&queue_denied).contains("ادمین"), "{}", text_of(&queue_denied));
    let approve_denied = handle_update(&mut env, &msg(61, SELLER, "/approve hitl:esc:c1m10")).await;
    assert!(text_of(&approve_denied).contains("ادمین"));
    assert_eq!(env.balance(SELLER).await.unwrap(), 1_000, "غیرادمین تسویه نکرد");
}

// ── authz fail-closed روی گذارهای امانت ──────────────────────────────────

#[tokio::test]
async fn seller_cannot_fund_and_stranger_cannot_act() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;

    let seller_fund = handle_update(&mut env, &msg(12, SELLER, "/fund")).await;
    assert!(
        text_of(&seller_fund).contains("فقط خریدار"),
        "authz فروشنده برای fund: {}",
        text_of(&seller_fund)
    );
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "CREATED");

    let stranger_dispute = handle_update(&mut env, &msg(13, 9999, "/dispute esc:c1m10")).await;
    assert!(
        text_of(&stranger_dispute).contains("نه خریدار")
            || text_of(&stranger_dispute).contains("فروشنده"),
        "authz غریبه: {}",
        text_of(&stranger_dispute)
    );
}

#[tokio::test]
async fn only_seller_delivers_and_only_buyer_releases() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;
    handle_update(&mut env, &msg(11, BUYER, "/fund")).await;

    let buyer_deliver = handle_update(&mut env, &msg(14, BUYER, "/deliver")).await;
    assert!(text_of(&buyer_deliver).contains("فقط فروشنده"), "{}", text_of(&buyer_deliver));
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "FUNDED");

    handle_update(&mut env, &msg(15, SELLER, "/deliver")).await;
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "DELIVERED");

    let seller_release = handle_update(&mut env, &msg(16, SELLER, "/release")).await;
    assert!(text_of(&seller_release).contains("فقط خریدار"), "{}", text_of(&seller_release));
}

#[tokio::test]
async fn release_below_threshold_settles_immediately_exactly_90_10() {
    let mut env = fresh().await;
    env.seed_user(BUYER, 0);
    delivered(&mut env, 40_000, 0).await;
    let seller_before = env.balance(SELLER).await.unwrap();

    let released = handle_update(&mut env, &msg(41, BUYER, "/release")).await;
    assert!(text_of(&released).contains("۹۰/۱۰"), "{}", text_of(&released));
    assert_eq!(env.balance(SELLER).await.unwrap(), seller_before + 36_000, "۹۰٪ دقیق");
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "RELEASED");
    assert_eq!(env.hitl_len(), 0, "زیر آستانه صف نمی‌رود");
}

#[tokio::test]
async fn dispute_by_buyer_locks_and_cancel_only_in_created() {
    let mut env = fresh().await;
    delivered(&mut env, 40_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/dispute کالا مطابق توضیح نبود")).await;
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "DISPUTED");

    // cancel روی DISPUTED → گذار نامعتبر (پیام محلی)
    let bad_cancel = handle_update(&mut env, &msg(42, BUYER, "/cancel")).await;
    assert!(
        text_of(&bad_cancel).contains("گذار") || text_of(&bad_cancel).contains("مجاز نیست"),
        "گذار نامعتبر: {}",
        text_of(&bad_cancel)
    );

    // cancel روی CREATED — بدون حرکت پول (امانت تازه در پیام ۴۴)
    handle_update(&mut env, &msg(44, BUYER, "/buy 2002 10000")).await;
    let buyer_before = env.balance(BUYER).await.unwrap();
    let cancelled = handle_update(&mut env, &msg(45, BUYER, "/cancel esc:c1m44")).await;
    assert!(text_of(&cancelled).contains("لغو"));
    assert_eq!(env.balance(BUYER).await.unwrap(), buyer_before);
}

// ── کیف و منو و i18n ─────────────────────────────────────────────────────

#[tokio::test]
async fn balance_and_history_and_products() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 40000")).await;

    let balance = handle_update(&mut env, &msg(21, BUYER, "/balance")).await;
    assert!(text_of(&balance).contains("60,000 Đ"), "{}", text_of(&balance));

    let history = handle_update(&mut env, &msg(22, BUYER, "/history")).await;
    assert!(
        text_of(&history).contains("seed") || text_of(&history).contains("+"),
        "{}",
        text_of(&history)
    );

    let products = handle_update(&mut env, &msg(23, BUYER, "/products")).await;
    assert!(text_of(&products).contains("نیست"), "بدون بذر محصول: {}", text_of(&products));

    env.seed_product("prod:1", SELLER, "Droplinked Pro", 12_500);
    let products = handle_update(&mut env, &msg(24, BUYER, "/products")).await;
    assert!(text_of(&products).contains("prod:1") && text_of(&products).contains("12,500 Đ"));
}

#[tokio::test]
async fn buy_from_product_id_uses_seller_and_price() {
    let mut env = fresh().await;
    env.seed_user(SELLER, 0);
    env.seed_product("prod:1", SELLER, "Droplinked Pro", 12_500);
    handle_update(&mut env, &msg(10, BUYER, "/buy prod:1")).await;
    let escrow = env.escrow("esc:c1m10").await.unwrap().expect("escrow from product");
    assert_eq!(escrow.amount, 12_500, "مبلغ از ردیف محصول");
    assert_eq!(escrow.seller_id, format!("user:tg:{SELLER}"));
    assert_eq!(escrow.product_ref.as_deref(), Some("prod:1"));
    let funded = handle_update(&mut env, &msg(11, BUYER, "/fund")).await;
    assert!(text_of(&funded).contains("تأمین"), "{}", text_of(&funded));
}

#[tokio::test]
async fn unknown_account_error_is_localized() {
    let mut env = MemoryDomain::new(NOW);
    let balance = handle_update(&mut env, &msg(1, 5555, "/balance")).await;
    assert!(
        text_of(&balance).contains("حساب") && text_of(&balance).contains("موجود نیست"),
        "پیام fa حساب ناشناخته: {}",
        text_of(&balance)
    );
}

// ── منو، زبان و برچسب‌های کیبورد ─────────────────────────────────────────

#[tokio::test]
async fn start_welcomes_with_glass_keyboard_fa() {
    let mut env = fresh().await;
    let start = handle_update(&mut env, &msg(1, BUYER, "/start")).await;
    assert!(text_of(&start).contains("کشور Agent-One"));
    assert!(has_keyboard(&start), "کیبورد شیشه‌ای با /start می‌آید");
}

#[tokio::test]
async fn english_language_code_gets_english_replies() {
    let mut env = MemoryDomain::new(NOW);
    env.seed_user(BUYER, 100);
    let mut m = msg(1, BUYER, "/balance");
    if let Some(f) = m.from.as_mut() {
        f.language_code = Some("en".into());
    }
    let reply = handle_update(&mut env, &m).await;
    assert!(text_of(&reply).contains("Your wallet balance"), "{}", text_of(&reply));
    // hint زبان per-message است — انتخاب پایدار فقط با /lang نشسته می‌شود
    handle_update(&mut env, &msg(2, BUYER, "/lang en")).await;
    assert_eq!(env.locale(BUYER, None), Locale::En, "انتخاب صریح در session نشسته");
}

#[tokio::test]
async fn lang_command_switches_and_persists() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(1, BUYER, "/lang en")).await;
    let balance = handle_update(&mut env, &msg(2, BUYER, "/balance")).await;
    assert!(
        text_of(&balance).contains("Your wallet balance"),
        "زبان persist شد: {}",
        text_of(&balance)
    );
    // برگشت به fa
    handle_update(&mut env, &msg(3, BUYER, "/lang fa")).await;
    let balance = handle_update(&mut env, &msg(4, BUYER, "/balance")).await;
    assert!(text_of(&balance).contains("موجودی کیف شما"));
}

#[tokio::test]
async fn glass_keyboard_labels_resolve_to_commands() {
    let mut env = fresh().await;
    let via_label = handle_update(&mut env, &msg(1, BUYER, "💰 موجودی")).await;
    assert!(text_of(&via_label).contains("موجودی کیف شما"), "برچسب کیبورد = فرمان");
    let fa_help = handle_update(&mut env, &msg(2, BUYER, "❓ راهنما")).await;
    assert!(text_of(&fa_help).contains("راهنمای بات"), "برچسب fa → پیام fa");
    // برچسب en با زبان en
    let mut m = msg(3, BUYER, "❓ Help");
    if let Some(f) = m.from.as_mut() {
        f.language_code = Some("en".into());
    }
    let via_en_label = handle_update(&mut env, &m).await;
    assert!(text_of(&via_en_label).contains("Agent-One bot guide"), "برچسب en → پیام en");
}

#[tokio::test]
async fn help_hides_admin_commands_from_citizens() {
    let mut env = fresh().await;
    let citizen = handle_update(&mut env, &msg(1, BUYER, "/help")).await;
    assert!(!text_of(&citizen).contains("/approve"), "غیرادمین فرمان ادمین نمی‌بیند");
    let admin = handle_update(&mut env, &msg(2, ADMIN, "/help")).await;
    assert!(text_of(&admin).contains("/approve"), "ادمین فرمان‌های ادمین را می‌بیند");
}

#[tokio::test]
async fn unknown_text_and_bad_args_get_menu_hints() {
    let mut env = fresh().await;
    let unknown = handle_update(&mut env, &msg(1, BUYER, "سلام چطوری")).await;
    assert!(text_of(&unknown).contains("نمی‌فهمم"), "{}", text_of(&unknown));
    assert!(has_keyboard(&unknown));

    let bad_buy = handle_update(&mut env, &msg(2, BUYER, "/buy")).await;
    assert!(text_of(&bad_buy).contains("استفادهٔ درست"), "{}", text_of(&bad_buy));
    let bad_amount = handle_update(&mut env, &msg(3, BUYER, "/buy 2002 -5")).await;
    assert!(text_of(&bad_amount).contains("استفادهٔ درست"));

    let no_escrow = handle_update(&mut env, &msg(4, BUYER, "/status esc:nope")).await;
    assert!(text_of(&no_escrow).contains("پیدا نشد"), "{}", text_of(&no_escrow));
}

// ── ادمین: اقتصاد و audit ────────────────────────────────────────────────

#[tokio::test]
async fn economy_and_audit_are_admin_only_and_balanced() {
    let mut env = fresh().await;
    delivered(&mut env, 40_000, 0).await;
    handle_update(&mut env, &msg(41, BUYER, "/release")).await;

    let eco = handle_update(&mut env, &msg(70, ADMIN, "/economy")).await;
    assert!(text_of(&eco).contains("متوازن: بله"), "تراز آزمایشی صفر: {}", text_of(&eco));
    assert!(text_of(&eco).contains("خزانه"));

    let audit = handle_update(&mut env, &msg(71, ADMIN, "/audit")).await;
    assert!(text_of(&audit).contains("escrow.create"), "audit قلب شفافیت: {}", text_of(&audit));

    let eco_denied = handle_update(&mut env, &msg(72, BUYER, "/economy")).await;
    assert!(text_of(&eco_denied).contains("ادمین"));
}

#[tokio::test]
async fn fund_overdraft_is_localized_typed_error() {
    let mut env = fresh().await;
    handle_update(&mut env, &msg(10, BUYER, "/buy 2002 99000")).await;
    let reply = handle_update(&mut env, &msg(11, BUYER, "/fund")).await;
    assert!(
        text_of(&reply).contains("کافی نیست") || text_of(&reply).contains("منفی"),
        "overdraft fail-closed: {}",
        text_of(&reply)
    );
    assert_eq!(env.escrow("esc:c1m10").await.unwrap().unwrap().status, "CREATED", "پولی قفل نشد");
}
