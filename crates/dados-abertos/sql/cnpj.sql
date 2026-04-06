
CREATE SCHEMA IF NOT EXISTS cnpj;

CREATE TABLE cnpj.cnaes (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.motivos (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.municipios (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.naturezas (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.paises (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.qualificacoes (
    codigo INT PRIMARY KEY,
    descricao TEXT NOT NULL
);

CREATE TABLE cnpj.empresas (
    cnpj_basico CHAR(8) PRIMARY KEY,
    razao_social TEXT,
    natureza_juridica INT,
    qualificacao_responsavel INT,
    capital_social NUMERIC,
    porte SMALLINT,
    ente_federativo TEXT
);

CREATE TABLE cnpj.estabelecimentos (
    cnpj_basico CHAR(8) NOT NULL,
    cnpj_ordem CHAR(4) NOT NULL,
    cnpj_dv CHAR(2) NOT NULL,
    identificador_matriz_filial SMALLINT,
    nome_fantasia TEXT,
    situacao_cadastral SMALLINT,
    data_situacao_cadastral DATE,
    motivo_situacao_cadastral INT,
    nome_cidade_exterior TEXT,
    pais INT,
    data_inicio_atividade DATE,
    cnae_fiscal_principal TEXT,
    cnae_fiscal_secundaria TEXT[],
    tipo_logradouro TEXT,
    logradouro TEXT,
    numero TEXT,
    complemento TEXT,
    bairro TEXT,
    cep CHAR(8),
    uf CHAR(2),
    municipio INT,
    ddd1 TEXT,
    telefone1 TEXT,
    ddd2 TEXT,
    telefone2 TEXT,
    ddd_fax TEXT,
    fax TEXT,
    email TEXT,
    situacao_especial TEXT,
    data_situacao_especial DATE,
    PRIMARY KEY (cnpj_basico, cnpj_ordem, cnpj_dv)
);

CREATE INDEX ON cnpj.estabelecimentos (cnpj_basico);
CREATE INDEX ON cnpj.estabelecimentos (situacao_cadastral);
CREATE INDEX ON cnpj.estabelecimentos (cnae_fiscal_principal);
CREATE INDEX ON cnpj.estabelecimentos (municipio);
CREATE INDEX ON cnpj.estabelecimentos (uf);

CREATE TABLE cnpj.simples (
    cnpj_basico CHAR(8) PRIMARY KEY,
    opcao_simples BOOLEAN,
    data_opcao_simples DATE,
    data_exclusao_simples DATE,
    opcao_mei BOOLEAN,
    data_opcao_mei DATE,
    data_exclusao_mei DATE
);

CREATE INDEX ON cnpj.simples (opcao_mei);

CREATE TABLE cnpj.socios (
    cnpj_basico CHAR(8) NOT NULL,
    identificador_socio SMALLINT,
    nome TEXT,
    cnpj_cpf_socio TEXT,
    qualificacao INT,
    data_entrada DATE,
    pais INT,
    representante_legal TEXT,
    nome_representante TEXT,
    qualificacao_representante INT,
    faixa_etaria SMALLINT
);

CREATE INDEX ON cnpj.socios (cnpj_basico);
