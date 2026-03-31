use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, Result};
use tokio_postgres::Transaction;

use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// Anexo XI da Resolução CGSN nº 140, de 22 de maio de 2018
// Ocupações Permitidas ao MEI - Tabelas A e B
// https://www8.receita.fazenda.gov.br/SimplesNacional/Arquivos/manual/Anexo_XI.pdf

const DATA_VERSION: &str = "2018";
const EXTRACTOR_VERSION: u32 = 3;

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

fn capitalize_first(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

async fn import_mei_cnaes(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    let path = data_dir.join("anexo_xi.txt");
    let text = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("falha ao ler {}", path.display()))?;

    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"ocupacoes\" (ocupacao, cnae_subclasse_id, cnae_descricao, tabela, iss, icms) \
             VALUES ($1, $2, $3, $4, $5, $6)"
        ))
        .await?;

    let mut count: u64 = 0;
    let mut ocupacao: Option<String> = None;
    let mut cnae: Option<String> = None;
    let mut cnae_descricao: Option<String> = None;
    let mut tabela: Option<String> = None;
    let mut iss: Option<bool> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(val) = line.strip_prefix("OCUPAÇÃO=") {
            ocupacao = Some(capitalize_first(val.trim()));
        } else if let Some(val) = line.strip_prefix("CNAE=") {
            // Converte "4724-5/00" → "4724500"
            cnae = Some(val.trim().replace(['-', '/'], ""));
        } else if let Some(val) = line.strip_prefix("CNAE_DESCRIÇÃO=") {
            cnae_descricao = Some(capitalize_first(val.trim()));
        } else if let Some(val) = line.strip_prefix("TABELA=") {
            tabela = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("ISS=") {
            iss = Some(val.trim() == "S");
        } else if let Some(val) = line.strip_prefix("ICMS=") {
            let icms = val.trim() == "S";

            if let (Some(o), Some(c), Some(d), Some(t), Some(i)) =
                (&ocupacao, &cnae, &cnae_descricao, &tabela, iss)
            {
                tx.execute(&stmt, &[o, c, d, t, &i, &icms]).await?;
                count += 1;
            }

            ocupacao = None;
            cnae = None;
            cnae_descricao = None;
            tabela = None;
            iss = None;
        }
    }

    println!("  ocupacoes: {count} registros");
    Ok(())
}

static TABLES: &[Table] = &[Table {
    name: "ocupacoes",
    file_prefix: "",
    file_count: 0,
    columns: &[
        Column::text("ocupacao", "TEXT NOT NULL"),
        Column::text("cnae_subclasse_id", "CHAR(7) NOT NULL"),
        Column::text("cnae_descricao", "TEXT NOT NULL"),
        Column::text("tabela", "CHAR(1) NOT NULL"),
        Column::text("iss", "BOOLEAN NOT NULL"),
        Column::text("icms", "BOOLEAN NOT NULL"),
    ],
    extra_ddl: &[
        "CREATE INDEX ON \"{schema}\".\"ocupacoes\" (\"cnae_subclasse_id\")",
    ],
    has_headers: false,
}];
