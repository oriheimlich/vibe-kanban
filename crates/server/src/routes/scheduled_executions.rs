use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
};
use chrono::{DateTime, Utc};
use db::models::scheduled_execution::{ScheduledExecution, ScheduledExecutionStatus};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

use deployment::Deployment;

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduledExecutionRequest {
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub scheduled_at: DateTime<Utc>,
    pub executor_profile_id: serde_json::Value,
    pub repos: Vec<ScheduledRepoInput>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRepoInput {
    pub repo_id: Uuid,
    pub target_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct ScheduledExecutionQuery {
    pub project_id: Uuid,
}

#[axum::debug_handler]
pub async fn create_scheduled_execution(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateScheduledExecutionRequest>,
) -> Result<ResponseJson<ApiResponse<ScheduledExecution>>, ApiError> {
    let pool = &deployment.db().pool;

    if payload.repos.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one repository is required".to_string(),
        ));
    }

    if payload.scheduled_at <= Utc::now() {
        return Err(ApiError::BadRequest(
            "scheduled_at must be in the future".to_string(),
        ));
    }

    let id = Uuid::new_v4();
    let executor_profile_id_json = serde_json::to_string(&payload.executor_profile_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid executor_profile_id: {}", e)))?;
    let repos_json = serde_json::to_string(&payload.repos)
        .map_err(|e| ApiError::BadRequest(format!("Invalid repos: {}", e)))?;

    let scheduled = ScheduledExecution::create(
        pool,
        id,
        payload.task_id,
        payload.project_id,
        payload.scheduled_at,
        &executor_profile_id_json,
        &repos_json,
    )
    .await?;

    tracing::info!(
        "Created scheduled execution {} for task {} at {}",
        scheduled.id,
        scheduled.task_id,
        scheduled.scheduled_at
    );

    Ok(ResponseJson(ApiResponse::success(scheduled)))
}

#[axum::debug_handler]
pub async fn list_scheduled_executions(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ScheduledExecutionQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<ScheduledExecution>>>, ApiError> {
    let pool = &deployment.db().pool;
    let executions =
        ScheduledExecution::find_by_project_id(pool, query.project_id).await?;
    Ok(ResponseJson(ApiResponse::success(executions)))
}

#[axum::debug_handler]
pub async fn get_scheduled_execution(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ScheduledExecution>>, ApiError> {
    let pool = &deployment.db().pool;
    let scheduled = ScheduledExecution::find_by_id(pool, id)
        .await?
        .ok_or(ApiError::BadRequest(
            "Scheduled execution not found".to_string(),
        ))?;
    Ok(ResponseJson(ApiResponse::success(scheduled)))
}

#[axum::debug_handler]
pub async fn cancel_scheduled_execution(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let scheduled = ScheduledExecution::find_by_id(pool, id)
        .await?
        .ok_or(ApiError::BadRequest(
            "Scheduled execution not found".to_string(),
        ))?;

    if scheduled.status != ScheduledExecutionStatus::Pending {
        return Err(ApiError::BadRequest(format!(
            "Cannot cancel a scheduled execution with status '{}'",
            scheduled.status
        )));
    }

    ScheduledExecution::mark_cancelled(pool, id).await?;

    tracing::info!("Cancelled scheduled execution {}", id);

    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let item_router = Router::new()
        .route("/", get(get_scheduled_execution).delete(cancel_scheduled_execution));

    let collection_router = Router::new()
        .route("/", get(list_scheduled_executions).post(create_scheduled_execution))
        .nest("/{id}", item_router);

    Router::new().nest("/scheduled-executions", collection_router)
}
