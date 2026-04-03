use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// https://www.gov.br/pgfn/pt-br/assuntos/divida-ativa-da-uniao/transparencia-fiscal-1/dados-abertos

const DATA_VERSION: &str = "2025_trimestre_04";
const EXTRACTOR_VERSION: u32 = 2;
const BASE_URL: &str = "https://dadosabertos.pgfn.gov.br/2025_trimestre_04";

fn parse_date_dmy(val: &str) -> Result<String, &'static str> {
    if val.len() == 10 && val.as_bytes()[2] == b'/' {
        Ok(format!("{}-{}-{}", &val[6..10], &val[3..5], &val[0..2]))
    } else {
        Err("formato de data DD/MM/YYYY inválido")
    }
}

fn parse_bool_sim_nao(val: &str) -> Result<String, &'static str> {
    match val.to_ascii_uppercase().as_str() {
        "SIM" => Ok("t".to_string()),
        "NAO" | "NÃO" => Ok("f".to_string()),
        _ => Err("valor booleano (SIM/NÃO) inesperado"),
    }
}

fn strip_cpf_cnpj(val: &str) -> Result<String, &'static str> {
    Ok(val.replace(['.', '/', '-'], ""))
}

fn normalize_tipo_devedor(val: &str) -> Result<String, &'static str> {
    match val.to_uppercase().as_str() {
        "PRINCIPAL" => Ok("principal".to_string()),
        "CORRESPONSAVEL" | "CORRESPONSÁVEL" => Ok("corresponsavel".to_string()),
        "SOLIDARIO" | "SOLIDÁRIO" => Ok("solidario".to_string()),
        _ => Err("valor inesperado para tipo_devedor"),
    }
}

fn normalize_tipo_pessoa(val: &str) -> Result<String, &'static str> {
    match val {
        "Pessoa física" | "Pessoa fisica" => Ok("pf".to_string()),
        "Pessoa jurídica" | "Pessoa juridica" => Ok("pj".to_string()),
        _ => Err("valor inesperado para tipo_pessoa"),
    }
}

pub struct PgfnSource;

impl DataSource for PgfnSource {
    fn schema_name(&self) -> &str {
        "pgfn"
    }

    fn data_version(&self) -> &str {
        DATA_VERSION
    }

    fn extractor_version(&self) -> u32 {
        EXTRACTOR_VERSION
    }

    fn tables(&self) -> &'static [Table] {
        TABLES
    }

    fn setup_ddl(&self) -> &'static [&'static str] {
        &[
            "CREATE TYPE \"{schema}\".tipo_pessoa AS ENUM ('pf', 'pj')",
            "CREATE TYPE \"{schema}\".tipo_devedor AS ENUM ('principal', 'corresponsavel', 'solidario')",
        ]
    }

    fn downloads(&self) -> Vec<Download> {
        TABLES
            .iter()
            .flat_map(|t| t.zip_filenames())
            .map(|filename| Download {
                url: format!("{BASE_URL}/{filename}"),
                filename,
            })
            .collect()
    }
}

// CPF_CNPJ;TIPO_PESSOA;TIPO_DEVEDOR;NOME_DEVEDOR;UF_DEVEDOR;UNIDADE_RESPONSAVEL;
// NUMERO_INSCRICAO;TIPO_SITUACAO_INSCRICAO;SITUACAO_INSCRICAO;RECEITA_PRINCIPAL;
// DATA_INSCRICAO;INDICADOR_AJUIZADO;VALOR_CONSOLIDADO
static COLUMNS_NAO_PREV: &[Column] = &[
    Column::custom("cpf_cnpj", "TEXT NOT NULL", strip_cpf_cnpj),
    Column::custom(
        "tipo_pessoa",
        "\"{schema}\".tipo_pessoa",
        normalize_tipo_pessoa,
    ),
    Column::custom(
        "tipo_devedor",
        "\"{schema}\".tipo_devedor",
        normalize_tipo_devedor,
    ),
    Column::text("nome_devedor", "TEXT"),
    Column::text("uf_devedor", "CHAR(2)"),
    Column::text("unidade_responsavel", "TEXT"),
    Column::text("numero_inscricao", "TEXT NOT NULL"),
    Column::text("tipo_situacao_inscricao", "TEXT"),
    Column::text("situacao_inscricao", "TEXT"),
    Column::text("receita_principal", "TEXT"),
    Column::date("data_inscricao", parse_date_dmy),
    Column::bool("indicador_ajuizado", parse_bool_sim_nao),
    Column::decimal("valor_consolidado", "NUMERIC"),
];

// CPF_CNPJ;TIPO_PESSOA;TIPO_DEVEDOR;NOME_DEVEDOR;UF_DEVEDOR;UNIDADE_RESPONSAVEL;
// NUMERO_INSCRICAO;TIPO_SITUACAO_INSCRICAO;SITUACAO_INSCRICAO;TIPO_CREDITO;
// DATA_INSCRICAO;INDICADOR_AJUIZADO;VALOR_CONSOLIDADO
static COLUMNS_PREV: &[Column] = &[
    Column::custom("cpf_cnpj", "TEXT NOT NULL", strip_cpf_cnpj),
    Column::custom(
        "tipo_pessoa",
        "\"{schema}\".tipo_pessoa",
        normalize_tipo_pessoa,
    ),
    Column::custom(
        "tipo_devedor",
        "\"{schema}\".tipo_devedor",
        normalize_tipo_devedor,
    ),
    Column::text("nome_devedor", "TEXT"),
    Column::text("uf_devedor", "CHAR(2)"),
    Column::text("unidade_responsavel", "TEXT"),
    Column::text("numero_inscricao", "TEXT NOT NULL"),
    Column::text("tipo_situacao_inscricao", "TEXT"),
    Column::text("situacao_inscricao", "TEXT"),
    Column::text("tipo_credito", "TEXT"),
    Column::date("data_inscricao", parse_date_dmy),
    Column::bool("indicador_ajuizado", parse_bool_sim_nao),
    Column::decimal("valor_consolidado", "NUMERIC"),
];

// CPF_CNPJ;TIPO_PESSOA;TIPO_DEVEDOR;NOME_DEVEDOR;UF_DEVEDOR;UNIDADE_RESPONSAVEL;
// ENTIDADE_RESPONSAVEL;UNIDADE_INSCRICAO;NUMERO_INSCRICAO;TIPO_SITUACAO_INSCRICAO;
// SITUACAO_INSCRICAO;RECEITA_PRINCIPAL;DATA_INSCRICAO;INDICADOR_AJUIZADO;VALOR_CONSOLIDADO
static COLUMNS_FGTS: &[Column] = &[
    Column::custom("cpf_cnpj", "TEXT NOT NULL", strip_cpf_cnpj),
    Column::custom(
        "tipo_pessoa",
        "\"{schema}\".tipo_pessoa",
        normalize_tipo_pessoa,
    ),
    Column::custom(
        "tipo_devedor",
        "\"{schema}\".tipo_devedor",
        normalize_tipo_devedor,
    ),
    Column::text("nome_devedor", "TEXT"),
    Column::text("uf_devedor", "CHAR(2)"),
    Column::text("unidade_responsavel", "TEXT"),
    Column::text("entidade_responsavel", "TEXT"),
    Column::text("unidade_inscricao", "TEXT"),
    Column::text("numero_inscricao", "TEXT NOT NULL"),
    Column::text("tipo_situacao_inscricao", "TEXT"),
    Column::text("situacao_inscricao", "TEXT"),
    Column::text("receita_principal", "TEXT"),
    Column::date("data_inscricao", parse_date_dmy),
    Column::bool("indicador_ajuizado", parse_bool_sim_nao),
    Column::decimal("valor_consolidado", "NUMERIC"),
];

static TABLES: &[Table] = &[
    Table {
        name: "divida_ativa_geral",
        file_prefix: "Dados_abertos_Nao_Previdenciario",
        file_count: 1,
        columns: COLUMNS_NAO_PREV,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"divida_ativa_geral\" (\"numero_inscricao\")",
            "CREATE INDEX ON \"{schema}\".\"divida_ativa_geral\" (\"cpf_cnpj\")",
            "CREATE INDEX ON \"{schema}\".\"divida_ativa_geral\" (\"uf_devedor\")",
        ],
        has_headers: true,
    },
    Table {
        name: "divida_previdenciaria",
        file_prefix: "Dados_abertos_Previdenciario",
        file_count: 1,
        columns: COLUMNS_PREV,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"divida_previdenciaria\" (\"numero_inscricao\")",
            "CREATE INDEX ON \"{schema}\".\"divida_previdenciaria\" (\"cpf_cnpj\")",
            "CREATE INDEX ON \"{schema}\".\"divida_previdenciaria\" (\"uf_devedor\")",
        ],
        has_headers: true,
    },
    Table {
        name: "divida_fgts",
        file_prefix: "Dados_abertos_FGTS",
        file_count: 1,
        columns: COLUMNS_FGTS,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"divida_fgts\" (\"numero_inscricao\")",
            "CREATE INDEX ON \"{schema}\".\"divida_fgts\" (\"cpf_cnpj\")",
            "CREATE INDEX ON \"{schema}\".\"divida_fgts\" (\"uf_devedor\")",
        ],
        has_headers: true,
    },
];
