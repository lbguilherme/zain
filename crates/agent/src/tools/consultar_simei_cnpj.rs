use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// CNPJ a consultar. Pode vir com ou sem pontuação, a ferramenta normaliza.
    cnpj: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "consultar_simei_cnpj",
            description: "Consulta se um CNPJ é optante pelo SIMEI (ou seja, se é um MEI ativo). Use SEMPRE que o cliente informar um CNPJ, antes de aceitar que ele já tem MEI. A consulta leva 15-30 segundos. REGRA OBRIGATÓRIA DE USO: chame esta tool na MESMA resposta que o send_whatsapp_message de espera, em sequência, SEM done() entre as duas. Exemplo do fluxo correto numa única resposta: send_whatsapp_message('deixa eu dar uma olhada aqui rapidinho') → consultar_simei_cnpj(cnpj='...'). Se você chamar done() antes de consultar_simei_cnpj, a consulta nunca vai rodar e o cliente fica sem resposta. Não mencione 'Receita' ou 'portal' na mensagem de espera — diga só 'aqui'. Retorna optante_simei (bool), simei_desde, optante_simples, nome_empresarial. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta NOVA ao cliente baseada no resultado (ex: 'Confirmado! Vi que você é MEI desde X' ou 'Vi que esse CNPJ não é MEI'). A mensagem de espera anterior NÃO conta como resposta — ela só servia pra avisar que você ia consultar. Nunca termine o turno sem mandar uma mensagem nova contando o que descobriu.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, props, memory| async move {
            let (value, props) = run(ctx.client_id, &args.cnpj, props).await;
            ToolOutput {
                value,
                props,
                memory,
            }
        }),
    }
}

async fn run(client_id: Uuid, cnpj_raw: &str, mut props: Value) -> (Value, Value) {
    let cnpj_digits: String = cnpj_raw.chars().filter(|c| c.is_ascii_digit()).collect();

    if cnpj_digits.len() != 14 || !validators::validar_cnpj(&cnpj_digits) {
        tracing::warn!(
            client_id = %client_id,
            cnpj_recebido = %cnpj_raw,
            "consultar_simei_cnpj: CNPJ inválido"
        );
        return (
            json!({
                "erro": "CNPJ inválido — número não passou na validação",
                "cnpj_recebido": cnpj_raw,
            }),
            props,
        );
    }

    tracing::info!(
        client_id = %client_id,
        cnpj = %cnpj_digits,
        "consultar_simei_cnpj: iniciando consulta via rpa::mei (~15-30s)"
    );
    let start = std::time::Instant::now();

    match rpa::mei::consultar_optante(&cnpj_digits).await {
        Ok(consulta) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::info!(
                client_id = %client_id,
                cnpj = %cnpj_digits,
                elapsed_ms = elapsed_ms as u64,
                optante_simei = consulta.situacao_simei.optante,
                optante_simples = consulta.situacao_simples.optante,
                nome_empresarial = %consulta.nome_empresarial,
                "consultar_simei_cnpj: consulta concluída com sucesso"
            );

            let result = json!({
                "optante_simei": consulta.situacao_simei.optante,
                "simei_desde": consulta.situacao_simei.desde,
                "optante_simples": consulta.situacao_simples.optante,
                "simples_desde": consulta.situacao_simples.desde,
                "nome_empresarial": consulta.nome_empresarial,
                "data_consulta": consulta.data_consulta,
            });

            if let Some(obj) = props.as_object_mut() {
                obj.insert("ultima_consulta_simei".into(), result.clone());
            }

            (result, props)
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::warn!(
                client_id = %client_id,
                cnpj = %cnpj_digits,
                elapsed_ms = elapsed_ms as u64,
                error = %e,
                "consultar_simei_cnpj: falha na consulta"
            );
            (
                json!({
                    "erro": format!("Falha ao consultar: {}", e),
                }),
                props,
            )
        }
    }
}
