CREATE TABLE whatsapp.webhooks (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    body        JSONB NOT NULL,
    processed   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhooks_unprocessed ON whatsapp.webhooks(created_at)
    WHERE processed = false;
