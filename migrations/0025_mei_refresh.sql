-- Suporte ao refresh recorrente da situação MEI em background.
--
-- 1. Elegibilidade para abertura de MEI: hoje é calculada em `auth_govbr`
--    (via `checar_pode_abrir_mei`) mas o resultado só volta no JSON da
--    resposta e some. Persistir permite o `get_client_state` reportar
--    "impedido de abrir MEI" (+motivo) como leitura SQL pura, e dá um
--    timestamp (`mei_consultado_em`, de 0022) pra dirigir o recheck por
--    TTL no worker `jobs::mei_refresh`.
--
-- 2. `govbr_otp_pendente`: flag que quebra o loop de re-login infinito no
--    background. Quando um login fresco (cpf+senha) para num 2FA, o
--    worker marca essa flag e PARA de tentar relogar sozinho — só o
--    fluxo interativo (`auth_govbr`/`auth_govbr_otp`), com o cliente
--    presente pra digitar o código, religa. Um login bem-sucedido
--    (`save_success`) zera a flag.

ALTER TABLE zain.clients
    ADD COLUMN mei_pode_abrir         BOOLEAN,                      -- NULL = não verificado; irrelevante se já tem MEI
    ADD COLUMN mei_impedimento_motivo TEXT,                        -- texto exato do banner quando pode_abrir = false
    ADD COLUMN govbr_otp_pendente     BOOLEAN NOT NULL DEFAULT false;
