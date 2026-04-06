CREATE SCHEMA IF NOT EXISTS pgfn;

CREATE TYPE pgfn.tipo_pessoa AS ENUM ('pf', 'pj');
CREATE TYPE pgfn.tipo_devedor AS ENUM ('principal', 'corresponsavel', 'solidario');

CREATE TABLE pgfn.divida_ativa_geral (
    cpf_cnpj TEXT NOT NULL,
    tipo_pessoa pgfn.tipo_pessoa,
    tipo_devedor pgfn.tipo_devedor,
    nome_devedor TEXT,
    uf_devedor CHAR(2),
    unidade_responsavel TEXT,
    numero_inscricao TEXT NOT NULL,
    tipo_situacao_inscricao TEXT,
    situacao_inscricao TEXT,
    receita_principal TEXT,
    data_inscricao DATE,
    indicador_ajuizado BOOLEAN,
    valor_consolidado NUMERIC
);

CREATE INDEX ON pgfn.divida_ativa_geral (numero_inscricao);
CREATE INDEX ON pgfn.divida_ativa_geral (cpf_cnpj);
CREATE INDEX ON pgfn.divida_ativa_geral (uf_devedor);

CREATE TABLE pgfn.divida_previdenciaria (
    cpf_cnpj TEXT NOT NULL,
    tipo_pessoa pgfn.tipo_pessoa,
    tipo_devedor pgfn.tipo_devedor,
    nome_devedor TEXT,
    uf_devedor CHAR(2),
    unidade_responsavel TEXT,
    numero_inscricao TEXT NOT NULL,
    tipo_situacao_inscricao TEXT,
    situacao_inscricao TEXT,
    tipo_credito TEXT,
    data_inscricao DATE,
    indicador_ajuizado BOOLEAN,
    valor_consolidado NUMERIC
);

CREATE INDEX ON pgfn.divida_previdenciaria (numero_inscricao);
CREATE INDEX ON pgfn.divida_previdenciaria (cpf_cnpj);
CREATE INDEX ON pgfn.divida_previdenciaria (uf_devedor);

CREATE TABLE pgfn.divida_fgts (
    cpf_cnpj TEXT NOT NULL,
    tipo_pessoa pgfn.tipo_pessoa,
    tipo_devedor pgfn.tipo_devedor,
    nome_devedor TEXT,
    uf_devedor CHAR(2),
    unidade_responsavel TEXT,
    entidade_responsavel TEXT,
    unidade_inscricao TEXT,
    numero_inscricao TEXT NOT NULL,
    tipo_situacao_inscricao TEXT,
    situacao_inscricao TEXT,
    receita_principal TEXT,
    data_inscricao DATE,
    indicador_ajuizado BOOLEAN,
    valor_consolidado NUMERIC
);

CREATE INDEX ON pgfn.divida_fgts (numero_inscricao);
CREATE INDEX ON pgfn.divida_fgts (cpf_cnpj);
CREATE INDEX ON pgfn.divida_fgts (uf_devedor);
