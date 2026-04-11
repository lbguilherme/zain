-- Cache das consultas SIMEI. Mesma lógica de 0016: a consulta ao
-- Portal do Simples Nacional leva 15-30s e o regime de um CNPJ não
-- muda de um dia pro outro, então um TTL de 48h é suficiente pra
-- acelerar a conversa e ainda pegar mudanças recentes.

CREATE TABLE zain.simei_cache (
    cnpj         TEXT PRIMARY KEY,
    resultado    JSONB NOT NULL,
    consulted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
