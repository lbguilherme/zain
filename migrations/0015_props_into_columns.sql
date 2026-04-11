-- Move os campos dinâmicos que viviam em `zain.clients.props` (JSONB)
-- para colunas dedicadas. A mesma lógica de 0014 (govbr_*): acabou que
-- os campos são poucos e fixos, então virou JSONB-freestyle sem ganho
-- nenhum sobre colunas tipadas — e colunas tipadas casam melhor com as
-- queries, índices e com o pattern de UPDATE direto que as tools já
-- usam pro gov.br.

ALTER TABLE zain.clients
    ADD COLUMN cpf                     TEXT,
    ADD COLUMN cnpj                    TEXT,
    ADD COLUMN tem_mei                 BOOLEAN,
    ADD COLUMN atividade_descricao     TEXT,
    ADD COLUMN cnae                    TEXT,
    ADD COLUMN endereco                TEXT,
    ADD COLUMN pagamento_solicitado_em TIMESTAMPTZ,
    ADD COLUMN recusa_motivo           TEXT,
    ADD COLUMN recusado_em             TIMESTAMPTZ;

UPDATE zain.clients SET
    cpf                     = props->>'cpf',
    cnpj                    = props->>'cnpj',
    tem_mei                 = (props->>'tem_mei')::boolean,
    atividade_descricao     = props->>'atividade_descricao',
    cnae                    = props->>'cnae',
    endereco                = props->>'endereco',
    pagamento_solicitado_em = (props->>'pagamento_solicitado_em')::timestamptz,
    recusa_motivo           = props->'recusado'->>'motivo',
    recusado_em             = (props->'recusado'->>'em')::timestamptz
WHERE props IS NOT NULL AND props <> '{}'::jsonb;

ALTER TABLE zain.clients DROP COLUMN props;
