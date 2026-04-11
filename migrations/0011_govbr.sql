CREATE TYPE zain.govbr_nivel AS ENUM ('bronze', 'prata', 'ouro');

CREATE DOMAIN zain.govbr_session AS JSONB;

CREATE TABLE zain.govbr (
    cpf             TEXT PRIMARY KEY,
    password        TEXT NOT NULL,
    otp             TEXT,
    session         zain.govbr_session,
    nome            TEXT,
    email           TEXT,
    telefone        TEXT,
    nivel           zain.govbr_nivel,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
