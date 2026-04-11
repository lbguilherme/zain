use cubos_sql::sql;
use deadpool_postgres::Pool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Descrição livre da atividade que o cliente exerce (ex: 'vendo bolo no pote', 'conserto celular')
    descricao: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "buscar_cnae_por_atividade",
            description: "Busca CNAEs MEI-compatíveis a partir de uma descrição livre da atividade (ex: 'doces artesanais', 'cabelereiro', 'mecânico'). Use quando o cliente descrever o que faz mas não souber o código CNAE. Consulta rápida, sem mensagem de espera. Retorna até 10 ocupações MEI que batem com a descrição. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta ao cliente baseada no resultado. Nunca termine o turno sem responder ao cliente.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            match run(
                &ctx.pool,
                &ctx.ai,
                &ctx.models.embedding,
                ctx.client_id,
                &args.descricao,
            )
            .await
            {
                Ok(value) => ToolOutput::new(value, memory),
                Err(value) => ToolOutput::err(value, memory),
            }
        }),
        must_use_tool_result: false,
    }
}

async fn run(
    pool: &Pool,
    ai: &ai::Client,
    embedding_model: &str,
    client_id: Uuid,
    descricao: &str,
) -> Result<Value, Value> {
    let descricao = descricao.trim();

    if descricao.is_empty() {
        return Err(json!({ "erro": "descrição vazia" }));
    }

    let embedding = match ai.embed(embedding_model, descricao, None).await {
        Ok(v) => {
            let half: Vec<half::f16> = v.into_iter().map(half::f16::from_f32).collect();
            pgvector::HalfVector::from(half)
        }
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "Falha ao gerar embedding");
            return Err(json!({ "erro": format!("Falha ao gerar embedding: {}", e) }));
        }
    };

    let rows = sql!(
        pool,
        "SELECT s.id AS codigo,
            s.descricao AS descricao,
            o.ocupacao AS ocupacao
         FROM cnae.subclasses s
         JOIN mei_cnaes.ocupacoes o ON o.cnae_subclasse_id = s.id
         ORDER BY s.embedding <=> $embedding
         LIMIT 6"
    )
    .fetch_all()
    .await;

    match rows {
        Ok(rows) => {
            let resultados: Vec<Value> = rows
                .iter()
                .map(|row| {
                    json!({
                        "codigo": row.codigo.trim(),
                        "ocupacao": row.ocupacao,
                        "descricao": row.descricao,
                    })
                })
                .collect();

            if resultados.is_empty() {
                Ok(json!({
                    "resultados": [],
                    "mensagem": "Nenhuma ocupação MEI bate com essa descrição. Pode ser uma atividade não permitida para MEI.",
                }))
            } else {
                Ok(json!({ "resultados": resultados }))
            }
        }
        Err(e) => {
            tracing::warn!(
                client_id = %client_id,
                error = %e,
                "Falha na busca de CNAE por atividade"
            );
            Err(json!({ "erro": format!("Falha ao buscar CNAE: {}", e) }))
        }
    }
}
