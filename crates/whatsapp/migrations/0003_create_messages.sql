CREATE TABLE whatsapp.messages (
    id          TEXT PRIMARY KEY,
    chat_id     TEXT NOT NULL REFERENCES whatsapp.chats(id),
    type        TEXT NOT NULL DEFAULT 'text',
    subtype     TEXT,
    from_number TEXT,
    from_me     BOOLEAN NOT NULL DEFAULT false,
    from_name   TEXT,
    timestamp   BIGINT,
    source      TEXT,
    status      TEXT,
    text_body   TEXT,
    has_media   BOOLEAN NOT NULL DEFAULT false,
    media_mime  TEXT,
    media_url   TEXT,
    context_quoted_id   TEXT,
    context_forwarded   BOOLEAN,
    raw         JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_whatsapp_messages_chat_id ON whatsapp.messages(chat_id);
CREATE INDEX idx_whatsapp_messages_timestamp ON whatsapp.messages(timestamp);
