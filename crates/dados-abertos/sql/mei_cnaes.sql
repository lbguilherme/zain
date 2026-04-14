CREATE SCHEMA IF NOT EXISTS mei_cnaes;

CREATE TABLE mei_cnaes.familias (
    codigo CHAR(1) PRIMARY KEY,
    nome TEXT NOT NULL
);

CREATE TABLE mei_cnaes.formas_atuacao (
    codigo INTEGER PRIMARY KEY,
    titulo TEXT NOT NULL,
    descricao TEXT NOT NULL
);

CREATE TABLE mei_cnaes.ocupacoes (
    codigo INTEGER PRIMARY KEY,
    nome TEXT NOT NULL,
    descricao TEXT NOT NULL,
    familia CHAR(1) NOT NULL REFERENCES mei_cnaes.familias(codigo),
    cnae CHAR(7) NOT NULL
);

CREATE INDEX ON mei_cnaes.ocupacoes (cnae);
