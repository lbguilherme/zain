CREATE SCHEMA IF NOT EXISTS mei_cnaes;

CREATE TABLE mei_cnaes.ocupacoes (
    codigo INTEGER PRIMARY KEY,
    nome TEXT NOT NULL,
    descricao TEXT NOT NULL,
    familia CHAR(1) NOT NULL,
    cnae CHAR(7) NOT NULL
);

CREATE INDEX ON mei_cnaes.ocupacoes (cnae);
