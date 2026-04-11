-- Cache das consultas PGFN. A consulta real é via scraping e leva
-- 15-30s, então cacheamos o resultado por 48h — tempo suficiente pra
-- uma conversa completa de lead sem re-consultar e curto o bastante
-- pra pegar mudanças de status recentes (pagamento, parcelamento).

CREATE TABLE zain.pgfn_cache (
    documento    TEXT PRIMARY KEY,
    resultado    JSONB NOT NULL,
    consulted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
