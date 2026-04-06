CREATE SCHEMA IF NOT EXISTS zain;

CREATE TABLE zain.clients (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id                     TEXT NOT NULL UNIQUE,
    phone                       TEXT,
    name                        TEXT,
    state                       TEXT NOT NULL DEFAULT 'LEAD',
    state_props                 JSONB NOT NULL DEFAULT '{}',
    memory                      JSONB NOT NULL DEFAULT '{}',
    needs_processing            BOOLEAN NOT NULL DEFAULT true,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_clients_needs_processing ON zain.clients(updated_at)
    WHERE needs_processing = true;

CREATE TABLE zain.executions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id       UUID NOT NULL REFERENCES zain.clients(id),
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
