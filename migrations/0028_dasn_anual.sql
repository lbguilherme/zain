-- Situação da DASN-SIMEI (Declaração Anual do MEI) por ano-calendário,
-- consolidada do portal dasnsimei.app pelo worker `jobs::dasn_refresh`,
-- pra que o `get_client_state` reporte anos entregues/pendentes como
-- leitura SQL pura (sem RPA no caminho de leitura).
--
-- Fonte: a tela "Iniciar" do wizard Declarar/Retificar embute, em cada
-- radio de ano, `data-tipo-declaracao` (Original = nunca entregue /
-- Retificadora = já entregue) e `data-situacao-especial-*`. Lemos todos
-- os anos de uma vez (acesso público por CNPJ + hCaptcha, sem gov.br).
--
-- ATENÇÃO: o portal lista uma janela fixa de ~5 anos, INDEPENDENTE de
-- quando o CNPJ virou MEI. Logo, "Original" não significa "em atraso" —
-- pode ser ano anterior à vigência do MEI. O cálculo de atraso cruza
-- estes dados com os períodos MEI (do certificado) e é feito na leitura
-- (`get_client_state`), não aqui — esta tabela guarda só o status cru.

CREATE TABLE zain.dasn_anual (
    client_id         UUID        NOT NULL REFERENCES zain.clients(id) ON DELETE CASCADE,
    ano               INTEGER     NOT NULL,
    entregue          BOOLEAN     NOT NULL,  -- tipo = 'Retificadora'
    tipo              TEXT        NOT NULL,  -- 'Original' | 'Retificadora'
    situacao_especial TEXT,                  -- baixa/extinção; NULL quando '-'
    consultado_em     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (client_id, ano)
);

-- Controle do worker (mesmo padrão de mei_refresh/das_refresh): agendamento
-- por cliente + backoff. A DASN muda raríssimo (1x/ano), então a cadência é
-- bem mais longa — definida em `tools::dasn::refresh_dasn_status`.
ALTER TABLE zain.clients
    ADD COLUMN dasn_consultado_em        TIMESTAMPTZ,
    ADD COLUMN dasn_refresh_falhas       INTEGER     NOT NULL DEFAULT 0,
    ADD COLUMN dasn_proxima_tentativa_em TIMESTAMPTZ;
