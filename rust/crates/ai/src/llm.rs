//! لایهٔ LLM — سه‌اسلات ADR-008 با fallback قطعی primary→secondary→fallback.
//!
//! هر اسلات «همه‌یا-هیچ» است (ADR-006: base_url + api_key + model)؛ کلید
//! هرگز در لاگ/خطا نمی‌آید (redaction حفظ می‌شود)؛ timeout با `tokio::time`
//! (معادل AbortController). برای تست، `FakeProvider` بدون شبکه جواب می‌دهد.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;

use crate::error::LlmError;

/// پاسخ کامل یک اسلات.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmReply {
    /// متن خام خروجی (هنوز پاکت نیست — در engine تجزیه می‌شود)
    pub text: String,
    /// توکن ورودی
    pub prompt_tokens: i64,
    /// توکن خروجی
    pub completion_tokens: i64,
}

/// قرارداد ارائه‌دهنده — RPITIT؛ dispatch با enum (بدون dyn).
pub trait LlmProvider: Send + Sync {
    /// نام اسلات — `primary`/`secondary`/`fallback`
    fn slot(&self) -> &'static str;
    /// نام ارائه‌دهنده — در پاسخ و audit می‌آید
    fn provider(&self) -> &str;
    /// نام مدل
    fn model(&self) -> &str;
    /// اجرای chat completion
    ///
    /// # Errors
    /// [`LlmError`] — timeout/HTTP/بدنهٔ بد/اسلات قطع.
    fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> impl Future<Output = Result<LlmReply, LlmError>> + Send;
}

/// timeout پیش‌فرض فراخوانی — همان سیاست TS فعلی.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// یکی از دو پیاده‌سازی — dispatch بدون dyn (RPITIT سازگار است).
pub enum AnyProvider {
    /// کلاینت OpenAI-سازگار (reqwest + rustls)
    OpenAi(OpenAiProvider),
    /// ارائه‌دهندهٔ اسکریپت‌شدهٔ تست — بدون شبکه
    Fake(FakeProvider),
}

impl LlmProvider for AnyProvider {
    fn slot(&self) -> &'static str {
        match self {
            AnyProvider::OpenAi(p) => p.slot(),
            AnyProvider::Fake(p) => p.slot(),
        }
    }

    fn provider(&self) -> &str {
        match self {
            AnyProvider::OpenAi(p) => p.provider(),
            AnyProvider::Fake(p) => p.provider(),
        }
    }

    fn model(&self) -> &str {
        match self {
            AnyProvider::OpenAi(p) => p.model(),
            AnyProvider::Fake(p) => p.model(),
        }
    }

    async fn complete(&self, system: &str, user: &str) -> Result<LlmReply, LlmError> {
        match self {
            AnyProvider::OpenAi(p) => p.complete(system, user).await,
            AnyProvider::Fake(p) => p.complete(system, user).await,
        }
    }
}

/// کلاینت OpenAI-سازگار — POST {base_url}/chat/completions.
#[derive(Clone)]
pub struct OpenAiProvider {
    slot: &'static str,
    base_url: String,
    /// کلید API — هرگز در Display/لاگ/خطا نمی‌آید
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // کلید API هرگز در Debug/لاگ نمی‌آید — redaction حفظ می‌شود
        f.debug_struct("OpenAiProvider")
            .field("slot", &self.slot)
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .finish()
    }
}

impl OpenAiProvider {
    /// اسلات از کانفیگ — کلید فقط برای فراخوانی bearer_auth استفاده می‌شود.
    #[must_use]
    pub fn new(slot: &'static str, base_url: String, api_key: String, model: String) -> Self {
        Self {
            slot,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

impl LlmProvider for OpenAiProvider {
    fn slot(&self) -> &'static str {
        self.slot
    }

    fn provider(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, system: &str, user: &str) -> Result<LlmReply, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        });
        let call = self.client.post(&url).bearer_auth(&self.api_key).json(&body).send();
        let response = tokio::time::timeout(CALL_TIMEOUT, call)
            .await
            .map_err(|_| LlmError::Timeout)?
            .map_err(|e| LlmError::Http { status: 0, detail: e.to_string() })?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(LlmError::Http { status, detail: format!("upstream returned {status}") });
        }
        let json: serde_json::Value =
            response.json().await.map_err(|e| LlmError::BadResponse(e.to_string()))?;

        let text = json
            .pointer("/choices/0/message/content")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| LlmError::BadResponse("missing choices[0].message.content".into()))?
            .to_owned();
        let usage = json.get("usage").cloned().unwrap_or(serde_json::json!({}));
        let usage = UsageView::deserialize(usage).unwrap_or_default();
        Ok(LlmReply {
            text,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
        })
    }
}

/// شکل usage پاسخ — فیلدهای غایب = ۰ (مثل TS).
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct UsageView {
    /// توکن ورودی
    prompt_tokens: i64,
    /// توکن خروجی
    completion_tokens: i64,
}

/// هستهٔ مشترک fake — Clone برای به‌اشتراک‌گذاری صف/شمارنده‌ها بین نسخه‌ها.
#[derive(Debug, Default)]
struct FakeInner {
    /// صف پاسخ‌ها — خالی = پاکت reply پیش‌فرض
    script: Mutex<VecDeque<Result<String, LlmError>>>,
    /// همهٔ فراخوانی‌ها برای ادعاها (gates: «قبل از provider»)
    calls: Mutex<Vec<(String, String)>>,
}

/// ارائه‌دهندهٔ اسکریپت‌شدهٔ تست — بدون شبکه؛ cloneها حالت مشترک دارند.
#[derive(Debug, Clone, Default)]
pub struct FakeProvider {
    slot: &'static str,
    provider_name: String,
    model_name: String,
    inner: std::sync::Arc<FakeInner>,
}

impl FakeProvider {
    /// اسلات fake با نام provider/model دلخواه.
    #[must_use]
    pub fn new(slot: &'static str, provider: &str, model: &str) -> Self {
        Self {
            slot,
            provider_name: provider.to_owned(),
            model_name: model.to_owned(),
            inner: std::sync::Arc::new(FakeInner::default()),
        }
    }

    /// اضافه‌کردن پاسخ به صف اسکریپت.
    pub fn push_ok(&self, packet_text: &str) -> &Self {
        if let Ok(mut q) = self.inner.script.lock() {
            q.push_back(Ok(packet_text.to_owned()));
        }
        self
    }

    /// اضافه‌کردن خطا به صف اسکریپت.
    pub fn push_err(&self, err: LlmError) -> &Self {
        if let Ok(mut q) = self.inner.script.lock() {
            q.push_back(Err(err));
        }
        self
    }

    /// تعداد فراخوانی‌های واقعی — ادعای «قبل از provider».
    #[must_use]
    pub fn call_count(&self) -> usize {
        self.inner.calls.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// آخرین فراخوانی (system, user) — برای ادعای redaction روی ورود.
    #[must_use]
    pub fn last_call(&self) -> Option<(String, String)> {
        self.inner.calls.lock().ok().and_then(|c| c.last().cloned())
    }
}

impl LlmProvider for FakeProvider {
    fn slot(&self) -> &'static str {
        self.slot
    }

    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    async fn complete(&self, system: &str, user: &str) -> Result<LlmReply, LlmError> {
        if let Ok(mut calls) = self.inner.calls.lock() {
            calls.push((system.to_owned(), user.to_owned()));
        }
        let next = self.inner.script.lock().ok().and_then(|mut q| q.pop_front());
        let text = match next {
            Some(Ok(t)) => t,
            Some(Err(e)) => return Err(e),
            None => r#"{"reply":"fake default reply"}"#.to_owned(),
        };
        // توکن‌های قطعی برای تست سقف — هر کاراکتر ≈ توکن/۴
        let prompt = i64::try_from(system.len().div_ceil(4)).unwrap_or(i64::MAX);
        let completion = i64::try_from(text.len().div_ceil(4)).unwrap_or(i64::MAX);
        Ok(LlmReply { text, prompt_tokens: prompt, completion_tokens: completion })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_provider_scripts_and_records_calls() {
        let fake = FakeProvider::new("primary", "fake", "test-model");
        fake.push_ok(r#"{"reply":"سلام"}"#);
        let reply = fake.complete("system", "user").await.expect("ok");
        assert_eq!(reply.text, r#"{"reply":"سلام"}"#);
        assert_eq!(fake.call_count(), 1);
        assert_eq!(fake.last_call().expect("recorded").1, "user");
    }

    #[tokio::test]
    async fn fake_provider_default_reply_when_script_empty() {
        let fake = FakeProvider::new("secondary", "fake", "m");
        let reply = fake.complete("s", "u").await.expect("default");
        assert!(reply.text.contains("fake default reply"));
    }

    #[test]
    fn openai_provider_hides_key_in_debug() {
        let p = OpenAiProvider::new(
            "primary",
            "https://api.example.com/v1".into(),
            "sk-super-secret-123456".into(),
            "gpt-x".to_owned(),
        );
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("sk-super-secret"), "کلید هرگز در Debug نمی‌آید: {dbg}");
    }
}
