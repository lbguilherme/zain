CREATE SCHEMA IF NOT EXISTS mei_cnaes;

CREATE TABLE mei_cnaes.ocupacoes (
    ocupacao TEXT NOT NULL,
    cnae_subclasse_id CHAR(7) NOT NULL,
    cnae_descricao TEXT NOT NULL,
    tabela CHAR(1) NOT NULL,
    iss BOOLEAN NOT NULL,
    icms BOOLEAN NOT NULL
);

CREATE INDEX ON mei_cnaes.ocupacoes (cnae_subclasse_id);
