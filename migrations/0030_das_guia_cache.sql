-- Cache da guia DAS emitida, por (cliente, competência), válido no MESMO
-- DIA. O PGMEI tem limite diário de geração de DAS por CNPJ (erro 23998 -
-- "limite diário excedido"); como o valor de uma competência NÃO muda
-- dentro do mesmo dia (multa/juros são calculados pra a data de geração),
-- re-emitir a mesma guia no mesmo dia produz um PDF idêntico e gasta uma
-- geração do limite à toa. Então: emitiu hoje → serve o PDF cacheado;
-- dia novo → emite de novo (valor mudou) e sobrescreve.
--
-- "Mesmo dia" é avaliado no fuso America/Sao_Paulo (sem horário de verão
-- desde 2019), que é o fuso do portal/limite.

CREATE TABLE zain.das_guia_cache (
    client_id       UUID        NOT NULL REFERENCES zain.clients(id) ON DELETE CASCADE,
    periodo         TEXT        NOT NULL,  -- competência 'YYYYMM'
    gerado_em       TIMESTAMPTZ NOT NULL DEFAULT now(),
    competencia     TEXT        NOT NULL,  -- rótulo humano, ex: 'Abril/2026'
    numero_das      TEXT        NOT NULL,
    total_cents     BIGINT,
    vencimento      DATE,
    pagar_ate       DATE,
    linha_digitavel TEXT,
    pdf             BYTEA       NOT NULL,
    PRIMARY KEY (client_id, periodo)
);
