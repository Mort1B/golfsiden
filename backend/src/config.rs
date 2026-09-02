use std::env;

use axum::http::{HeaderValue, Uri};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use thiserror::Error;

use crate::{auth::AuthConfig, proxy::ProxyTrustConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Production,
}

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
    pub environment: AppEnvironment,
    pub database_url: String,
    pub database_max_connections: u32,
    pub expected_database_user: Option<String>,
    pub port: u16,
    pub run_migrations: bool,
    pub auth: AuthConfig,
    pub course_provider: CourseProviderConfig,
    pub proxy_trust: ProxyTrustConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {name}: {value}")]
    Invalid { name: &'static str, value: String },
}

#[derive(Default)]
struct RawConfig {
    app_env: Option<String>,
    database_url: Option<String>,
    database_max_connections: Option<String>,
    app_database_user: Option<String>,
    port: Option<String>,
    run_migrations: Option<String>,
    session_cookie_secure: Option<String>,
    session_ttl_hours: Option<String>,
    cors_allowed_origin: Option<String>,
    course_provider_api_key: Option<String>,
    course_provider_daily_limit: Option<String>,
    proxy_shared_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_raw(RawConfig {
            app_env: env::var("APP_ENV").ok(),
            database_url: env::var("DATABASE_URL").ok(),
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS").ok(),
            app_database_user: env::var("APP_DATABASE_USER").ok(),
            port: env::var("PORT").ok(),
            run_migrations: env::var("RUN_MIGRATIONS").ok(),
            session_cookie_secure: env::var("SESSION_COOKIE_SECURE").ok(),
            session_ttl_hours: env::var("SESSION_TTL_HOURS").ok(),
            cors_allowed_origin: env::var("CORS_ALLOWED_ORIGIN").ok(),
            course_provider_api_key: env::var("GOLF_COURSE_API_KEY").ok(),
            course_provider_daily_limit: env::var("GOLF_COURSE_API_DAILY_LIMIT").ok(),
            proxy_shared_secret: env::var("PROXY_SHARED_SECRET").ok(),
        })
    }

    fn from_raw(raw: RawConfig) -> Result<Self, ConfigError> {
        let environment = parse_environment(raw.app_env)?;
        let session_ttl_hours = parse_value("SESSION_TTL_HOURS", raw.session_ttl_hours, 24_i64)?;
        if !(1..=720).contains(&session_ttl_hours) {
            return Err(invalid("SESSION_TTL_HOURS", session_ttl_hours));
        }
        let run_migrations = parse_value("RUN_MIGRATIONS", raw.run_migrations, false)?;
        let cookie_secure = parse_value("SESSION_COOKIE_SECURE", raw.session_cookie_secure, true)?;
        if environment == AppEnvironment::Production && !cookie_secure {
            return Err(invalid("SESSION_COOKIE_SECURE", false));
        }
        if environment == AppEnvironment::Production && run_migrations {
            return Err(invalid("RUN_MIGRATIONS", true));
        }
        let cors_allowed_origin = parse_origin(raw.cors_allowed_origin, environment)?;
        let daily_limit = parse_value(
            "GOLF_COURSE_API_DAILY_LIMIT",
            raw.course_provider_daily_limit,
            50_u32,
        )?;
        if !(1..=1_000_000).contains(&daily_limit) {
            return Err(invalid("GOLF_COURSE_API_DAILY_LIMIT", daily_limit));
        }

        let database_max_connections = parse_value(
            "DATABASE_MAX_CONNECTIONS",
            raw.database_max_connections,
            10_u32,
        )?;
        if !(1..=100).contains(&database_max_connections) {
            return Err(invalid(
                "DATABASE_MAX_CONNECTIONS",
                database_max_connections,
            ));
        }
        let port = parse_value("PORT", raw.port, 3000_u16)?;
        if environment == AppEnvironment::Production && port == 0 {
            return Err(invalid("PORT", port));
        }
        let proxy_trust = parse_proxy_trust(raw.proxy_shared_secret, environment)?;
        let expected_database_user = match (environment, optional_secret(raw.app_database_user)) {
            (AppEnvironment::Production, None) => {
                return Err(ConfigError::Missing("APP_DATABASE_USER"));
            }
            (_, value) => value,
        };

        Ok(Self {
            environment,
            database_url: raw
                .database_url
                .ok_or(ConfigError::Missing("DATABASE_URL"))?,
            database_max_connections,
            expected_database_user,
            port,
            run_migrations,
            auth: AuthConfig {
                cookie_secure,
                session_ttl_hours,
                cors_allowed_origin,
            },
            course_provider: CourseProviderConfig {
                api_key: optional_secret(raw.course_provider_api_key),
                daily_limit,
            },
            proxy_trust,
        })
    }
}

fn parse_proxy_trust(
    value: Option<String>,
    environment: AppEnvironment,
) -> Result<ProxyTrustConfig, ConfigError> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return if environment == AppEnvironment::Production {
            Err(ConfigError::Missing("PROXY_SHARED_SECRET"))
        } else {
            Ok(ProxyTrustConfig::direct())
        };
    };
    let valid = value.len() == 43
        && matches!(URL_SAFE_NO_PAD.decode(value.as_bytes()), Ok(bytes) if bytes.len() == 32);
    if !valid {
        return Err(ConfigError::Invalid {
            name: "PROXY_SHARED_SECRET",
            value: "[redacted]".to_owned(),
        });
    }
    Ok(ProxyTrustConfig::trusted(value))
}

fn parse_environment(value: Option<String>) -> Result<AppEnvironment, ConfigError> {
    match value.as_deref().unwrap_or("development") {
        "development" => Ok(AppEnvironment::Development),
        "production" => Ok(AppEnvironment::Production),
        value => Err(ConfigError::Invalid {
            name: "APP_ENV",
            value: value.to_owned(),
        }),
    }
}

fn parse_origin(
    value: Option<String>,
    environment: AppEnvironment,
) -> Result<Option<HeaderValue>, ConfigError> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let uri: Uri = value.parse().map_err(|_| ConfigError::Invalid {
        name: "CORS_ALLOWED_ORIGIN",
        value: value.clone(),
    })?;
    let scheme = uri.scheme_str();
    let valid_scheme = matches!(scheme, Some("http" | "https"));
    let exact_origin = match (scheme, uri.authority()) {
        (Some(scheme), Some(authority)) if !authority.as_str().contains('@') => {
            value == format!("{scheme}://{authority}")
        }
        _ => false,
    };
    let production_https = environment != AppEnvironment::Production || scheme == Some("https");
    if !valid_scheme || !exact_origin || !production_https {
        return Err(ConfigError::Invalid {
            name: "CORS_ALLOWED_ORIGIN",
            value,
        });
    }
    value.parse().map(Some).map_err(|_| ConfigError::Invalid {
        name: "CORS_ALLOWED_ORIGIN",
        value,
    })
}

fn optional_secret(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_value<T>(name: &'static str, value: Option<String>, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    match value {
        Some(value) => value
            .parse()
            .map_err(|_| ConfigError::Invalid { name, value }),
        None => Ok(default),
    }
}

fn invalid(name: &'static str, value: impl ToString) -> ConfigError {
    ConfigError::Invalid {
        name,
        value: value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(environment: &str) -> RawConfig {
        RawConfig {
            app_env: Some(environment.to_owned()),
            database_url: Some("postgres://example.invalid/golf".to_owned()),
            proxy_shared_secret: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned()),
            app_database_user: Some("golfsiden_app".to_owned()),
            ..RawConfig::default()
        }
    }

    #[test]
    fn production_rejects_insecure_cookies_and_implicit_migrations() {
        let mut insecure = raw("production");
        insecure.session_cookie_secure = Some("false".to_owned());
        assert!(matches!(
            Config::from_raw(insecure),
            Err(ConfigError::Invalid {
                name: "SESSION_COOKIE_SECURE",
                ..
            })
        ));

        let mut migrations = raw("production");
        migrations.run_migrations = Some("true".to_owned());
        assert!(matches!(
            Config::from_raw(migrations),
            Err(ConfigError::Invalid {
                name: "RUN_MIGRATIONS",
                ..
            })
        ));
    }

    #[test]
    fn cors_requires_an_exact_origin_and_https_in_production() {
        for origin in [
            "https://user@example.test",
            "https://example.test/path",
            "https://example.test?query=yes",
            "ftp://example.test",
            "https://example.test/#fragment",
        ] {
            let mut value = raw("production");
            value.cors_allowed_origin = Some(origin.to_owned());
            assert!(Config::from_raw(value).is_err(), "accepted {origin}");
        }
        let mut http = raw("production");
        http.cors_allowed_origin = Some("http://example.test".to_owned());
        assert!(Config::from_raw(http).is_err());

        let mut https = raw("production");
        https.cors_allowed_origin = Some("https://example.test:8443".to_owned());
        assert_eq!(
            Config::from_raw(https)
                .unwrap()
                .auth
                .cors_allowed_origin
                .unwrap(),
            "https://example.test:8443"
        );
    }

    #[test]
    fn development_keeps_local_security_overrides_compatible() {
        let mut value = raw("development");
        value.session_cookie_secure = Some("false".to_owned());
        value.run_migrations = Some("true".to_owned());
        value.cors_allowed_origin = Some("http://127.0.0.1:5173".to_owned());
        let config = Config::from_raw(value).unwrap();
        assert!(!config.auth.cookie_secure);
        assert!(config.run_migrations);
        assert_eq!(config.environment, AppEnvironment::Development);
    }

    #[test]
    fn rejects_unusable_pool_and_production_port_values() {
        for connections in ["0", "101"] {
            let mut value = raw("production");
            value.database_max_connections = Some(connections.to_owned());
            assert!(matches!(
                Config::from_raw(value),
                Err(ConfigError::Invalid {
                    name: "DATABASE_MAX_CONNECTIONS",
                    ..
                })
            ));
        }
        let mut value = raw("production");
        value.port = Some("0".to_owned());
        assert!(matches!(
            Config::from_raw(value),
            Err(ConfigError::Invalid { name: "PORT", .. })
        ));
    }

    #[test]
    fn production_requires_a_structurally_strong_proxy_secret_without_echoing_it() {
        let mut missing = raw("production");
        missing.proxy_shared_secret = None;
        assert!(matches!(
            Config::from_raw(missing),
            Err(ConfigError::Missing("PROXY_SHARED_SECRET"))
        ));

        let mut short = raw("production");
        short.proxy_shared_secret = Some("not-secret-enough".to_owned());
        assert!(matches!(
            Config::from_raw(short),
            Err(ConfigError::Invalid {
                name: "PROXY_SHARED_SECRET",
                value,
            }) if value == "[redacted]"
        ));
    }

    #[test]
    fn production_requires_the_expected_runtime_database_role() {
        let mut missing = raw("production");
        missing.app_database_user = None;
        assert!(matches!(
            Config::from_raw(missing),
            Err(ConfigError::Missing("APP_DATABASE_USER"))
        ));

        let mut blank = raw("production");
        blank.app_database_user = Some("  ".to_owned());
        assert!(matches!(
            Config::from_raw(blank),
            Err(ConfigError::Missing("APP_DATABASE_USER"))
        ));
    }
}
