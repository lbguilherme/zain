-- Simplifica a máquina de estados: "lead" deixa de ser um estado
-- específico e passa a ser o core da lógica do agent. Em vez de
-- estados distintos (LEAD, RECUSADO, COBRANCA, ...), o cliente tem
-- um único conjunto de propriedades que acumula flags (recusado,
-- pagamento_solicitado, cliente_ativo, govbr_vinculado, ...).

ALTER TABLE zain.clients DROP COLUMN state;
ALTER TABLE zain.clients RENAME COLUMN state_props TO props;

ALTER TABLE zain.executions DROP COLUMN state_before;
ALTER TABLE zain.executions DROP COLUMN state_after;
