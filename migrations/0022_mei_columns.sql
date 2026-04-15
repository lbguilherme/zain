-- Persistência do Certificado de MEI (CCMEI) em `zain.clients`.
-- Preenchida automaticamente após `auth_govbr` concluir com sucesso,
-- quando a consulta pública ao portal do CCMEI retorna um MEI ativo
-- para o CPF autenticado.
--
-- Os dados estruturados do certificado vão num único JSONB mapeado pro
-- struct `rpa::mei::CertificadoMei` (mesmo padrão de `govbr_session`),
-- e o PDF fica numa `bytea` separada — blob binário não casa com JSONB
-- e mandar ele em base64 só pra caber no tipo seria desperdício.

CREATE DOMAIN zain.mei_ccmei AS JSONB;

ALTER TABLE zain.clients
    ADD COLUMN mei_ccmei         zain.mei_ccmei,
    ADD COLUMN mei_ccmei_pdf     BYTEA,
    ADD COLUMN mei_consultado_em TIMESTAMPTZ;
