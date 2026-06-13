-- Situação mensal do DAS (a "mensalidade" do MEI), consolidada do PGMEI
-- pelo worker `jobs::das_refresh` pra que o `get_client_state` reporte
-- atraso/próximo vencimento como leitura SQL pura (sem RPA no caminho).
--
-- Fonte: tabela de períodos da tela "Emitir Guia de Pagamento (DAS)" do
-- PGMEI (acesso público por CNPJ — não requer login gov.br). Uma linha
-- por (cliente, competência YYYYMM), upsert a cada consulta.
--
-- O PDF da guia NÃO é persistido: guia de mês em atraso é recalculada
-- por dia (multa/juros) e o "pagar até" da emissão é o próprio dia —
-- a tool `emitir_das` sempre emite na hora.

CREATE TABLE zain.das_mensal (
    client_id       UUID        NOT NULL REFERENCES zain.clients(id) ON DELETE CASCADE,
    periodo         TEXT        NOT NULL,  -- competência 'YYYYMM' (value do checkbox do PGMEI)
    competencia     TEXT        NOT NULL,  -- rótulo humano, ex: 'Abril/2026'
    apurado         BOOLEAN     NOT NULL,
    situacao        TEXT        NOT NULL,  -- liquidado | devedor | a_vencer | nao_optante | outra
    situacao_texto  TEXT        NOT NULL,  -- texto cru da célula (ex: 'Liquidado em 10/05/2026')
    principal_cents BIGINT,
    multa_cents     BIGINT,
    juros_cents     BIGINT,
    total_cents     BIGINT,
    vencimento      DATE,
    acolhimento     DATE,
    consultado_em   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (client_id, periodo)
);

-- Controle do worker (mesmo padrão do mei_refresh, migrations 0025/0026):
-- TTL via das_consultado_em + backoff exponencial em falha transitória.
ALTER TABLE zain.clients
    ADD COLUMN das_consultado_em        TIMESTAMPTZ,
    ADD COLUMN das_refresh_falhas       INTEGER     NOT NULL DEFAULT 0,
    ADD COLUMN das_proxima_tentativa_em TIMESTAMPTZ;
