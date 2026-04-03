CREATE SCHEMA IF NOT EXISTS whatsapp;

CREATE TABLE whatsapp.accounts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT,
    phone       TEXT,
    avatar_url  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
