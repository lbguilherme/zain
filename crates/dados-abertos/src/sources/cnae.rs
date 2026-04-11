use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio_postgres::Transaction;

use crate::embedding::EmbeddingClient;
use crate::schema::{Column, Table};
use crate::source::{DataSource, Download};

// https://servicodados.ibge.gov.br/api/v2/cnae/subclasses

const API_URL: &str = "https://servicodados.ibge.gov.br/api/v2/cnae/subclasses";
const DATA_VERSION: &str = "2.3";
const EXTRACTOR_VERSION: u32 = 4;

// --- Deserialização do JSON da API ---

#[derive(Deserialize)]
struct SecaoResp {
    id: String,
    descricao: String,
}

#[derive(Deserialize)]
struct DivisaoResp {
    id: String,
    descricao: String,
    secao: SecaoResp,
}

#[derive(Deserialize)]
struct GrupoResp {
    id: String,
    descricao: String,
    divisao: DivisaoResp,
}

#[derive(Deserialize)]
struct ClasseResp {
    id: String,
    descricao: String,
    grupo: GrupoResp,
    observacoes: Vec<String>,
}

#[derive(Deserialize)]
struct SubclasseResp {
    id: String,
    descricao: String,
    classe: ClasseResp,
    atividades: Vec<String>,
    observacoes: Vec<String>,
}

// --- DataSource ---

pub struct CnaeSource;

impl DataSource for CnaeSource {
    fn schema_name(&self) -> &str {
        "cnae"
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
            url: API_URL.to_string(),
            filename: "subclasses.json".to_string(),
        }]
    }

    fn import_data<'a>(
        &self,
        tx: &'a Transaction<'a>,
        temp_schema: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        let data_dir = self.data_dir();
        Box::pin(async move { import_cnae(tx, temp_schema, &data_dir).await })
    }
}

// --- Import ---

async fn import_cnae(tx: &Transaction<'_>, schema: &str, data_dir: &std::path::Path) -> Result<()> {
    let json_path = data_dir.join("subclasses.json");
    let json_data = tokio::fs::read_to_string(&json_path)
        .await
        .with_context(|| format!("falha ao ler {}", json_path.display()))?;
    let subclasses: Vec<SubclasseResp> =
        serde_json::from_str(&json_data).context("falha ao parsear JSON de CNAE")?;

    println!("  {} subclasses lidas do JSON", subclasses.len());

    // Coletar entidades únicas de cada nível
    let mut secoes_map: HashMap<String, String> = HashMap::new();
    let mut divisoes_map: HashMap<String, (String, String)> = HashMap::new();
    let mut grupos_map: HashMap<String, (String, String)> = HashMap::new();
    let mut classes_map: HashMap<String, (String, String, String)> = HashMap::new();
    let mut subclasse_data: Vec<(String, String, String, Option<String>)> = Vec::new();
    let mut atividades_data: Vec<(String, String)> = Vec::new();

    for sc in &subclasses {
        let classe = &sc.classe;
        let grupo = &classe.grupo;
        let divisao = &grupo.divisao;
        let secao = &divisao.secao;

        secoes_map
            .entry(secao.id.clone())
            .or_insert_with(|| titlecase(&secao.descricao));
        divisoes_map
            .entry(divisao.id.clone())
            .or_insert_with(|| (titlecase(&divisao.descricao), secao.id.clone()));
        grupos_map
            .entry(grupo.id.clone())
            .or_insert_with(|| (titlecase(&grupo.descricao), divisao.id.clone()));
        classes_map.entry(classe.id.clone()).or_insert_with(|| {
            (
                titlecase(&classe.descricao),
                grupo.id.clone(),
                classe.observacoes.join("\n"),
            )
        });

        let descricao = titlecase(&sc.descricao);
        let obs = sc.observacoes.join("\n");
        let obs = if obs.is_empty() { None } else { Some(obs) };
        subclasse_data.push((sc.id.clone(), descricao, sc.classe.id.clone(), obs));

        for ativ in &sc.atividades {
            atividades_data.push((sc.id.clone(), normalize_atividade(ativ)));
        }
    }

    // Converter para Vecs ordenados
    let secoes: Vec<(String, String)> = secoes_map.into_iter().collect();
    let divisoes: Vec<(String, (String, String))> = divisoes_map.into_iter().collect();
    let grupos: Vec<(String, (String, String))> = grupos_map.into_iter().collect();
    let classes: Vec<(String, (String, String, String))> = classes_map.into_iter().collect();

    // Gerar embeddings antes dos inserts
    println!("  Gerando embeddings...");
    let cache_dir = std::path::PathBuf::from(".dados_abertos")
        .join("embeddings")
        .join("cnae");
    let embedder = EmbeddingClient::new(cache_dir)?;

    let subclasse_texts: Vec<String> = subclasse_data
        .iter()
        .map(|(_, descricao, _, obs)| {
            format!("# {}\n\n{}", descricao, obs.as_deref().unwrap_or(""))
        })
        .collect();
    println!("    subclasses:");
    let subclasse_embs = embedder.embed_many(&subclasse_texts).await?;

    let atividade_texts: Vec<String> = atividades_data.iter().map(|(_, a)| a.clone()).collect();
    println!("    subclasse_atividades:");
    let atividade_embs = embedder.embed_many(&atividade_texts).await?;

    // Inserir seções
    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"secoes\" (id, descricao) VALUES ($1, $2)"
        ))
        .await?;
    for (id, descricao) in &secoes {
        tx.execute(&stmt, &[id, descricao]).await?;
    }
    println!("  secoes: {} registros", secoes.len());

    // Inserir divisões
    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"divisoes\" (id, descricao, secao_id) VALUES ($1, $2, $3)"
        ))
        .await?;
    for (id, (descricao, secao_id)) in &divisoes {
        tx.execute(&stmt, &[id, descricao, secao_id]).await?;
    }
    println!("  divisoes: {} registros", divisoes.len());

    // Inserir grupos
    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"grupos\" (id, descricao, divisao_id) VALUES ($1, $2, $3)"
        ))
        .await?;
    for (id, (descricao, divisao_id)) in &grupos {
        tx.execute(&stmt, &[id, descricao, divisao_id]).await?;
    }
    println!("  grupos: {} registros", grupos.len());

    // Inserir classes
    let stmt = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"classes\" (id, descricao, grupo_id, observacoes) VALUES ($1, $2, $3, $4)"
        ))
        .await?;
    for (id, (descricao, grupo_id, observacoes)) in &classes {
        let obs: Option<&str> = if observacoes.is_empty() {
            None
        } else {
            Some(observacoes.as_str())
        };
        tx.execute(&stmt, &[id, descricao, grupo_id, &obs]).await?;
    }
    println!("  classes: {} registros", classes.len());

    // Inserir subclasses
    let stmt_sub = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"subclasses\" (id, descricao, classe_id, observacoes, embedding) VALUES ($1, $2, $3, $4, $5)"
        ))
        .await?;
    for ((id, descricao, classe_id, obs), emb) in subclasse_data.iter().zip(&subclasse_embs) {
        let obs_ref: Option<&str> = obs.as_deref();
        tx.execute(&stmt_sub, &[id, descricao, classe_id, &obs_ref, emb])
            .await?;
    }
    println!("  subclasses: {} registros", subclasse_data.len());

    // Inserir atividades
    let stmt_ativ = tx
        .prepare(&format!(
            "INSERT INTO \"{schema}\".\"subclasse_atividades\" (subclasse_id, atividade, embedding) VALUES ($1, $2, $3)"
        ))
        .await?;
    for ((subclasse_id, atividade), emb) in atividades_data.iter().zip(&atividade_embs) {
        tx.execute(&stmt_ativ, &[subclasse_id, atividade, emb])
            .await?;
    }
    println!(
        "  subclasse_atividades: {} registros",
        atividades_data.len()
    );

    Ok(())
}

// --- Normalização de texto ---

fn titlecase(s: &str) -> String {
    capitalize_first(&s.to_lowercase())
}

// --- Normalização de atividades ---

/// Transforma "DORMENTES DE MADEIRA; PRODUÇÃO DE" em
/// "Produção de dormentes de madeira".
///
/// 1. Encontra o último ";"
/// 2. Inverte as partes (depois ; + antes ;)
/// 3. Lowercase + capitaliza primeira letra
fn normalize_atividade(s: &str) -> String {
    let s = s.trim();
    let result = if let Some(pos) = s.rfind(';') {
        let before = s[..pos].trim();
        let after = s[pos + 1..].trim();
        if after.is_empty() {
            before.to_lowercase()
        } else if before.is_empty() {
            after.to_lowercase()
        } else {
            format!("{} {}", after.to_lowercase(), before.to_lowercase())
        }
    } else {
        s.to_lowercase()
    };
    capitalize_first(&result)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper: String = c.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}

// --- Schema ---

static TABLES: &[Table] = &[
    Table {
        name: "secoes",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("id", "CHAR(1) PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
        ],
        extra_ddl: &[],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "divisoes",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("id", "CHAR(2) PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
            Column::text("secao_id", "CHAR(1) NOT NULL"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"divisoes\" (\"secao_id\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "grupos",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("id", "CHAR(3) PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
            Column::text("divisao_id", "CHAR(2) NOT NULL"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"grupos\" (\"divisao_id\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "classes",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("id", "CHAR(5) PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
            Column::text("grupo_id", "CHAR(3) NOT NULL"),
            Column::text("observacoes", "TEXT"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"classes\" (\"grupo_id\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "subclasses",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("id", "CHAR(7) PRIMARY KEY"),
            Column::text("descricao", "TEXT NOT NULL"),
            Column::text("classe_id", "CHAR(5) NOT NULL"),
            Column::text("observacoes", "TEXT"),
            Column::text("embedding", "halfvec NOT NULL"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"subclasses\" (\"classe_id\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
    Table {
        name: "subclasse_atividades",
        file_prefix: "",
        file_count: 0,
        columns: &[
            Column::text("subclasse_id", "CHAR(7) NOT NULL"),
            Column::text("atividade", "TEXT NOT NULL"),
            Column::text("embedding", "halfvec NOT NULL"),
        ],
        extra_ddl: &["CREATE INDEX ON \"{schema}\".\"subclasse_atividades\" (\"subclasse_id\")"],
        has_headers: false,
        delimiter: b';',
        csv_filename: None,
    },
];
