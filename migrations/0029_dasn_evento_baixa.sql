-- Completa o status por ano da DASN com o segundo atributo de situação
-- especial que o portal expõe no radio do ano: `data-situacao-especial-
-- eventobaixa` (a coluna `situacao_especial` já guarda o `-tipo`).
--
-- São os ÚNICOS dados que o portal público (acesso por CNPJ) traz por ano,
-- além de entregue/não-entregue: o valor declarado, recibo e data de
-- transmissão NÃO ficam disponíveis sem login gov.br (e-CAC). Ambos os
-- campos de situação especial são `-` no caso comum (só preenchem em
-- baixa/extinção do MEI).

ALTER TABLE zain.dasn_anual
    ADD COLUMN situacao_especial_evento TEXT;
