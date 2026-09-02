use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use thiserror::Error;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");

#[derive(Debug, Error)]
pub enum SchemaCompatibilityError {
    #[error("database schema migration history is missing")]
    MissingHistory,
    #[error("database migration {0} did not complete successfully")]
    Dirty(i64),
    #[error("database contains unknown migration {0}")]
    Unknown(i64),
    #[error("database migration {0} does not match the compiled migration")]
    ChecksumMismatch(i64),
    #[error("database is missing compiled migration {0}")]
    Pending(i64),
    #[error("database schema compatibility query failed")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum RuntimeAuthorityError {
    #[error("database runtime role does not match production configuration")]
    UnexpectedRole,
    #[error("database runtime role has prohibited cluster or schema privileges")]
    ExcessPrivileges,
    #[error("database runtime role can modify migration history")]
    MigrationHistoryWritable,
    #[error("database runtime authority query failed")]
    Database(#[source] sqlx::Error),
}

pub async fn check_compatibility(pool: &PgPool) -> Result<(), SchemaCompatibilityError> {
    let history_exists =
        sqlx::query_scalar::<_, bool>("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(SchemaCompatibilityError::Database)?;
    if !history_exists {
        return Err(SchemaCompatibilityError::MissingHistory);
    }

    let rows = sqlx::query_as::<_, (i64, bool, Vec<u8>)>(
        "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .map_err(SchemaCompatibilityError::Database)?;
    let compiled: HashMap<_, _> = MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| (migration.version, migration.checksum.as_ref()))
        .collect();
    let mut applied = HashSet::with_capacity(rows.len());

    for (version, success, checksum) in rows {
        if !success {
            return Err(SchemaCompatibilityError::Dirty(version));
        }
        let Some(expected_checksum) = compiled.get(&version) else {
            return Err(SchemaCompatibilityError::Unknown(version));
        };
        if checksum.as_slice() != *expected_checksum {
            return Err(SchemaCompatibilityError::ChecksumMismatch(version));
        }
        applied.insert(version);
    }

    if let Some(version) = compiled
        .keys()
        .copied()
        .filter(|version| !applied.contains(version))
        .min()
    {
        return Err(SchemaCompatibilityError::Pending(version));
    }
    Ok(())
}

pub async fn check_runtime_authority(
    pool: &PgPool,
    expected_user: &str,
) -> Result<(), RuntimeAuthorityError> {
    let (current_user, superuser, create_db, create_role, schema_create, history_write) =
        sqlx::query_as::<_, (String, bool, bool, bool, bool, bool)>(
            "SELECT current_user,
                    role.rolsuper,
                    role.rolcreatedb,
                    role.rolcreaterole,
                    has_schema_privilege(current_user, 'public', 'CREATE'),
                    has_table_privilege(current_user, 'public._sqlx_migrations', 'INSERT')
                      OR has_table_privilege(current_user, 'public._sqlx_migrations', 'UPDATE')
                      OR has_table_privilege(current_user, 'public._sqlx_migrations', 'DELETE')
                      OR has_table_privilege(current_user, 'public._sqlx_migrations', 'TRUNCATE')
               FROM pg_roles role
              WHERE role.rolname = current_user",
        )
        .fetch_one(pool)
        .await
        .map_err(RuntimeAuthorityError::Database)?;
    if current_user != expected_user {
        return Err(RuntimeAuthorityError::UnexpectedRole);
    }
    if superuser || create_db || create_role || schema_create {
        return Err(RuntimeAuthorityError::ExcessPrivileges);
    }
    if history_write {
        return Err(RuntimeAuthorityError::MigrationHistoryWritable);
    }
    Ok(())
}
