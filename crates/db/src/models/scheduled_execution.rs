use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

#[derive(
    Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default,
)]
#[sqlx(type_name = "scheduled_execution_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ScheduledExecutionStatus {
    #[default]
    Pending,
    Fired,
    Cancelled,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ScheduledExecution {
    pub id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub scheduled_at: DateTime<Utc>,
    pub status: ScheduledExecutionStatus,
    pub executor_profile_id: String, // JSON ExecutorProfileId
    pub repos: String,               // JSON Vec<WorkspaceRepoInput>
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub fired_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl ScheduledExecution {
    pub async fn create(
        pool: &SqlitePool,
        id: Uuid,
        task_id: Uuid,
        project_id: Uuid,
        scheduled_at: DateTime<Utc>,
        executor_profile_id_json: &str,
        repos_json: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            ScheduledExecution,
            r#"INSERT INTO scheduled_executions (id, task_id, project_id, scheduled_at, executor_profile_id, repos)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING
                   id                  AS "id!: Uuid",
                   task_id             AS "task_id!: Uuid",
                   project_id          AS "project_id!: Uuid",
                   scheduled_at        AS "scheduled_at!: DateTime<Utc>",
                   status              AS "status!: ScheduledExecutionStatus",
                   executor_profile_id,
                   repos,
                   created_at          AS "created_at!: DateTime<Utc>",
                   updated_at          AS "updated_at!: DateTime<Utc>",
                   fired_at            AS "fired_at: DateTime<Utc>",
                   error_message"#,
            id,
            task_id,
            project_id,
            scheduled_at,
            executor_profile_id_json,
            repos_json,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ScheduledExecution,
            r#"SELECT
                   id                  AS "id!: Uuid",
                   task_id             AS "task_id!: Uuid",
                   project_id          AS "project_id!: Uuid",
                   scheduled_at        AS "scheduled_at!: DateTime<Utc>",
                   status              AS "status!: ScheduledExecutionStatus",
                   executor_profile_id,
                   repos,
                   created_at          AS "created_at!: DateTime<Utc>",
                   updated_at          AS "updated_at!: DateTime<Utc>",
                   fired_at            AS "fired_at: DateTime<Utc>",
                   error_message
               FROM scheduled_executions
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_pending_due(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        let now = Utc::now();
        sqlx::query_as!(
            ScheduledExecution,
            r#"SELECT
                   id                  AS "id!: Uuid",
                   task_id             AS "task_id!: Uuid",
                   project_id          AS "project_id!: Uuid",
                   scheduled_at        AS "scheduled_at!: DateTime<Utc>",
                   status              AS "status!: ScheduledExecutionStatus",
                   executor_profile_id,
                   repos,
                   created_at          AS "created_at!: DateTime<Utc>",
                   updated_at          AS "updated_at!: DateTime<Utc>",
                   fired_at            AS "fired_at: DateTime<Utc>",
                   error_message
               FROM scheduled_executions
               WHERE status = 'pending'
                 AND scheduled_at <= $1
               ORDER BY scheduled_at ASC"#,
            now
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_pending_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ScheduledExecution,
            r#"SELECT
                   id                  AS "id!: Uuid",
                   task_id             AS "task_id!: Uuid",
                   project_id          AS "project_id!: Uuid",
                   scheduled_at        AS "scheduled_at!: DateTime<Utc>",
                   status              AS "status!: ScheduledExecutionStatus",
                   executor_profile_id,
                   repos,
                   created_at          AS "created_at!: DateTime<Utc>",
                   updated_at          AS "updated_at!: DateTime<Utc>",
                   fired_at            AS "fired_at: DateTime<Utc>",
                   error_message
               FROM scheduled_executions
               WHERE task_id = $1
                 AND status = 'pending'
               LIMIT 1"#,
            task_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_fired(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE scheduled_executions SET status = 'fired', fired_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_cancelled(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE scheduled_executions SET status = 'cancelled', updated_at = datetime('now', 'subsec') WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ScheduledExecution,
            r#"SELECT
                   id                  AS "id!: Uuid",
                   task_id             AS "task_id!: Uuid",
                   project_id          AS "project_id!: Uuid",
                   scheduled_at        AS "scheduled_at!: DateTime<Utc>",
                   status              AS "status!: ScheduledExecutionStatus",
                   executor_profile_id,
                   repos,
                   created_at          AS "created_at!: DateTime<Utc>",
                   updated_at          AS "updated_at!: DateTime<Utc>",
                   fired_at            AS "fired_at: DateTime<Utc>",
                   error_message
               FROM scheduled_executions
               WHERE project_id = $1
               ORDER BY scheduled_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn mark_error(
        pool: &SqlitePool,
        id: Uuid,
        message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE scheduled_executions SET error_message = $2, fired_at = datetime('now', 'subsec'), status = 'fired', updated_at = datetime('now', 'subsec') WHERE id = $1",
            id,
            message
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
