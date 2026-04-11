use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// CPF (11 dígitos) ou CNPJ (14 dígitos). Pode vir com ou sem pontuação, a ferramenta normaliza.
    documento: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "consultar_divida_pgfn",
            description: "Consulta se um CPF ou CNPJ possui dívida ativa na PGFN (Procuradoria-Geral da Fazenda Nacional). Use SEMPRE que o cliente informar um CPF ou CNPJ, para verificar se há débitos pendentes. A consulta leva 15-30 segundos. REGRA OBRIGATÓRIA DE USO: chame esta tool na MESMA resposta que o send_whatsapp_message de espera, em sequência, SEM done() entre as duas. Retorna tem_divida (bool), total_divida (valor em R$) e nome_devedor (se encontrado). **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta baseada no resultado. Se tem_divida=true e total_divida > 15000, recuse o lead gentilmente com recusar_lead. Nunca mencione 'PGFN', 'dívida ativa' ou o valor exato pro cliente — diga apenas que identificou uma pendência cadastral.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, props, memory| async move {
            let (value, props) = run(ctx.client_id, &args.documento, props).await;
            ToolOutput {
                value,
                props,
                memory,
            }
        }),
    }
}

async fn run(client_id: Uuid, doc_raw: &str, mut props: Value) -> (Value, Value) {
    let doc_digits: String = doc_raw.chars().filter(|c| c.is_ascii_digit()).collect();

    let is_cpf = doc_digits.len() == 11;
    let is_cnpj = doc_digits.len() == 14;

    if !is_cpf && !is_cnpj {
        tracing::warn!(
            client_id = %client_id,
            documento_recebido = %doc_raw,
            "consultar_divida_pgfn: documento inválido (nem CPF nem CNPJ)"
        );
        return (
            json!({
                "erro": "Documento inválido — deve ser CPF (11 dígitos) ou CNPJ (14 dígitos)",
                "documento_recebido": doc_raw,
            }),
            props,
        );
    }

    if is_cpf && !validators::validar_cpf(&doc_digits) {
        return (
            json!({
                "erro": "CPF inválido — número não passou na validação",
                "documento_recebido": doc_raw,
            }),
            props,
        );
    }

    if is_cnpj && !validators::validar_cnpj(&doc_digits) {
        return (
            json!({
                "erro": "CNPJ inválido — número não passou na validação",
                "documento_recebido": doc_raw,
            }),
            props,
        );
    }

    tracing::info!(
        client_id = %client_id,
        documento = %doc_digits,
        "consultar_divida_pgfn: iniciando consulta (~15-30s)"
    );
    let start = std::time::Instant::now();

    match rpa::pgfn::consultar_divida(&doc_digits).await {
        Ok(consulta) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::info!(
                client_id = %client_id,
                documento = %doc_digits,
                elapsed_ms = elapsed_ms as u64,
                tem_divida = consulta.tem_divida,
                total_divida = consulta.total_divida,
                nome = ?consulta.nome,
                "consultar_divida_pgfn: consulta concluída"
            );

            let result = json!({
                "tem_divida": consulta.tem_divida,
                "total_divida": consulta.total_divida,
                "nome_devedor": consulta.nome,
            });

            if let Some(obj) = props.as_object_mut() {
                obj.insert("ultima_consulta_pgfn".into(), result.clone());
            }

            (result, props)
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::warn!(
                client_id = %client_id,
                documento = %doc_digits,
                elapsed_ms = elapsed_ms as u64,
                error = %e,
                "consultar_divida_pgfn: falha na consulta"
            );
            (
                json!({
                    "erro": format!("Falha ao consultar PGFN: {}", e),
                }),
                props,
            )
        }
    }
}
