-- Inverte a semântica de `tem_mei`: antes era "a pessoa já possui MEI?";
-- agora é "a pessoa tem intenção de abrir um MEI novo?". O valor se
-- deriva naturalmente — quem já tem CNPJ MEI não precisa abrir outro,
-- então `save_cnpj` também passa a zerar essa flag quando confirma um
-- MEI ativo. Armazenar como intent simplifica a decisão de fluxo no
-- prompt (intent de abertura → coleta os dados do cadastro e segue).

ALTER TABLE zain.clients RENAME COLUMN tem_mei TO quer_abrir_mei;

UPDATE zain.clients
SET quer_abrir_mei = NOT quer_abrir_mei
WHERE quer_abrir_mei IS NOT NULL;
