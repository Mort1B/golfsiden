use std::env;

use axum::http::HeaderValue;
use thiserror::Error;

use crate::auth::AuthConfig;

#[derive(Clone)]
pub struct CourseProviderConfig {
    pub api_key: Option<String>,
    pub daily_limit: u32,
}

impl std::fmt::Debug for CourseProviderConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseProviderConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("daily_limit", &self.daily_limit)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub port: u16,
    pub run_migrations: bool,
    pub auth: AuthConfig,
    pub course_provider: CourseProviderConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {name}: {value}")]
    Invalid { name: &'static str, value: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let session_ttl_hours = parse_env("SESSION_TTL_HOURS", 24_i64)?;
        if !(1..=720).contains(&session_ttl_hours) {
            return Err(ConfigError::Invalid {
                name: "SESSION_TTL_HOURS",
                value: session_ttl_hours.to_string(),
            });
        }
        let cors_allowed_origin = optional_header("CORS_ALLOWED_ORIGIN")?;
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
            database_max_connections: parse_env("DATABASE_MAX_CONNECTIONS", 10)?,
            port: parse_env("PORT", 3000)?,
            run_migrations: parse_env("RUN_MIGRATIONS", false)?,
            auth: AuthConfig {
                cookie_secure: parse_env("SESSION_COOKIE_SECURE", true)?,
                session_ttl_hours,
                cors_allowed_origin,
            },
            course_provider: CourseProviderConfig {
                api_key: optional_secret("GOLF_COURSE_API_KEY"),
                daily_limit: parse_bounded_daily_limit()?,
            },
        })
    }
}

fn parse_bounded_daily_limit() -> Result<u32, ConfigError> {
    let value = parse_env("GOLF_COURSE_API_DAILY_LIMIT", 50_u32)?;
    if !(1..=1_000_000).contains(&value) {
        return Err(ConfigError::Invalid {
            name: "GOLF_COURSE_API_DAILY_LIMIT",
            value: value.to_string(),
        });
    }
    Ok(value)
}

fn optional_secret(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn optional_header(name: &'static str) -> Result<Option<HeaderValue>, ConfigError> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value
            .parse()
            .map(Some)
            .map_err(|_| ConfigError::Invalid { name, value }),
        _ => Ok(None),
    }
}

fn parse_env<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| ConfigError::Invalid { name, value }),
        Err(_) => Ok(default),
    }
}
