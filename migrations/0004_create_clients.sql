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
