use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// https://arquivos.receitafederal.gov.br/public.php/dav/files/gn672Ad4CF8N6TK/Dados/Cadastros/CNO/

const DATA_VERSION: &str = "2026-04-06";
const EXTRACTOR_VERSION: u32 = 1;
const ZIP_URL: &str = "https://arquivos.receitafederal.gov.br/public.php/dav/files/gn672Ad4CF8N6TK/Dados/Cadastros/CNO/cno.zip";
const ZIP_FILENAME: &str = "cno.zip";

fn parse_date(val: &str) -> Result<String, &'static str> {
    if val == "00000000" || val == "0" {
        return Ok("\\N".to_string());
    }
    if val.len() == 10 && val.as_bytes()[4] == b'-' {
        Ok(val.to_string())
    } else if val.len() == 8 {
        Ok(format!("{}-{}-{}", &val[0..4], &val[4..6], &val[6..8]))
    } else {
        Err("formato de data inválido")
    }
}

pub struct CnoSource;

impl DataSource for CnoSource {
    fn schema_name(&self) -> &str {
        "cno"
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
        vec![Download {
            url: ZIP_URL.to_string(),
            filename: ZIP_FILENAME.to_string(),
        }]
    }
}

// CNO.CSV
static COLUMNS_CNO: &[Column] = &[
    Column::text("cno", "CHAR(12) PRIMARY KEY"),
    Column::int("codigo_pais", "INT"),
    Column::text("nome_pais", "TEXT"),
    Column::date("data_inicio", parse_date),
    Column::date("data_inicio_responsabilidade", parse_date),
    Column::date("data_registro", parse_date),
    Column::text("cno_vinculado", "CHAR(12)"),
    Column::text("cep", "CHAR(8)"),
    Column::text("ni_responsavel", "TEXT"),
    Column::int("qualificacao_responsavel", "SMALLINT"),
    Column::text("nome", "TEXT"),
    Column::int("codigo_municipio", "INT"),
    Column::text("nome_municipio", "TEXT"),
    Column::text("tipo_logradouro", "TEXT"),
    Column::text("logradouro", "TEXT"),
    Column::text("numero_logradouro", "TEXT"),
    Column::text("bairro", "TEXT"),
    Column::text("estado", "TEXT"),
    Column::text("caixa_postal", "TEXT"),
    Column::text("complemento", "TEXT"),
    Column::text("unidade_medida", "TEXT"),
    Column::decimal("area_total", "NUMERIC"),
    Column::int("situacao", "SMALLINT"),
    Column::date("data_situacao", parse_date),
    Column::text("nome_empresarial", "TEXT"),
    Column::text("localizacao", "TEXT"),
];

// CNO_CNAES.CSV
static COLUMNS_CNAES: &[Column] = &[
    Column::text("cno", "CHAR(12) NOT NULL"),
    Column::text("cnae", "TEXT NOT NULL"),
    Column::date("data_registro", parse_date),
];

// CNO_VINCULOS.CSV
static COLUMNS_VINCULOS: &[Column] = &[
    Column::text("cno", "CHAR(12) NOT NULL"),
    Column::date("data_inicio", parse_date),
    Column::date("data_fim", parse_date),
    Column::date("data_registro", parse_date),
    Column::int("qualificacao_contribuinte", "SMALLINT"),
    Column::text("ni_responsavel", "TEXT"),
];

// CNO_AREAS.CSV
static COLUMNS_AREAS: &[Column] = &[
    Column::text("cno", "CHAR(12) NOT NULL"),
    Column::text("categoria", "TEXT"),
    Column::text("destinacao", "TEXT"),
    Column::text("tipo_obra", "TEXT"),
    Column::text("tipo_area", "TEXT"),
    Column::text("tipo_area_complementar", "TEXT"),
    Column::decimal("metragem", "NUMERIC"),
];

static TABLES: &[Table] = &[
    Table {
        name: "obras",
        file_prefix: "cno",
        file_count: 1,
        columns: COLUMNS_CNO,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"obras\" (\"ni_responsavel\")",
            "CREATE INDEX ON \"{schema}\".\"obras\" (\"codigo_municipio\")",
            "CREATE INDEX ON \"{schema}\".\"obras\" (\"situacao\")",
            "CREATE INDEX ON \"{schema}\".\"obras\" (\"cep\")",
        ],
        has_headers: true,
        delimiter: b',',
        csv_filename: Some("CNO.CSV"),
    },
    Table {
        name: "cnaes",
        file_prefix: "cno",
        file_count: 1,
        columns: COLUMNS_CNAES,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"cnaes\" (\"cno\")",
            "CREATE INDEX ON \"{schema}\".\"cnaes\" (\"cnae\")",
        ],
        has_headers: true,
        delimiter: b',',
        csv_filename: Some("CNO_CNAES.CSV"),
    },
    Table {
        name: "vinculos",
        file_prefix: "cno",
        file_count: 1,
        columns: COLUMNS_VINCULOS,
        extra_ddl: &[
            "CREATE INDEX ON \"{schema}\".\"vinculos\" (\"cno\")",
            "CREATE INDEX ON \"{schema}\".\"vinculos\" (\"ni_responsavel\")",
        ],
        has_headers: true,
        delimiter: b',',
        csv_filename: Some("CNO_VINCULOS.CSV"),
    },
    Table {
        name: "areas",
        file_prefix: "cno",
        file_count: 1,
        columns: COLUMNS_AREAS,
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"areas\" (\"cno\")"],
        has_headers: true,
        delimiter: b',',
        csv_filename: Some("CNO_AREAS.CSV"),
    },
];
