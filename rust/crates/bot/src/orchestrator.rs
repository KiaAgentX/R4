//! قلب بات — orchestrator خالص بدون teloxide و بدون IO (ADR-009 §۸٫۵).
//!
//! ```text
//! update → orchestrator → Vec<BotAction> → رانر (webhook آکسوم / polling)
//! ```
//!
//! مسئولیت‌ها:
//! * تجزیهٔ فرمان/برچسب کیبورد (fa/en) و گیت ادمین fail-closed
//! * Authz گذارهای امانت روی سیم (خریدار fund/release/dispute/cancel،
//!   فروشنده deliver) — backend گارد را دوباره داخل فرمان اعمال می‌کند
//! * نگاشت خطای وایر به پیام محلی (i18n fa/en)
//! * کلید یکسان‌سازی از پیام تلگرام: retry با همان message_id هرگز دوبار
//!   پول جابه‌جا نمی‌کند (با chat_id مقیاس می‌شود چون message_id فقط در چت یکتاست)
//!
//! پیاده‌کننده‌های [`Env`] باید `Send + Sync` باشند تا futureهای این ماژول
//! در هندلرهای آکسوم (که `Send` می‌خواهند) قابل await باشند.

use std::future::Future;
use std::pin::Pin;

use dropagentx_domain::DomainErrorCode;

use crate::cmd::Cmd;
use crate::env::{user_account, BotError, Env, Transition};
use crate::i18n::{self, Locale};
use crate::keyboard;
use crate::wire::{BotAction, WireMessage};

/// سقف صفحات فهرست‌ها — همان سقفِ رجیستری داشبورد.
const LIST_LIMIT: usize = 10;

/// اجرای یک update — خالص؛ ورودی/خروجی فقط DTOهای وایر.
pub async fn handle_update<E: Env + Send + Sync>(env: &mut E, msg: &WireMessage) -> Vec<BotAction> {
    // بدون فرستنده (پست کانال) هرگز فرمان اجرا نمی‌شود — fail-closed
    let Some(user) = msg.from.as_ref() else { return Vec::new() };
    let chat = msg.chat.id;
    let uid = user.id;
    let locale = env.locale(uid, user.language_code.as_deref());
    let Some(text) = msg.command_text() else {
        return vec![BotAction::Send {
            chat_id: chat,
            text: i18n::unknown_text(locale),
            keyboard: Some(keyboard::menu(locale)),
        }];
    };

    let cmd = Cmd::parse(text).or_else(|| i18n::label_to_cmd(text));
    let Some(cmd) = cmd else {
        return vec![BotAction::Send {
            chat_id: chat,
            text: i18n::unknown_text(locale),
            keyboard: Some(keyboard::menu(locale)),
        }];
    };
    let args = command_args(text);

    match cmd {
        // ── منو و راهنما ────────────────────────────────────────────────────
        Cmd::Start => {
            let name = user.first_name.clone();
            vec![BotAction::Send {
                chat_id: chat,
                text: i18n::welcome(locale, name.as_deref()),
                keyboard: Some(keyboard::menu(locale)),
            }]
        }
        Cmd::Help => vec![BotAction::Send {
            chat_id: chat,
            text: i18n::help(locale, env.is_admin(uid)),
            keyboard: Some(keyboard::menu(locale)),
        }],
        Cmd::Menu => vec![BotAction::Send {
            chat_id: chat,
            text: i18n::unknown_text(locale),
            keyboard: Some(keyboard::menu(locale)),
        }],
        Cmd::Lang => {
            let next = match args.first().map(String::as_str) {
                Some("en") => Locale::En,
                Some("fa") => Locale::Fa,
                _ => locale.opposite(),
            };
            env.set_locale(uid, next);
            vec![BotAction::Send {
                chat_id: chat,
                text: i18n::lang_switched(next),
                keyboard: Some(keyboard::menu(next)),
            }]
        }

        // ── کیف ─────────────────────────────────────────────────────────────
        Cmd::Balance => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let amount = env.balance(uid).await?;
                    Ok(i18n::balance_msg(locale, amount))
                })
            })
            .await
        }
        Cmd::History => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let items = env.history(uid, LIST_LIMIT).await?;
                    let plain: Vec<(String, i64, Option<String>)> =
                        items.into_iter().map(|h| (h.tx_id, h.amount, h.memo)).collect();
                    Ok(i18n::history_msg(locale, &user_account(uid), &plain))
                })
            })
            .await
        }
        Cmd::Products => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let items = env.products(LIST_LIMIT).await?;
                    let plain: Vec<(String, String, i64)> =
                        items.into_iter().map(|p| (p.id, p.title, p.price_din)).collect();
                    Ok(i18n::products_msg(locale, &plain))
                })
            })
            .await
        }

        // ── امانت ───────────────────────────────────────────────────────────
        Cmd::Buy => {
            let args_owned: Vec<String> = args.to_vec();
            let msg_id = msg.message_id;
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let (seller, amount, product_ref) = parse_buy_args(env, &args_owned).await?;
                    let (escrow, replayed) =
                        env.buy(uid, chat, msg_id, &seller, amount, product_ref.as_deref()).await?;
                    if replayed {
                        return Ok(i18n::escrow_msg(locale, &escrow));
                    }
                    Ok(i18n::escrow_created(locale, &escrow))
                })
            })
            .await
        }
        Cmd::Status => {
            let args_owned: Vec<String> = args.to_vec();
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let escrow_id = resolve_escrow_id(env, uid, &args_owned).await?;
                    let item = env.escrow(&escrow_id).await?.ok_or(BotError::EscrowNotFound)?;
                    Ok(i18n::escrow_msg(locale, &item))
                })
            })
            .await
        }
        Cmd::Fund => {
            transition(env, chat, uid, msg.message_id, &args, Transition::Fund, locale).await
        }
        Cmd::Deliver => {
            transition(env, chat, uid, msg.message_id, &args, Transition::Deliver, locale).await
        }
        Cmd::Release => {
            transition(env, chat, uid, msg.message_id, &args, Transition::Release, locale).await
        }
        Cmd::Dispute => {
            transition(env, chat, uid, msg.message_id, &args, Transition::Dispute, locale).await
        }
        Cmd::Cancel => {
            transition(env, chat, uid, msg.message_id, &args, Transition::Cancel, locale).await
        }

        // ── ادمین (گیت fail-closed) ─────────────────────────────────────────
        Cmd::Queue if env.is_admin(uid) => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let items = env.hitl_queue().await?;
                    Ok(i18n::hitl_list_msg(locale, &items))
                })
            })
            .await
        }
        Cmd::Approve if env.is_admin(uid) => {
            let hitl_id = args.first().cloned();
            let note = args.get(1).cloned();
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let hitl_id =
                        hitl_id.as_deref().ok_or(BotError::BadArgs("/approve <hitlId> [note]"))?;
                    let outcome = env.hitl_approve(uid, hitl_id, note.as_deref()).await?;
                    Ok(i18n::hitl_approved(locale, &outcome))
                })
            })
            .await
        }
        Cmd::Reject if env.is_admin(uid) => {
            let hitl_id = args.first().cloned();
            let note = args.get(1).cloned();
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let hitl_id =
                        hitl_id.as_deref().ok_or(BotError::BadArgs("/reject <hitlId> [note]"))?;
                    let outcome = env.hitl_reject(uid, hitl_id, note.as_deref()).await?;
                    Ok(i18n::hitl_rejected(locale, &outcome.id))
                })
            })
            .await
        }
        Cmd::Economy if env.is_admin(uid) => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let e = env.economy().await?;
                    Ok(i18n::economy_msg(locale, &e))
                })
            })
            .await
        }
        Cmd::Audit if env.is_admin(uid) => {
            run(env, chat, locale, |env, locale| {
                Box::pin(async move {
                    let items = env.audit_tail(LIST_LIMIT).await?;
                    Ok(i18n::audit_msg(locale, &items))
                })
            })
            .await
        }

        // گیت ادمین رد شد — fail-closed (مثل guards.ts فاز ۳)
        Cmd::Queue | Cmd::Approve | Cmd::Reject | Cmd::Economy | Cmd::Audit => {
            error_actions(chat, locale, &BotError::NotAdmin)
        }
    }
}

/// اجرای گذار امانت — resolve شناسه + authz سیم + پیام نتیجه.
async fn transition<E: Env + Send + Sync>(
    env: &mut E,
    chat: i64,
    user: i64,
    message_id: i64,
    args: &[String],
    verb: Transition,
    locale: Locale,
) -> Vec<BotAction> {
    match run_transition(env, chat, user, message_id, args, verb, locale).await {
        Ok(text) => vec![BotAction::Send { chat_id: chat, text, keyboard: None }],
        Err(err) => error_actions(chat, locale, &err),
    }
}

async fn run_transition<E: Env + Send + Sync>(
    env: &mut E,
    chat: i64,
    user: i64,
    message_id: i64,
    args: &[String],
    verb: Transition,
    locale: Locale,
) -> Result<String, BotError> {
    // تشخیص شناسهٔ امانت در آرگومان‌ها — فقط اگر با پیشوند امانت شروع شود
    // (esc: برای بات این ریپو، escrow: برای هندلر سرور)؛ در غیر این صورت
    // auto-resolve و کل آرگومان‌ها به‌عنوان دلیل اختلاف می‌مانند.
    let (escrow_id, reason_args): (String, Vec<String>) = match args.first() {
        Some(a) if a.starts_with("esc") => (a.clone(), args[1..].to_vec()),
        _ => (resolve_escrow_id(env, user, &[]).await?, args.to_vec()),
    };
    // authz سیم — پیام دقیقِ نقش، قبل از فراخوانی backend (backend هم گارد را
    // دوباره داخل فرمان اعمال می‌کند — دو لایه fail-closed)
    let item = env.escrow(&escrow_id).await?.ok_or(BotError::EscrowNotFound)?;
    let account = user_account(user);
    let is_buyer = item.buyer_id == account;
    let is_seller = item.seller_id == account;
    if !is_buyer && !is_seller {
        return Err(BotError::NotParty);
    }
    if verb.buyer_only() && !is_buyer {
        return Err(BotError::BuyerOnly);
    }
    if !verb.buyer_only() && !is_seller {
        return Err(BotError::SellerOnly);
    }

    // دلیل اختلاف — از آرگومان‌های بعد از شناسه یا پیش‌فرض
    let dispute_reason: Option<String> = if verb == Transition::Dispute {
        let joined = reason_args.join(" ");
        Some(if joined.is_empty() { "buyer dispute".to_owned() } else { joined })
    } else {
        None
    };

    let out =
        env.transition(user, chat, message_id, &escrow_id, verb, dispute_reason.as_deref()).await?;

    if let Some(hitl) = out.hitl_queued.as_ref() {
        return Ok(i18n::hitl_queued(locale, hitl));
    }
    if out.replayed {
        return Ok(i18n::escrow_funded(locale, &out.escrow, true));
    }
    match verb {
        Transition::Fund => Ok(i18n::escrow_funded(locale, &out.escrow, false)),
        Transition::Deliver => Ok(i18n::escrow_delivered(locale, &out.escrow)),
        Transition::Release => Ok(i18n::escrow_released(
            locale,
            &out.escrow,
            out.seller_net.unwrap_or(0),
            out.tax.unwrap_or(0),
        )),
        Transition::Dispute => Ok(i18n::escrow_disputed(
            locale,
            &out.escrow,
            dispute_reason.as_deref().unwrap_or("buyer dispute"),
        )),
        Transition::Cancel => Ok(i18n::escrow_cancelled(locale, &out.escrow)),
    }
}

/// اجرای یک عملیات خواندنی/ادمینی — خطاها به پیام محلی می‌رسند.
/// Bound مرتبه‌بالاتر: future جعبه‌شده باید برای همان عمر `&mut E` ورودی معتبر
/// و `Send` باشد (هندلرهای آکسوم `Send` می‌خواهند)؛ ضربه‌گیرها فقط مقدارهای
/// owned/Copy می‌گیرند تا عمرها از مرز closure رد نشوند.
async fn run<E, F>(env: &mut E, chat: i64, locale: Locale, f: F) -> Vec<BotAction>
where
    E: Env + Send + Sync,
    for<'a> F: FnOnce(
        &'a mut E,
        Locale,
    ) -> Pin<Box<dyn Future<Output = Result<String, BotError>> + Send + 'a>>,
{
    match f(env, locale).await {
        Ok(text) => vec![BotAction::Send { chat_id: chat, text, keyboard: None }],
        Err(err) => error_actions(chat, locale, &err),
    }
}

/// اقدام خطا — پیام محلی + کیبورد منو برای ادامهٔ کار.
fn error_actions(chat: i64, locale: Locale, err: &BotError) -> Vec<BotAction> {
    vec![BotAction::Send {
        chat_id: chat,
        text: i18n::error_msg(locale, err),
        keyboard: Some(keyboard::menu(locale)),
    }]
}

/// آرگومان‌های فرمان — بعد از توکن فرمان (با پسوند @Bot هم کار می‌کند).
fn command_args(text: &str) -> Vec<String> {
    let no_slash = text.strip_prefix('/').unwrap_or(text);
    let after_cmd = no_slash.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
    after_cmd.split_whitespace().map(str::to_owned).collect()
}

/// آرگومان‌های /buy — دو حالت: `<productId>` یا `<sellerTgId> <amount>`.
async fn parse_buy_args<E: Env + Send + Sync>(
    env: &E,
    args: &[String],
) -> Result<(String, i64, Option<String>), BotError> {
    let usage = "/buy <productId> | /buy <sellerTgId> <amount>";
    match args.len() {
        1 => {
            // حالت محصول — فروشنده و مبلغ از ردیف محصول (درس تحویل داخل ردیف)
            let pid = args[0].as_str();
            let products = env.products(100).await?;
            let product = products
                .iter()
                .find(|p| p.id == pid)
                .ok_or(BotError::Domain(DomainErrorCode::ProductNotFound))?;
            Ok((product.seller_id.clone(), product.price_din, Some(product.id.clone())))
        }
        2 => {
            let seller = args[0].clone();
            let amount = parse_amount(&args[1], usage)?;
            Ok((seller, amount, None))
        }
        _ => Err(BotError::BadArgs(usage)),
    }
}

/// مبلغ صحیح مثبت — مثل مرز امن contracts (≤ 2^53−1).
fn parse_amount(raw: &str, usage: &'static str) -> Result<i64, BotError> {
    let n: i64 = raw.parse::<i64>().map_err(|_| BotError::BadArgs(usage))?;
    if n <= 0 || n > dropagentx_domain::MAX_DIN {
        return Err(BotError::BadArgs(usage));
    }
    Ok(n)
}

/// resolve شناسهٔ امانت — آرگومان صریح یا آخرین امانت بازِ شهروند.
async fn resolve_escrow_id<E: Env + Send + Sync>(
    env: &E,
    user: i64,
    args: &[String],
) -> Result<String, BotError> {
    if let Some(id) = args.first() {
        return Ok(id.clone());
    }
    // auto-resolve: آخرین امانت بازِ شهروند (خریدار یا فروشنده)
    let open = env.open_escrows_of(user).await?;
    open.last().map(|e| e.id.clone()).ok_or(BotError::NoEligibleEscrow)
}
