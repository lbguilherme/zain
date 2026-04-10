CREATE EXTENSION IF NOT EXISTS vector;

CREATE SCHEMA IF NOT EXISTS cnae;

CREATE TABLE cnae.secoes (
    id CHAR(1) PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnae.divisoes (
    id CHAR(2) PRIMARY KEY,
    descricao TEXT NOT NULL,
    secao_id CHAR(1) NOT NULL
);

CREATE INDEX ON cnae.divisoes (secao_id);

CREATE TABLE cnae.grupos (
    id CHAR(3) PRIMARY KEY,
    descricao TEXT NOT NULL,
    divisao_id CHAR(2) NOT NULL
);

CREATE INDEX ON cnae.grupos (divisao_id);

CREATE TABLE cnae.classes (
    id CHAR(5) PRIMARY KEY,
    descricao TEXT NOT NULL,
    grupo_id CHAR(3) NOT NULL,
    observacoes TEXT
);

CREATE INDEX ON cnae.classes (grupo_id);

CREATE TABLE cnae.subclasses (
    id CHAR(7) PRIMARY KEY,
    descricao TEXT NOT NULL,
    classe_id CHAR(5) NOT NULL,
    observacoes TEXT,
    embedding halfvec NOT NULL
);

CREATE INDEX ON cnae.subclasses (classe_id);

CREATE TABLE cnae.subclasse_atividades (
    subclasse_id CHAR(7) NOT NULL,
    atividade TEXT NOT NULL,
    embedding halfvec NOT NULL
);

CREATE INDEX ON cnae.subclasse_atividades (subclasse_id);
