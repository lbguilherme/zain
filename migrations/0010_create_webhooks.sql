CREATE TABLE whatsapp.webhooks (
    id          BIGSERIAL PRIMARY KEY,
    body        JSONB NOT NULL,
    processed   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhooks_unprocessed ON whatsapp.webhooks(created_at)
    WHERE processed = false;
