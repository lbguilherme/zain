//! Tool unificada de lookup de CNAE MEI.
//!
//! Aceita tanto um código CNAE (ex: `"4520-0/01"`, `"4520001"`) quanto
//! uma descrição livre da atividade (ex: `"conserto de celular"`,
//! `"doces artesanais"`). A detecção é pelo conteúdo: se o argumento
//! não tem nenhuma letra ASCII, tratamos como código e fazemos busca
//! exata por prefixo; caso contrário, passamos pelo pipeline de
//! embedding + similaridade vetorial.

use deadpool_postgres::Pool;
use pgsafe::sql;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {
    /// Código CNAE ou descrição livre da atividade. Se o texto for
    /// puramente numérico (com pontuação opcional, ex: '4520-0/01' ou
    /// '4520001'), a tool faz busca por prefixo do código. Caso
    /// contrário (ex: 'vendo bolo no pote', 'conserto celular'), faz
    /// busca semântica por similaridade.
    pub descricao_ou_codigo: String,
}

pub async fn run(state: &AppState, client_id: Uuid, args: Args) -> Value {
    let input = args.descricao_ou_codigo.trim();
    if input.is_empty() {
        return json!({ "erro": "argumento vazio" });
    }

    // Tira tudo que não é alfanumérico (pontuação típica de CNAE como
    // '-', '/', '.' e espaços) e verifica se o que sobrou são até 7
    // dígitos numéricos (uma subclasse CNAE completa tem 7 dígitos;
    // menos que isso ainda dá pra buscar por prefixo). Caso contrário,
    // é descrição livre.
    let stripped: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let looks_like_code =
        !stripped.is_empty() && stripped.len() <= 7 && stripped.chars().all(|c| c.is_ascii_digit());

    let outcome = if looks_like_code {
        run_code(&state.pool, client_id, &stripped).await
    } else {
        run_semantic(
            &state.pool,
            &state.ai,
            &state.models.embedding,
            client_id,
            input,
        )
        .await
    };

    match outcome {
        Ok(value) => value,
        Err(value) => value,
    }
}

/// Lookup por código CNAE — busca por prefixo. `codigo` precisa vir já
/// normalizado (só dígitos, até 7 caracteres). Aceita prefixos curtos
/// pra cobrir o caso do cliente passar só os primeiros dígitos da
/// família/grupo (ex: "4520" → todos os 4520-x/xx).
async fn run_code(pool: &Pool, client_id: Uuid, codigo: &str) -> Result<Value, Value> {
    let pattern = format!("{codigo}%");
    let rows = sql!(
        pool,
        "SELECT o.nome AS ocupacao, o.cnae, s.descricao
         FROM mei_cnaes.ocupacoes o
         JOIN cnae.subclasses s ON s.id = o.cnae
         WHERE o.cnae LIKE $pattern
         ORDER BY o.nome
         LIMIT 6"
    )
    .fetch_all()
    .await;

    match rows {
        Ok(rows) => {
            let matches: Vec<Value> = rows
                .iter()
                .map(|row| {
                    json!({
                        "codigo": row.cnae.trim(),
                        "ocupacao": row.ocupacao,
                        "descricao": row.descricao,
                    })
                })
                .collect();
            if matches.is_empty() {
                Ok(json!(format!("Nenhum CNAE encontrado para: {}", codigo)))
            } else {
                Ok(json!(matches))
            }
        }
        Err(e) => {
            tracing::warn!(
                %client_id,
                error = %e,
                "Falha na query de CNAE por código"
            );
            Err(json!({ "erro": format!("Falha ao consultar CNAE: {}", e) }))
        }
    }
}

async fn run_semantic(
    pool: &Pool,
    ai: &ai::Client,
    embedding_model: &str,
    client_id: Uuid,
    descricao: &str,
) -> Result<Value, Value> {
    let embedding = match ai.embed(embedding_model, descricao, None).await {
        Ok(v) => {
            let half: Vec<half::f16> = v.into_iter().map(half::f16::from_f32).collect();
            pgvector::HalfVector::from(half)
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %e, "Falha ao gerar embedding");
            return Err(json!({ "erro": format!("Falha ao gerar embedding: {}", e) }));
        }
    };

    let rows = sql!(
        pool,
        "SELECT s.id AS codigo,
            s.descricao,
            o.nome AS ocupacao
         FROM cnae.subclasses s
         JOIN mei_cnaes.ocupacoes o ON o.cnae = s.id
         ORDER BY s.embedding <=> $embedding
         LIMIT 6"
    )
    .fetch_all()
    .await;

    match rows {
        Ok(rows) => {
            let matches: Vec<Value> = rows
                .iter()
                .map(|row| {
                    json!({
                        "codigo": row.codigo.trim(),
                        "ocupacao": row.ocupacao,
                        "descricao": row.descricao,
                    })
                })
                .collect();
            Ok(json!({
                "pode_ser_mei": !matches.is_empty(),
                "matches": matches,
            }))
        }
        Err(e) => {
            tracing::warn!(
                %client_id,
                error = %e,
                "Falha na busca de CNAE por atividade"
            );
            Err(json!({ "erro": format!("Falha ao buscar CNAE: {}", e) }))
        }
    }
}
