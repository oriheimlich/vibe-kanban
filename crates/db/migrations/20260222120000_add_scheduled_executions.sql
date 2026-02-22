CREATE TABLE scheduled_executions (
    id                    BLOB PRIMARY KEY,
    task_id               BLOB NOT NULL,
    project_id            BLOB NOT NULL,
    scheduled_at          TEXT NOT NULL,       -- ISO8601 UTC
    status                TEXT NOT NULL DEFAULT 'pending'
                             CHECK (status IN ('pending', 'fired', 'cancelled')),
    executor_profile_id   TEXT NOT NULL,       -- JSON ExecutorProfileId
    repos                 TEXT NOT NULL,        -- JSON Vec<WorkspaceRepoInput>
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    fired_at              TEXT,
    error_message         TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_scheduled_executions_pending
    ON scheduled_executions(status, scheduled_at)
    WHERE status = 'pending';
