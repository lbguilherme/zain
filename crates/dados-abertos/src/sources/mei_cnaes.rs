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
const EXTRACTOR_VERSION: u32 = 5;

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
        Box::pin(async move {
            import_familias(tx, temp_schema, &data_dir).await?;
            import_formas_atuacao(tx, temp_schema, &data_dir).await?;
            import_ocupacoes(tx, temp_schema, &data_dir).await?;
            Ok(())
        })
    }
}

#[derive(Deserialize)]
struct FamiliaJson {
    codigo: String,
    nome: String,
}

#[derive(Deserialize)]
struct FormaAtuacaoJson {
    codigo: String,
    titulo: String,
    descricao: String,
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

async fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> Result<Vec<T>> {
    let text = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("falha ao ler {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("falha ao parsear {}", path.display()))
}

async fn import_familias(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    let familias: Vec<FamiliaJson> = read_json(&data_dir.join("familia.json")).await?;

    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"familias\" (codigo, nome) VALUES ($1, $2)"
        ))
        .await?;

    for f in &familias {
        tx.execute(&stmt, &[&f.codigo, &f.nome]).await?;
    }

    println!("  familias: {} registros", familias.len());
    Ok(())
}

async fn import_formas_atuacao(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    let formas: Vec<FormaAtuacaoJson> = read_json(&data_dir.join("forma-atuacao.json")).await?;

    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"formas_atuacao\" (codigo, titulo, descricao) VALUES ($1, $2, $3)"
        ))
        .await?;

    for f in &formas {
        let codigo: i32 = f
            .codigo
            .parse()
            .with_context(|| format!("codigo de forma de atuação inválido: {}", f.codigo))?;
        tx.execute(&stmt, &[&codigo, &f.titulo, &f.descricao])
            .await?;
    }

    println!("  formas_atuacao: {} registros", formas.len());
    Ok(())
}

async fn import_ocupacoes(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    let ocupacoes: Vec<OcupacaoJson> = read_json(&data_dir.join("ocupacao.json")).await?;

    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"ocupacoes\" (codigo, nome, descricao, familia, cnae) \
             VALUES ($1, $2, $3, $4, $5)"
        ))
        .await?;

    for o in &ocupacoes {
        let codigo: i32 = o
            .codigo
            .parse()
            .with_context(|| format!("codigo de ocupação inválido: {}", o.codigo))?;
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
    }

    println!("  ocupacoes: {} registros", ocupacoes.len());
    Ok(())
}

static TABLES: &[Table] = &[
    Table {
        name: "familias",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("codigo", "CHAR(1) PRIMARY KEY"),
            Column::text("nome", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "formas_atuacao",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::int("codigo", "INTEGER PRIMARY KEY"),
            Column::text("titulo", "TEXT NOT NULL"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "ocupacoes",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::int("codigo", "INTEGER PRIMARY KEY"),
            Column::text("nome", "TEXT NOT NULL"),
            Column::text("descricao", "TEXT NOT NULL"),
            Column::text(
                "familia",
                "CHAR(1) NOT NULL REFERENCES \"{schema}\".\"familias\"(\"codigo\")",
            ),
            Column::text("cnae", "CHAR(7) NOT NULL"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"ocupacoes\" (\"cnae\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
];
