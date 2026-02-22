use std::time::Duration;

use db::{
    DBService,
    models::{
        repo::Repo,
        scheduled_execution::ScheduledExecution,
        task::{Task, TaskStatus},
        workspace::{CreateWorkspace, Workspace},
        workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo},
    },
};
use executors::profile::ExecutorProfileId;
use serde::{Deserialize, Serialize};
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::interval;
use tracing::{error, info};
use uuid::Uuid;

use crate::services::container::ContainerService;

#[derive(Debug, Error)]
enum SchedulerError {
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),
    #[error("Failed to deserialize executor_profile_id: {0}")]
    DeserializeExecutorProfile(String),
    #[error("Failed to deserialize repos: {0}")]
    DeserializeRepos(String),
    #[error("Repo not found: {0}")]
    RepoNotFound(Uuid),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Repo input stored as JSON in the scheduled_execution record.
/// Uses camelCase to match the frontend's serialization format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRepoInput {
    pub repo_id: Uuid,
    pub target_branch: String,
}

/// Service that polls for pending scheduled executions and fires them.
///
/// Generic over `C: ContainerService` so it can be instantiated from the
/// deployment layer (which knows the concrete container type).
pub struct SchedulerService<C: ContainerService> {
    db: DBService,
    container: C,
    poll_interval: Duration,
}

impl<C: ContainerService + Send + Sync + 'static> SchedulerService<C> {
    pub fn spawn(db: DBService, container: C) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            container,
            poll_interval: Duration::from_secs(15),
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting scheduler service with interval {:?}",
            self.poll_interval
        );

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_pending().await {
                error!("Error checking pending scheduled executions: {}", e);
            }
        }
    }

    async fn check_pending(&self) -> Result<(), SchedulerError> {
        let due = ScheduledExecution::find_pending_due(&self.db.pool).await?;

        if due.is_empty() {
            return Ok(());
        }

        info!("Found {} pending scheduled executions to fire", due.len());

        for scheduled in due {
            if let Err(e) = self.fire_scheduled_task(&scheduled).await {
                error!(
                    "Error firing scheduled execution {} for task {}: {}",
                    scheduled.id, scheduled.task_id, e
                );
                let msg = format!("{}", e);
                if let Err(mark_err) =
                    ScheduledExecution::mark_error(&self.db.pool, scheduled.id, &msg).await
                {
                    error!(
                        "Failed to mark scheduled execution {} as error: {}",
                        scheduled.id, mark_err
                    );
                }
            }
        }

        Ok(())
    }

    async fn fire_scheduled_task(
        &self,
        scheduled: &ScheduledExecution,
    ) -> Result<(), SchedulerError> {
        let pool = &self.db.pool;

        // 1. Deserialize executor_profile_id and repos from JSON
        let executor_profile_id: ExecutorProfileId =
            serde_json::from_str(&scheduled.executor_profile_id).map_err(|e| {
                SchedulerError::DeserializeExecutorProfile(format!("{}", e))
            })?;

        let repos: Vec<ScheduledRepoInput> =
            serde_json::from_str(&scheduled.repos).map_err(|e| {
                SchedulerError::DeserializeRepos(format!("{}", e))
            })?;

        // 2. Verify task still exists and is in Todo status
        let task = Task::find_by_id(pool, scheduled.task_id)
            .await?
            .ok_or(SchedulerError::TaskNotFound(scheduled.task_id))?;

        if task.status != TaskStatus::Todo {
            info!(
                "Skipping scheduled execution {} — task {} is in {:?} status, not Todo",
                scheduled.id, task.id, task.status
            );
            ScheduledExecution::mark_cancelled(pool, scheduled.id).await?;
            return Ok(());
        }

        // 3. Create workspace (same logic as create_task_and_start)
        let workspace_id = Uuid::new_v4();
        let git_branch_name = self
            .container
            .git_branch_from_workspace(&workspace_id, &task.title)
            .await;

        // Compute agent_working_dir based on repo count
        let agent_working_dir = if repos.len() == 1 {
            let repo = Repo::find_by_id(pool, repos[0].repo_id)
                .await?
                .ok_or(SchedulerError::RepoNotFound(repos[0].repo_id))?;
            Some(repo.name)
        } else {
            None
        };

        let workspace = Workspace::create(
            pool,
            &CreateWorkspace {
                branch: git_branch_name,
                agent_working_dir,
            },
            workspace_id,
            task.id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create workspace: {}", e))?;

        let workspace_repos: Vec<CreateWorkspaceRepo> = repos
            .iter()
            .map(|r| CreateWorkspaceRepo {
                repo_id: r.repo_id,
                target_branch: r.target_branch.clone(),
            })
            .collect();
        WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;

        // 4. Start workspace
        match self
            .container
            .start_workspace(&workspace, executor_profile_id)
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully fired scheduled execution {} for task {}",
                    scheduled.id, task.id
                );
                ScheduledExecution::mark_fired(pool, scheduled.id).await?;
            }
            Err(e) => {
                let msg = format!("Failed to start workspace: {}", e);
                error!(
                    "Scheduled execution {} failed to start workspace: {}",
                    scheduled.id, msg
                );
                ScheduledExecution::mark_error(pool, scheduled.id, &msg).await?;
            }
        }

        Ok(())
    }
}
