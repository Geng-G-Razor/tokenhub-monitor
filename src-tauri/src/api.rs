use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub const BASE_URL: &str = "https://tokenhub.cash";

#[cfg(target_os = "windows")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:152.0) Gecko/20100101 Firefox/152.0";
#[cfg(target_os = "macos")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:152.0) Gecko/20100101 Firefox/152.0";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:152.0) Gecko/20100101 Firefox/152.0";

/// Shared HTTP client. Reuses connections across requests.
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("failed to build reqwest client")
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("unauthorized (401)")]
    Unauthorized,
    #[error("unexpected status {0}: {1}")]
    Status(u16, String),
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PackageData {
    pub id: i64,
    pub plan_id: Option<i64>,
    pub plan_code: Option<String>,
    pub plan_display_name: String,
    pub package_kind: String,
    pub status: String,
    pub billing_unit: String,
    pub service_kind: Option<String>,

    pub total_quota: i64,
    pub remaining_quota: i64,

    pub used_daily: i64,
    pub used_weekly: i64,
    pub used_week: i64,
    pub used_monthly: i64,
    pub used_5h: i64,

    pub weekly_limit: Option<i64>,
    pub daily_limit: Option<i64>,
    pub monthly_limit: Option<i64>,
    pub wall_week_limit: Option<i64>,
    pub wall_5h_limit: Option<i64>,

    pub rpm_total_limit: i64,
    pub rpm_success_limit: i64,

    pub supported_models: Vec<String>,

    pub activated_at: String,
    pub absolute_expire_at: String,
    pub weekly_reset_at: Option<String>,
    pub weekly_reset_after_hours: Option<i64>,

    pub source_redemption_code: Option<String>,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct PackagesEnvelope {
    packages: Vec<PackageData>,
}

/// Fetch the first package from /api/me/packages.
pub async fn fetch_package(master_key: &str) -> Result<PackageData, ApiError> {
    let resp = client()
        .get(format!("{}/api/me/packages", BASE_URL))
        .header("Authorization", format!("Bearer {}", master_key))
        .header("Referer", "https://tokenhub.cash/me")
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if status == reqwest::StatusCode::OK {
        let mut env = serde_json::from_str::<PackagesEnvelope>(&text)
            .map_err(|e| ApiError::Status(200, format!("bad json: {e}")))?;
        // The first package (index 0) is the main model subscription.
        if env.packages.is_empty() {
            Err(ApiError::Status(200, "empty packages array".into()))
        } else {
            Ok(env.packages.remove(0))
        }
    } else if status.as_u16() == 401 {
        Err(ApiError::Unauthorized)
    } else {
        Err(ApiError::Status(status.as_u16(), text))
    }
}
