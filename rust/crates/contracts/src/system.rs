//! قراردادهای متای سیستم (health و backup) — پورت `system.ts`.
//!
//! فیلدهای LLM (ADR-006) فقط «وجود کانفیگ» را نشان می‌دهند — هرگز کلید،
//! هرگز base URL. مدل null یعنی سیکل AI غیرفعال است.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// `status` literal — همیشه `"ok"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthStatus {
    /// `"ok"`
    #[serde(rename = "ok")]
    Ok,
}

/// `service` literal — همیشه `"dropagentx-v3-server"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthService {
    /// `"dropagentx-v3-server"`
    #[serde(rename = "dropagentx-v3-server")]
    DropagentxV3Server,
}

/// `healthResponseSchema` — liveness عمیق: سروری که به دفترش نمی‌رسد سالم نیست.
/// فیلدهای چیپ‌های داشبورد (صفحهٔ settings) additive و optional هستند تا
/// کلاینت‌های قدیمی هم بتوانند validate کنند.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// همیشه `"ok"`
    pub status: HealthStatus,
    /// همیشه `"dropagentx-v3-server"`
    pub service: HealthService,
    /// نسخهٔ سرور
    pub version: String,
    /// آیا LLM کانفیگ شده؟ (فقط وجود — هرگز جزئیات)
    pub llm_configured: bool,
    /// نام مدل یا null (null = AI غیرفعال — صریح روی سیم)
    #[schema(required = true)]
    pub llm_model: Option<String>,
    /// چیپ DB — `"ok"`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db: Option<String>,
    /// چیپ بات — `"ok"` یا `"disabled"`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot: Option<String>,
    /// چیپ AI — `"ok"` یا `"disabled"`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
    /// آپ‌تایم پروسه (ثانیه، float)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<f64>,
}

/// `backupCreatedSchema` — پاسخ `POST /system/backup`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupCreated {
    /// مسیر مطلق اسنپ‌شات داخل volume (db/backups/…)
    pub file: String,
    /// حجم به بایت
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub size_bytes: i64,
    /// تعداد اسنپ‌شات‌های نگه‌داشته‌شده بعد از prune
    #[schema(minimum = 0_i64, maximum = 9_007_199_254_740_991_i64)]
    pub kept: i64,
    /// زمان ایجاد
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn health_literals_and_optional_chips() {
        let json = r#"{"status":"ok","service":"dropagentx-v3-server","version":"4.4.3","llmConfigured":true,"llmModel":"gpt-4o-mini","db":"ok","bot":"ok","ai":"ok","uptime":1234.5}"#;
        let v: HealthResponse = from_str(json).unwrap();
        assert_eq!(v.status, HealthStatus::Ok);
        assert_eq!(v.service, HealthService::DropagentxV3Server);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn health_without_optional_chips_still_parses_and_serializes_minimally() {
        let json = r#"{"status":"ok","service":"dropagentx-v3-server","version":"x","llmConfigured":false,"llmModel":null}"#;
        let v: HealthResponse = from_str(json).unwrap();
        assert_eq!(v.llm_model, None);
        assert_eq!(to_string(&v).unwrap(), json);
    }

    #[test]
    fn health_missing_required_fields_is_rejected() {
        // آزمون RESPONSE_MISMATCH — بدنهٔ ناقص باید رد شود (مثل client.test.ts)
        let bad: Result<HealthResponse, _> = serde_json::from_str(r#"{"status":"ok"}"#);
        assert!(bad.is_err());
    }
}
