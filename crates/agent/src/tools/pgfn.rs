//! Helper compartilhado para consulta de dívida ativa na PGFN.
//!
//! A consulta custa 15-30s por scraping, então cacheamos em
//! `zain.pgfn_cache` com TTL de 48h. É usada como gate no
//! `save_cpf` e `save_cnpj`: antes de persistir o documento a
//! gente confirma que não existe pendência acima do limite — o
//! objetivo é recusar o lead *antes* de começar a coleta do resto
//! dos dados, em vez de descobrir lá na frente que ele não vai
//! virar cliente.

use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde_json::{Value, json};
use uuid::Uuid;

/// Limite de dívida ativa que a gente tolera. Acima disso o lead é
/// recusado sem salvar o documento. O valor foi herdado da versão
/// anterior (quando PGFN era uma tool separada); continua em aberto
/// se vale a pena aceitar leads com dívida pequena — por enquanto
/// mantemos o mesmo corte.
const LIMITE_DIVIDA_BRL: f64 = 15_000.0;

/// Consulta PGFN (usando cache de 48h) e decide:
/// - `Ok(())` → sem dívida ou dívida dentro do limite, pode salvar.
/// - `Err(valor)` → dívida acima do limite OU consulta falhou; o
///   valor já é o payload pronto pra devolver ao LLM como erro.
pub async fn check_debt(pool: &Pool, client_id: Uuid, documento: &str) -> Result<(), Value> {
    let raw = match fetch_with_cache(pool, client_id, documento).await {
        Ok(v) => v,
        Err(e) => {
            return Err(json!({
                "status": "erro",
                "motivo": "consulta_pgfn_falhou",
                "mensagem": format!(
                    "Falha ao consultar pendências cadastrais ({e}). Tente novamente em alguns minutos."
                ),
            }));
        }
    };

    let tem_divida = raw
        .get("tem_divida")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let total_divida = raw
        .get("total_divida")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if tem_divida && total_divida > LIMITE_DIVIDA_BRL {
        tracing::info!(
            client_id = %client_id,
            documento = %documento,
            total_divida,
            "pgfn: dívida acima do limite, recusando"
        );
        return Err(json!({
            "status": "erro",
            "motivo": "pendencia_cadastral_acima_do_limite",
            "mensagem": "Lead tem pendência cadastral acima do limite aceitável. Recuse gentilmente com recusar_lead(motivo='pendência cadastral acima do limite'). **NÃO mencione** PGFN, dívida ativa, nem o valor — diga apenas que identificou uma pendência cadastral.",
        }));
    }

    tracing::info!(
        client_id = %client_id,
        documento = %documento,
        tem_divida,
        total_divida,
        "pgfn: ok, dentro do limite"
    );
    Ok(())
}

async fn fetch_with_cache(pool: &Pool, client_id: Uuid, documento: &str) -> anyhow::Result<Value> {
    match load_cache(pool, documento).await {
        Ok(Some(cached)) => {
            tracing::info!(
                client_id = %client_id,
                documento = %documento,
                "pgfn: cache hit (<48h)"
            );
            return Ok(cached);
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "pgfn: falha ao ler cache");
        }
    }

    tracing::info!(
        client_id = %client_id,
        documento = %documento,
        "pgfn: iniciando consulta (~15-30s)"
    );
    let start = std::time::Instant::now();

    let consulta = rpa::pgfn::consultar_divida(documento).await?;
    let elapsed_ms = start.elapsed().as_millis();
    tracing::info!(
        client_id = %client_id,
        documento = %documento,
        elapsed_ms = elapsed_ms as u64,
        tem_divida = consulta.tem_divida,
        total_divida = consulta.total_divida,
        nome = ?consulta.nome,
        "pgfn: consulta concluída"
    );

    let result = json!({
        "tem_divida": consulta.tem_divida,
        "total_divida": consulta.total_divida,
        "nome_devedor": consulta.nome,
    });

    if let Err(e) = save_cache(pool, documento, &result).await {
        tracing::warn!(client_id = %client_id, error = %e, "pgfn: falha ao salvar cache");
    }

    Ok(result)
}

async fn load_cache(pool: &Pool, documento: &str) -> anyhow::Result<Option<Value>> {
    let row = sql!(
        pool,
        "SELECT resultado
         FROM zain.pgfn_cache
         WHERE documento = $documento
           AND consulted_at > now() - interval '48 hours'"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| r.resultado))
}

async fn save_cache(pool: &Pool, documento: &str, resultado: &Value) -> anyhow::Result<()> {
    let resultado = resultado.clone();
    sql!(
        pool,
        "INSERT INTO zain.pgfn_cache (documento, resultado, consulted_at)
         VALUES ($documento, $resultado, now())
         ON CONFLICT (documento) DO UPDATE
         SET resultado    = EXCLUDED.resultado,
             consulted_at = EXCLUDED.consulted_at"
    )
    .execute()
    .await?;
    Ok(())
}
