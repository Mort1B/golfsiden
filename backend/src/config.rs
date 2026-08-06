use std::env;

use axum::http::HeaderValue;
use thiserror::Error;

use crate::auth::AuthConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub port: u16,
    pub run_migrations: bool,
    pub auth: AuthConfig,
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
        })
    }
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
