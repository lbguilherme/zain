CREATE TABLE whatsapp.outbox (
    id              BIGSERIAL PRIMARY KEY,
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
