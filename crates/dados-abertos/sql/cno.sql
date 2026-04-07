CREATE SCHEMA IF NOT EXISTS cno;

CREATE TABLE cno.obras (
    cno CHAR(12) PRIMARY KEY,
    codigo_pais INT,
    nome_pais TEXT,
    data_inicio DATE,
    data_inicio_responsabilidade DATE,
    data_registro DATE,
    cno_vinculado CHAR(12),
    cep CHAR(8),
    ni_responsavel TEXT,
    qualificacao_responsavel TEXT,
    nome TEXT,
    codigo_municipio INT,
    nome_municipio TEXT,
    tipo_logradouro TEXT,
    logradouro TEXT,
    numero_logradouro TEXT,
    bairro TEXT,
    estado TEXT,
    caixa_postal TEXT,
    complemento TEXT,
    unidade_medida TEXT,
    area_total NUMERIC,
    situacao TEXT,
    data_situacao DATE,
    nome_empresarial TEXT,
    localizacao TEXT
);

CREATE INDEX ON cno.obras (ni_responsavel);
CREATE INDEX ON cno.obras (codigo_municipio);
CREATE INDEX ON cno.obras (situacao);
CREATE INDEX ON cno.obras (cep);

CREATE TABLE cno.cnaes (
    cno CHAR(12) NOT NULL,
    cnae TEXT NOT NULL,
    data_registro DATE
);

CREATE INDEX ON cno.cnaes (cno);
CREATE INDEX ON cno.cnaes (cnae);

CREATE TABLE cno.vinculos (
    cno CHAR(12) NOT NULL,
    data_inicio DATE,
    data_fim DATE,
    data_registro DATE,
    qualificacao_contribuinte TEXT,
    ni_responsavel TEXT
);

CREATE INDEX ON cno.vinculos (cno);
CREATE INDEX ON cno.vinculos (ni_responsavel);

CREATE TABLE cno.areas (
    cno CHAR(12) NOT NULL,
    categoria TEXT,
    destinacao TEXT,
    tipo_obra TEXT,
    tipo_area TEXT,
    tipo_area_complementar TEXT,
    metragem NUMERIC
);

CREATE INDEX ON cno.areas (cno);
