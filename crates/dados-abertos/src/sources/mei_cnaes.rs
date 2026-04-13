use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio_postgres::Transaction;

use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// Ocupações permitidas ao MEI, obtidas do Portal do Empreendedor.
// https://www.gov.br/empresas-e-negocios/pt-br/empreendedor

const DATA_VERSION: &str = "2026-04";
const EXTRACTOR_VERSION: u32 = 4;

pub struct MeiCnaesSource;

impl DataSource for MeiCnaesSource {
    fn schema_name(&self) -> &str {
        "mei_cnaes"
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
        vec![]
    }

    fn import_data<'a>(
        &self,
        tx: &'a Transaction<'a>,
        temp_schema: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        let data_dir = self.data_dir();
        Box::pin(async move { import_mei_cnaes(tx, temp_schema, &data_dir).await })
    }
}

#[derive(Deserialize)]
struct OcupacaoJson {
    codigo: String,
    nome: String,
    #[serde(rename = "descricaoObjeto")]
    descricao_objeto: String,
    cnae: CnaeJson,
    familia: String,
}

#[derive(Deserialize)]
struct CnaeJson {
    codigo: String,
}

async fn import_mei_cnaes(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    let path = data_dir.join("ocupacao.json");
    let text = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("falha ao ler {}", path.display()))?;

    let ocupacoes: Vec<OcupacaoJson> = serde_json::from_str(&text)
        .with_context(|| format!("falha ao parsear {}", path.display()))?;

    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"ocupacoes\" (codigo, nome, descricao, familia, cnae) \
             VALUES ($1, $2, $3, $4, $5)"
        ))
        .await?;

    let mut count: u64 = 0;
    for o in &ocupacoes {
        let codigo: i32 = o
            .codigo
            .parse()
            .with_context(|| format!("codigo inválido: {}", o.codigo))?;
        tx.execute(
            &stmt,
            &[
                &codigo,
                &o.nome,
                &o.descricao_objeto,
                &o.familia,
                &o.cnae.codigo,
            ],
        )
        .await?;
        count += 1;
    }

    println!("  ocupacoes: {count} registros");
    Ok(())
}

static TABLES: &[Table] = &[Table {
    name: "ocupacoes",
    file_prefix: "",
    file_count: 0,
    columns: &[
        Column::int("codigo", "INTEGER PRIMARY KEY"),
        Column::text("nome", "TEXT NOT NULL"),
        Column::text("descricao", "TEXT NOT NULL"),
        Column::text("familia", "CHAR(1) NOT NULL"),
        Column::text("cnae", "CHAR(7) NOT NULL"),
    ],
    extra_ddl: &["CREATE INDEX ON \"{schema}\".\"ocupacoes\" (\"cnae\")"],
    has_headers: false,
    delimiter: b';',
    csv_filename: None,
}];
