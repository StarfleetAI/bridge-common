use chrono::Utc;
use serde_json::Value;
use sqlx::{query, query_as, Executor, Postgres};
use uuid::Uuid;

use crate::{settings::Settings, types::Result};

struct SettingsRow {
    value: Value,
}

/// Get settings for a company.
///
/// # Errors
///
/// Returns error if there was a problem while fetching settings.
pub async fn get<'a, E>(executor: E, company_id: Uuid) -> Result<Settings>
where
    E: Executor<'a, Database = Postgres> + std::marker::Copy,
{
    match query_as!(
        SettingsRow,
        "SELECT value FROM settings WHERE company_id = $1 LIMIT 1",
        company_id,
    )
    .fetch_optional(executor)
    .await?
    {
        Some(row) => Ok(Settings::try_from(row.value)?),
        None => {
            let settings = Settings::default();
            insert(executor, company_id, &settings).await?;

            Ok(settings)
        }
    }
}

/// Insert settings for a company.
///
/// # Errors
///
/// Returns error if there was a problem while inserting settings.
pub async fn insert<'a, E>(executor: E, company_id: Uuid, settings: &Settings) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "INSERT INTO settings (company_id, value, created_at, updated_at) VALUES ($1, $2, $3, $4)",
        company_id,
        serde_json::to_value(settings)?,
        Utc::now(),
        Utc::now(),
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Update settings for a company.
///
/// # Errors
///
/// Returns error if there was a problem while updating settings.
pub async fn update<'a, E>(executor: E, company_id: Uuid, settings: &Settings) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "UPDATE settings SET value = $1, updated_at = $2 WHERE company_id = $3",
        serde_json::to_value(settings)?,
        Utc::now(),
        company_id,
    )
    .execute(executor)
    .await?;

    Ok(())
}
