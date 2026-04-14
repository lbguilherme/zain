use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// http://arquivos.receitafederal.gov.br/index.php/s/gn672Ad4CF8N6TK?dir=/Dados/Cadastros/CNPJ

const DATA_VERSION: &str = "2026-04";
const EXTRACTOR_VERSION: u32 = 3;
const BASE_URL: &str = "https://arquivos.receitafederal.gov.br/public.php/dav/files/gn672Ad4CF8N6TK/Dados/Cadastros/CNPJ/2026-04/";

fn parse_date_ymd(val: &str) -> Result<String, &'static str> {
    if val == "00000000" || val == "0" {
        return Ok("\\N".to_string());
    }
    if val.len() == 8 {
        Ok(format!("{}-{}-{}", &val[0..4], &val[4..6], &val[6..8]))
    } else {
        Err("formato de data YYYYMMDD inválido")
    }
}

fn parse_text_array(val: &str) -> Result<String, &'static str> {
    let items: Vec<&str> = val
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return Ok("\\N".to_string());
    }
    Ok(format!("{{{}}}", items.join(",")))
}

fn parse_bool_sn(val: &str) -> Result<String, &'static str> {
    match val {
        "S" | "s" => Ok("t".to_string()),
        "N" | "n" => Ok("f".to_string()),
        _ => Err("valor booleano (S/N) inesperado"),
    }
}

pub struct CnpjSource;

impl DataSource for CnpjSource {
    fn schema_name(&self) -> &str {
        "cnpj"
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

static TABLES: &[Table] = &[
    // Tabelas de dominio
    Table {
        name: "cnaes",
        file_prefix: "Cnaes",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "motivos",
        file_prefix: "Motivos",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "municipios",
        file_prefix: "Municipios",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "naturezas",
        file_prefix: "Naturezas",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "paises",
        file_prefix: "Paises",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "qualificacoes",
        file_prefix: "Qualificacoes",
        file_count: 1,
        columns: &[
            Column::int("codigo", "INT PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    // Tabelas principais
    Table {
        name: "empresas",
        file_prefix: "Empresas",
        file_count: 10,
        columns: &[
            Column::text("cnpj_basico", "CHAR(8) PRIMARY KEY"),
            Column::text("razao_social", "TEXT"),
            Column::int("natureza_juridica", "INT"),
            Column::int("qualificacao_responsavel", "INT"),
            Column::decimal("capital_social", "NUMERIC"),
            Column::int("porte", "SMALLINT"),
            Column::text("ente_federativo", "TEXT"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "estabelecimentos",
        file_prefix: "Estabelecimentos",
        file_count: 10,
        columns: &[
            Column::text("cnpj_basico", "CHAR(8) NOT NULL"),
            Column::text("cnpj_ordem", "CHAR(4) NOT NULL"),
            Column::text("cnpj_dv", "CHAR(2) NOT NULL"),
            Column::int("identificador_matriz_filial", "SMALLINT"),
            Column::text("nome_fantasia", "TEXT"),
            Column::int("situacao_cadastral", "SMALLINT"),
            Column::date("data_situacao_cadastral", parse_date_ymd),
            Column::int("motivo_situacao_cadastral", "INT"),
            Column::text("nome_cidade_exterior", "TEXT"),
            Column::int("pais", "INT"),
            Column::date("data_inicio_atividade", parse_date_ymd),
            Column::text("cnae_fiscal_principal", "TEXT"),
            Column::custom("cnae_fiscal_secundaria", "TEXT[]", parse_text_array),
            Column::text("tipo_logradouro", "TEXT"),
            Column::text("logradouro", "TEXT"),
            Column::text("numero", "TEXT"),
            Column::text("complemento", "TEXT"),
            Column::text("bairro", "TEXT"),
            Column::text("cep", "CHAR(8)"),
            Column::text("uf", "CHAR(2)"),
            Column::int("municipio", "INT"),
            Column::text("ddd1", "TEXT"),
            Column::text("telefone1", "TEXT"),
            Column::text("ddd2", "TEXT"),
            Column::text("telefone2", "TEXT"),
            Column::text("ddd_fax", "TEXT"),
            Column::text("fax", "TEXT"),
            Column::text("email", "TEXT"),
            Column::text("situacao_especial", "TEXT"),
            Column::date("data_situacao_especial", parse_date_ymd),
        ],
        extra_ddl: &[
            "ALTER TABLE \"{schema}\".\"estabelecimentos\" ADD PRIMARY KEY (\"cnpj_basico\", \"cnpj_ordem\", \"cnpj_dv\")",
            "CREATE INDEX ON \"{schema}\".\"estabelecimentos\" (\"cnpj_basico\")",
            "CREATE INDEX ON \"{schema}\".\"estabelecimentos\" (\"situacao_cadastral\")",
            "CREATE INDEX ON \"{schema}\".\"estabelecimentos\" (\"cnae_fiscal_principal\")",
            "CREATE INDEX ON \"{schema}\".\"estabelecimentos\" (\"municipio\")",
            "CREATE INDEX ON \"{schema}\".\"estabelecimentos\" (\"uf\")",
        ],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "simples",
        file_prefix: "Simples",
        file_count: 1,
        columns: &[
            Column::text("cnpj_basico", "CHAR(8) PRIMARY KEY"),
            Column::bool("opcao_simples", parse_bool_sn),
            Column::date("data_opcao_simples", parse_date_ymd),
            Column::date("data_exclusao_simples", parse_date_ymd),
            Column::bool("opcao_mei", parse_bool_sn),
            Column::date("data_opcao_mei", parse_date_ymd),
            Column::date("data_exclusao_mei", parse_date_ymd),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"simples\" (\"opcao_mei\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "socios",
        file_prefix: "Socios",
        file_count: 10,
        columns: &[
            Column::text("cnpj_basico", "CHAR(8) NOT NULL"),
            Column::int("identificador_socio", "SMALLINT"),
            Column::text("nome", "TEXT"),
            Column::text("cnpj_cpf_socio", "TEXT"),
            Column::int("qualificacao", "INT"),
            Column::date("data_entrada", parse_date_ymd),
            Column::int("pais", "INT"),
            Column::text("representante_legal", "TEXT"),
            Column::text("nome_representante", "TEXT"),
            Column::int("qualificacao_representante", "INT"),
            Column::int("faixa_etaria", "SMALLINT"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"socios\" (\"cnpj_basico\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
];
