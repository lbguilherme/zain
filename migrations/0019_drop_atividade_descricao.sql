-- Remove a coluna `atividade_descricao`. A descrição da atividade é
-- derivável do código CNAE (via join com `cnae.subclasses`), então
-- não faz sentido guardar os dois — fica ambíguo quando eles se
-- desincronizam. Agora `save_atividade` só registra o CNAE e a
-- montagem do prompt busca a descrição por join em tempo de leitura.

ALTER TABLE zain.clients DROP COLUMN atividade_descricao;
