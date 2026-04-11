-- Passa a chavear `zain.govbr` por `client_id` (FK para `zain.clients`)
-- em vez de `cpf`. `cpf` vira coluna regular — múltiplos clientes podem
-- compartilhar o mesmo cpf.
--
-- A migração é feita em etapas incrementais para preservar dados
-- existentes: adiciona a coluna como nullable, obriga o operador a
-- backfillar antes de promovê-la a NOT NULL + PK. Se a tabela estiver
-- vazia, todas as etapas passam sem intervenção.

ALTER TABLE zain.govbr DROP CONSTRAINT govbr_pkey;

ALTER TABLE zain.govbr
    ADD COLUMN client_id UUID REFERENCES zain.clients(id);

-- Qualquer row que sobrou sem client_id bloqueia o NOT NULL abaixo;
-- o operador deve backfillar antes de rodar esta migração.
ALTER TABLE zain.govbr ALTER COLUMN client_id SET NOT NULL;

ALTER TABLE zain.govbr ADD PRIMARY KEY (client_id);
