CREATE TABLE whatsapp.chats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES whatsapp.accounts(id),
    chat_jid TEXT,
    title TEXT NOT NULL,
    avatar TEXT,
    displayed_timestamp TIMESTAMPTZ,
    displayed_last_message TEXT,
    chat_type TEXT NOT NULL DEFAULT 'unknown',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE whatsapp.messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL REFERENCES whatsapp.chats(id),
    account_id UUID NOT NULL REFERENCES whatsapp.accounts(id),
    message_id TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'text',
    timestamp TIMESTAMPTZ,
    data JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (account_id, message_id)
);
