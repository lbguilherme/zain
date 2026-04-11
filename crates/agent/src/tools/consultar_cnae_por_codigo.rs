use cubos_sql::sql;
use deadpool_postgres::Pool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Código CNAE, com ou sem formatação (ex: '4520-0/01' ou '4520001')
    codigo: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "consultar_cnae_por_codigo",
            description: "Verifica se um código CNAE específico está na lista de atividades permitidas para MEI. Use quando o cliente informar um código CNAE (ex: '4520-0/01') e quiser saber se pode ser MEI. Consulta rápida, sem mensagem de espera. Retorna pode_ser_mei (bool) e a lista de ocupações MEI que batem com o código. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta ao cliente baseada no resultado. Nunca termine o turno sem responder ao cliente.",
            consequential: false,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            match run(&ctx.pool, ctx.client_id, &args.codigo).await {
                Ok(value) => ToolOutput::new(value, memory),
                Err(value) => ToolOutput::err(value, memory),
            }
        }),
        must_use_tool_result: false,
    }
}

async fn run(pool: &Pool, client_id: Uuid, codigo_raw: &str) -> Result<Value, Value> {
    let codigo_norm: String = codigo_raw.chars().filter(|c| c.is_ascii_digit()).collect();

    if codigo_norm.is_empty() {
        return Err(json!({ "erro": "código CNAE vazio" }));
    }

    let pattern = format!("{}%", codigo_norm);
    let rows = sql!(
        pool,
        "SELECT ocupacao, cnae_subclasse_id, cnae_descricao
         FROM mei_cnaes.ocupacoes
         WHERE cnae_subclasse_id LIKE $pattern
         ORDER BY ocupacao
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
                        "codigo": row.cnae_subclasse_id.trim(),
                        "ocupacao": row.ocupacao,
                        "descricao": row.cnae_descricao,
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
                client_id = %client_id,
                error = %e,
                "Falha na query de CNAE por código"
            );
            Err(json!({ "erro": format!("Falha ao consultar CNAE: {}", e) }))
        }
    }
}
