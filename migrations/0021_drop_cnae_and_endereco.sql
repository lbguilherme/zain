-- Remove as colunas `cnae` e `endereco` de `zain.clients`. Esses dados
-- deixaram de ser persistidos em colunas dedicadas: a tool `abrir_empresa`
-- recebe tudo isso como argumento direto na hora da inscrição, e o que
-- precisa sobreviver entre turnos antes disso fica em `anotar`/`memory`.

ALTER TABLE zain.clients
    DROP COLUMN cnae,
    DROP COLUMN endereco;
