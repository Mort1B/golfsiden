use std::env;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub port: u16,
    pub run_migrations: bool,
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
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
            database_max_connections: parse_env("DATABASE_MAX_CONNECTIONS", 10)?,
            port: parse_env("PORT", 3000)?,
            run_migrations: parse_env("RUN_MIGRATIONS", false)?,
        })
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
