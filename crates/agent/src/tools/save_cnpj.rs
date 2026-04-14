use cubos_sql::sql;
use deadpool_postgres::Pool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, pgfn, typed_handler};
use crate::validators;

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// CNPJ (apenas números, 14 dígitos)
    cnpj: String,
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "save_cnpj",
            description: "Salva o CNPJ do lead no cadastro.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            if !validators::validar_cnpj(&args.cnpj) {
                return ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": "CNPJ inválido — os dígitos verificadores não batem. Peça o CNPJ correto ao cliente de forma amigável."
                    }),
                    memory,
                );
            }
            let cnpj_digits: String = args.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();

            // 1) Consulta SIMEI ANTES de salvar. Se não for MEI,
            //    retornamos erro e o CNPJ fica sem persistência.
            let mei = match consultar_simei(&ctx.pool, ctx.client_id, &cnpj_digits).await {
                Ok(v) => v,
                Err(value) => return ToolOutput::err(value, memory),
            };

            // 2) Checa pendências PGFN. Se tiver dívida acima do
            //    limite, recusa antes de persistir qualquer coisa.
            if let Err(err_value) = pgfn::check_debt(&ctx.pool, ctx.client_id, &cnpj_digits).await {
                return ToolOutput::err(err_value, memory);
            }

            // 3) MEI confirmado + sem pendência relevante: salva o
            //    CNPJ e zera a flag `quer_abrir_mei` — se já tem CNPJ
            //    MEI ativo, não tem motivo pra abrir um novo.
            let cnpj_opt: Option<&str> = Some(&cnpj_digits);
            let quer_abrir_mei_false: Option<bool> = Some(false);
            let client_id = ctx.client_id;
            if let Err(e) = sql!(
                &ctx.pool,
                "UPDATE zain.clients
                 SET cnpj           = $cnpj_opt,
                     quer_abrir_mei = $quer_abrir_mei_false,
                     updated_at     = now()
                 WHERE id = $client_id"
            )
            .execute()
            .await
            {
                tracing::warn!(client_id = %ctx.client_id, error = %e, "save_cnpj: falha ao salvar");
                return ToolOutput::err(
                    json!({ "status": "erro", "mensagem": format!("Falha ao salvar: {e}") }),
                    memory,
                );
            }
            ToolOutput::new(mei, memory)
        }),
        must_use_tool_result: false,
        enabled_when: None,
    }
}

/// Consulta o Portal do Simples Nacional (com cache de 48h). Retorna
/// `Ok(valor_sucesso)` quando é MEI ativo; `Err(valor_erro)` quando
/// não é MEI (outro regime, inexistente) ou a consulta falhou.
async fn consultar_simei(pool: &Pool, client_id: Uuid, cnpj: &str) -> Result<Value, Value> {
    match load_cache(pool, cnpj).await {
        Ok(Some(cached)) => {
            tracing::info!(
                client_id = %client_id,
                cnpj = %cnpj,
                "save_cnpj: cache hit SIMEI (<48h)"
            );
            return interpret_consulta(&cached);
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "save_cnpj: falha ao ler cache SIMEI");
        }
    }

    tracing::info!(
        client_id = %client_id,
        cnpj = %cnpj,
        "save_cnpj: consultando SIMEI via rpa::mei (~15-30s)"
    );
    let start = std::time::Instant::now();

    match rpa::mei::consultar_optante(cnpj).await {
        Ok(consulta) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::info!(
                client_id = %client_id,
                cnpj = %cnpj,
                elapsed_ms = elapsed_ms as u64,
                optante_simei = consulta.situacao_simei.optante,
                optante_simples = consulta.situacao_simples.optante,
                nome_empresarial = %consulta.nome_empresarial,
                "save_cnpj: consulta SIMEI concluída"
            );

            let raw = json!({
                "optante_simei": consulta.situacao_simei.optante,
                "simei_desde": consulta.situacao_simei.desde,
                "optante_simples": consulta.situacao_simples.optante,
                "simples_desde": consulta.situacao_simples.desde,
                "nome_empresarial": consulta.nome_empresarial,
                "data_consulta": consulta.data_consulta,
            });

            if let Err(e) = save_cache(pool, cnpj, &raw).await {
                tracing::warn!(client_id = %client_id, error = %e, "save_cnpj: falha ao salvar cache SIMEI");
            }

            interpret_consulta(&raw)
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis();
            tracing::warn!(
                client_id = %client_id,
                cnpj = %cnpj,
                elapsed_ms = elapsed_ms as u64,
                error = %e,
                "save_cnpj: falha na consulta SIMEI"
            );
            Err(json!({
                "status": "erro",
                "mensagem": format!("Consulta ao Portal do Simples Nacional falhou: {}. Tente novamente em alguns minutos.", e),
            }))
        }
    }
}

/// Traduz o JSON bruto da consulta RPA em:
/// - `Ok(value)` quando é MEI ativo (com `status: ok`, nome_empresarial, etc.);
/// - `Err(value)` quando não é MEI (outro regime ou inexistente).
fn interpret_consulta(raw: &Value) -> Result<Value, Value> {
    let optante_simei = raw
        .get("optante_simei")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let optante_simples = raw
        .get("optante_simples")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let nome_empresarial = raw
        .get("nome_empresarial")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let simei_desde = raw.get("simei_desde").cloned().unwrap_or(Value::Null);

    if optante_simei {
        return Ok(json!({
            "status": "ok",
            "nome_empresarial": nome_empresarial,
            "simei_desde": simei_desde,
            "mensagem": "CNPJ é MEI ativo e foi salvo. Pode seguir com a coleta de CPF.",
        }));
    }

    if optante_simples {
        return Err(json!({
            "status": "erro",
            "motivo": "nao_e_mei_outro_regime",
            "mensagem": "CNPJ existe mas não é MEI — está em outro regime do Simples Nacional. O CNPJ NÃO foi salvo. Recuse o lead gentilmente com recusar_lead(motivo='CNPJ não é SIMEI (outro regime)').",
        }));
    }

    Err(json!({
        "status": "erro",
        "motivo": "nao_e_mei",
        "mensagem": "CNPJ não é optante SIMEI. O CNPJ NÃO foi salvo. Recuse o lead gentilmente com recusar_lead(motivo='CNPJ não é SIMEI').",
    }))
}

async fn load_cache(pool: &Pool, cnpj: &str) -> anyhow::Result<Option<Value>> {
    let row = sql!(
        pool,
        "SELECT resultado
         FROM zain.simei_cache
         WHERE cnpj = $cnpj
           AND consulted_at > now() - interval '48 hours'"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| r.resultado))
}

async fn save_cache(pool: &Pool, cnpj: &str, resultado: &Value) -> anyhow::Result<()> {
    let resultado = resultado.clone();
    sql!(
        pool,
        "INSERT INTO zain.simei_cache (cnpj, resultado, consulted_at)
         VALUES ($cnpj, $resultado, now())
         ON CONFLICT (cnpj) DO UPDATE
         SET resultado    = EXCLUDED.resultado,
             consulted_at = EXCLUDED.consulted_at"
    )
    .execute()
    .await?;
    Ok(())
}
