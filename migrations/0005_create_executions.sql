CREATE TABLE zain.executions (
    id              BIGSERIAL PRIMARY KEY,
    client_id       BIGINT NOT NULL REFERENCES zain.clients(id),
    state_before    TEXT NOT NULL,
    state_after     TEXT,
    trigger_type    TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'running',
    error           TEXT,
    llm_messages    JSONB,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ
);

CREATE INDEX idx_executions_running ON zain.executions(status)
    WHERE status = 'running';
