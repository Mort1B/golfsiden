mod cache;
mod models;
#[cfg(test)]
mod tests;

use std::{sync::Arc, time::Duration};

use chrono::{NaiveDate, Utc};
use futures_util::StreamExt;
use reqwest::{
    StatusCode, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::sync::{Mutex, Semaphore};

use self::cache::Cache;
pub use models::{
    CourseDetail, CourseLocation, CourseSearchResult, Hole, Tee, TeeCategory, TeeCounts,
};
use models::{ProviderCourse, SearchEnvelope};

const OFFICIAL_BASE_URL: &str = "https://api.golfcourseapi.com/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_SEARCH_RESULTS: usize = 20;
const MAX_CONCURRENT_REQUESTS: usize = 2;
const DEFAULT_DAILY_LIMIT: u32 = 50;

#[derive(Clone)]
pub struct CourseProviderClient {
    configured: Option<Arc<ConfiguredClient>>,
}

struct ConfiguredClient {
    http: reqwest::Client,
    base_url: Url,
    permits: Arc<Semaphore>,
    cache: Mutex<Cache>,
    quota: Mutex<DailyQuota>,
}

struct DailyQuota {
    utc_date: NaiveDate,
    used: u32,
    limit: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CourseProviderError {
    #[error("course provider is not configured")]
    Unavailable,
    #[error("course provider is busy")]
    Saturated,
    #[error("course provider request timed out")]
    Timeout,
    #[error("course provider request allowance is exhausted")]
    Exhausted,
    #[error("course was not found")]
    NotFound,
    #[error("course provider request failed")]
    Upstream,
    #[error("course provider returned an invalid response")]
    InvalidResponse,
}

impl CourseProviderClient {
    pub fn disabled() -> Self {
        Self { configured: None }
    }

    pub fn configured(api_key: String) -> Result<Self, CourseProviderError> {
        Self::build(
            api_key,
            OFFICIAL_BASE_URL,
            DEFAULT_DAILY_LIMIT,
            REQUEST_TIMEOUT,
            MAX_CONCURRENT_REQUESTS,
        )
    }

    pub fn configured_with_daily_limit(
        api_key: String,
        daily_limit: u32,
    ) -> Result<Self, CourseProviderError> {
        Self::build(
            api_key,
            OFFICIAL_BASE_URL,
            daily_limit,
            REQUEST_TIMEOUT,
            MAX_CONCURRENT_REQUESTS,
        )
    }

    pub fn configured_with_base_url(
        api_key: String,
        base_url: &str,
    ) -> Result<Self, CourseProviderError> {
        Self::build(
            api_key,
            base_url,
            DEFAULT_DAILY_LIMIT,
            REQUEST_TIMEOUT,
            MAX_CONCURRENT_REQUESTS,
        )
    }

    fn build(
        api_key: String,
        base_url: &str,
        daily_limit: u32,
        timeout: Duration,
        concurrency: usize,
    ) -> Result<Self, CourseProviderError> {
        if api_key.trim().is_empty() || daily_limit == 0 || concurrency == 0 {
            return Err(CourseProviderError::Unavailable);
        }
        let base_url = Url::parse(base_url).map_err(|_| CourseProviderError::Unavailable)?;
        if !matches!(base_url.scheme(), "http" | "https") || base_url.cannot_be_a_base() {
            return Err(CourseProviderError::Unavailable);
        }
        let mut authorization = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|_| CourseProviderError::Unavailable)?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(timeout)
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| CourseProviderError::Unavailable)?;
        Ok(Self {
            configured: Some(Arc::new(ConfiguredClient {
                http,
                base_url,
                permits: Arc::new(Semaphore::new(concurrency)),
                cache: Mutex::new(Cache::default()),
                quota: Mutex::new(DailyQuota {
                    utc_date: Utc::now().date_naive(),
                    used: 0,
                    limit: daily_limit,
                }),
            })),
        })
    }

    #[cfg(test)]
    fn configured_for_test(
        api_key: String,
        base_url: &str,
        timeout: Duration,
        concurrency: usize,
    ) -> Result<Self, CourseProviderError> {
        Self::build(api_key, base_url, 100, timeout, concurrency)
    }

    pub async fn search(
        &self,
        query: &str,
        fuzzy_match: bool,
    ) -> Result<Vec<CourseSearchResult>, CourseProviderError> {
        let client = self.require_configured()?;
        let cache_key = format!("{fuzzy_match}:{}", query.to_lowercase());
        if let Some(cached) = client.cached_search(&cache_key).await {
            return Ok(cached);
        }
        let url = client
            .base_url
            .join("v1/search")
            .map_err(|_| CourseProviderError::Unavailable)?;
        let envelope: SearchEnvelope = client
            .get_json(
                client.http.get(url).query(&[
                    ("search_query", query),
                    ("fuzzy_match", &fuzzy_match.to_string()),
                ]),
                false,
            )
            .await?;
        let results = envelope
            .courses
            .into_iter()
            .take(MAX_SEARCH_RESULTS)
            .map(|course| course.normalize())
            .collect::<Result<Vec<_>, _>>()?;
        client.cache_search(cache_key, results.clone()).await;
        Ok(results)
    }

    pub async fn course(&self, id: &str) -> Result<CourseDetail, CourseProviderError> {
        let client = self.require_configured()?;
        if let Some(cached) = client.cached_course(id).await {
            return Ok(cached);
        }
        let url = client
            .base_url
            .join(&format!("v1/courses/{id}"))
            .map_err(|_| CourseProviderError::Unavailable)?;
        let provider: ProviderCourse = client.get_json(client.http.get(url), true).await?;
        let course = provider.normalize(id)?;
        client.cache_course(id.to_owned(), course.clone()).await;
        Ok(course)
    }

    fn require_configured(&self) -> Result<&Arc<ConfiguredClient>, CourseProviderError> {
        self.configured
            .as_ref()
            .ok_or(CourseProviderError::Unavailable)
    }
}

impl ConfiguredClient {
    async fn get_json<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        not_found_is_course: bool,
    ) -> Result<T, CourseProviderError> {
        let _permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| CourseProviderError::Saturated)?;
        self.consume_quota().await?;
        let response = request.send().await.map_err(map_reqwest_error)?;
        match response.status() {
            StatusCode::OK => {}
            StatusCode::TOO_MANY_REQUESTS => {
                self.exhaust_quota().await;
                return Err(CourseProviderError::Exhausted);
            }
            StatusCode::NOT_FOUND if not_found_is_course => {
                return Err(CourseProviderError::NotFound);
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(CourseProviderError::Unavailable);
            }
            _ => return Err(CourseProviderError::Upstream),
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(CourseProviderError::InvalidResponse);
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(map_reqwest_error)?;
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(CourseProviderError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_| CourseProviderError::InvalidResponse)
    }

    async fn cached_search(&self, key: &str) -> Option<Vec<CourseSearchResult>> {
        let mut cache = self.cache.lock().await;
        cache.search(key)
    }

    async fn cached_course(&self, key: &str) -> Option<CourseDetail> {
        let mut cache = self.cache.lock().await;
        cache.course(key)
    }

    async fn cache_search(&self, key: String, value: Vec<CourseSearchResult>) {
        let mut cache = self.cache.lock().await;
        cache.insert_search(key, value);
    }

    async fn cache_course(&self, key: String, value: CourseDetail) {
        let mut cache = self.cache.lock().await;
        cache.insert_course(key, value);
    }

    async fn consume_quota(&self) -> Result<(), CourseProviderError> {
        let today = Utc::now().date_naive();
        let mut quota = self.quota.lock().await;
        if quota.utc_date != today {
            quota.utc_date = today;
            quota.used = 0;
        }
        if quota.used >= quota.limit {
            return Err(CourseProviderError::Exhausted);
        }
        quota.used += 1;
        Ok(())
    }

    async fn exhaust_quota(&self) {
        let mut quota = self.quota.lock().await;
        quota.utc_date = Utc::now().date_naive();
        quota.used = quota.limit;
    }
}

fn map_reqwest_error(error: reqwest::Error) -> CourseProviderError {
    if error.is_timeout() {
        CourseProviderError::Timeout
    } else {
        CourseProviderError::Upstream
    }
}

pub fn validate_search_query(value: &str) -> Result<&str, &'static str> {
    let value = value.trim();
    if !(2..=80).contains(&value.len()) || value.chars().any(char::is_control) {
        return Err("q must contain between 2 and 80 bytes without control characters");
    }
    Ok(value)
}

pub fn normalize_course_id(value: &str) -> Result<String, &'static str> {
    let normalized = value.to_ascii_lowercase();
    let valid = normalized.len() == 8
        && normalized
            .bytes()
            .all(|byte| b"0123456789abcdefghjkmnpqrstvwxyz".contains(&byte))
        && normalized.bytes().any(|byte| byte.is_ascii_alphabetic());
    if !valid {
        return Err("course id is invalid");
    }
    Ok(normalized)
}
