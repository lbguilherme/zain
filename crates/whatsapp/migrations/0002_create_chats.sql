CREATE TABLE whatsapp.chats (
    id          TEXT PRIMARY KEY,
    name        TEXT,
    type        TEXT NOT NULL DEFAULT 'unknown',
    timestamp   BIGINT,
    chat_pic    TEXT,
    pin         BOOLEAN NOT NULL DEFAULT false,
    mute        BOOLEAN NOT NULL DEFAULT false,
    archive     BOOLEAN NOT NULL DEFAULT false,
    unread      INTEGER NOT NULL DEFAULT 0,
    read_only   BOOLEAN NOT NULL DEFAULT false,
    last_message_id TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
