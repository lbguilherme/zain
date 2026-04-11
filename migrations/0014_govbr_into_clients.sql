-- Move os dados do gov.br para colunas em `zain.clients` e elimina a
-- tabela dedicada. O `zain.govbr` existia como tabela lateral presumindo
-- que uma mesma credencial pudesse ser compartilhada entre leads; na
-- prática cada lead tem o seu próprio gov.br e fica mais simples manter
-- tudo junto.

ALTER TABLE zain.clients
    ADD COLUMN govbr_cpf              TEXT,
    ADD COLUMN govbr_password         TEXT,
    ADD COLUMN govbr_otp              TEXT,
    ADD COLUMN govbr_session          zain.govbr_session,
    ADD COLUMN govbr_session_valid_at TIMESTAMPTZ,
    ADD COLUMN govbr_nome             TEXT,
    ADD COLUMN govbr_email            TEXT,
    ADD COLUMN govbr_telefone         TEXT,
    ADD COLUMN govbr_nivel            zain.govbr_nivel;

DROP TABLE zain.govbr;
