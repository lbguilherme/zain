CREATE SCHEMA IF NOT EXISTS whatsapp;

CREATE TABLE whatsapp.outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id         TEXT NOT NULL,
    content_type    TEXT NOT NULL DEFAULT 'text',
    content         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    attempts        INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    sent_message_id TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending ON whatsapp.outbox(status, created_at)
    WHERE status IN ('pending', 'failed');

CREATE TABLE whatsapp.webhook_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    body        JSONB NOT NULL,
    processed   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhook_events_unprocessed ON whatsapp.webhook_events(created_at)
    WHERE processed = false;
