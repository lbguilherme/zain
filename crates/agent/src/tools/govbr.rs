//! Tools de autenticação gov.br.
//!
//! Dividida em duas porque o fluxo é inerentemente interativo: primeiro
//! chegam `cpf + senha`, o login às vezes para num 2FA, e só depois
//! chega o código OTP que o usuário recebe no app/SMS. O agent conduz
//! essa conversa ping-pong chamando as duas tools em turnos diferentes.
//!
//! Persistência: `govbr_cpf` e `govbr_password` vão em colunas dedicadas
//! de `zain.clients` (para sobreviver a reinicializações entre os dois
//! passos). O OTP nunca é salvo — chega pelo argumento da tool e é
//! descartado após o uso.

use cubos_sql::sql;
use deadpool_postgres::Pool;
use rpa::govbr::{CheckOutcome, GovbrError, Profile, check_govbr_profile, session::SavedSession};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};

#[derive(Deserialize, JsonSchema)]
struct AuthArgs {
    /// Senha do gov.br.
    senha: String,
}

#[derive(Deserialize, JsonSchema)]
struct OtpArgs {
    /// Código OTP de 6 dígitos recebido pelo app/SMS do gov.br.
    otp: String,
}

pub fn auth_tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "auth_govbr",
            description: "Autentica no gov.br usando a senha fornecida e o CPF previamente salvo via save_cpf. Se o SSO pedir 2FA, orienta a chamar auth_govbr_otp na sequência.",
            consequential: true,
            parameters: params_for::<AuthArgs>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: AuthArgs, memory| async move {
            // `must_use_tool_result: true` já força o LLM a ver o
            // resultado antes de encerrar — qualquer desfecho (sucesso,
            // otp_necessário, login_recusado, erro interno) segue pelo
            // mesmo caminho de `::new`, é o LLM que interpreta.
            ToolOutput::new(run_auth(&ctx, &args.senha).await, memory)
        }),
        must_use_tool_result: true,
    }
}

pub fn otp_tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "auth_govbr_otp",
            description: "Completa o login gov.br quando o SSO pediu 2FA, usando o código OTP de 6 dígitos que o usuário recebeu. Reutiliza o CPF e a senha salvos pela chamada anterior de auth_govbr — não precisa repassá-los.",
            consequential: true,
            parameters: params_for::<OtpArgs>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: OtpArgs, memory| async move {
            ToolOutput::new(run_otp(&ctx, &args.otp).await, memory)
        }),
        must_use_tool_result: true,
    }
}

// ── Runners ────────────────────────────────────────────────────────────

async fn run_auth(ctx: &ToolContext, senha: &str) -> Value {
    let cpf = match load_cpf(&ctx.pool, ctx.client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return json!({
                "status": "erro",
                "mensagem": "CPF ainda não foi salvo — chame save_cpf com o CPF antes de tentar autenticar no gov.br.",
            });
        }
        Err(e) => {
            tracing::warn!(client_id = %ctx.client_id, error = %e, "auth_govbr: falha ao ler CPF");
            return json!({
                "status": "erro",
                "mensagem": format!("Falha ao ler CPF: {e}"),
            });
        }
    };

    if let Err(e) = save_credentials(&ctx.pool, ctx.client_id, &cpf, senha).await {
        tracing::warn!(client_id = %ctx.client_id, error = %e, "auth_govbr: falha ao salvar credenciais");
        return json!({
            "status": "erro",
            "mensagem": format!("Falha ao salvar credenciais: {e}"),
        });
    }

    tracing::info!(client_id = %ctx.client_id, "auth_govbr: iniciando login gov.br");
    let outcome = check_govbr_profile(&cpf, senha, None, None).await;
    dispatch_outcome(ctx, outcome).await
}

async fn run_otp(ctx: &ToolContext, otp: &str) -> Value {
    let otp_digits: String = otp.chars().filter(|c| c.is_ascii_digit()).collect();
    if otp_digits.len() != 6 {
        return json!({
            "status": "erro",
            "mensagem": "Código OTP inválido — deve ter exatamente 6 dígitos.",
        });
    }

    let creds = match load_credentials(&ctx.pool, ctx.client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return json!({
                "status": "erro",
                "mensagem": "Ainda não há credenciais gov.br salvas — chame auth_govbr primeiro com a senha.",
            });
        }
        Err(e) => {
            tracing::warn!(client_id = %ctx.client_id, error = %e, "auth_govbr_otp: falha ao carregar credenciais");
            return json!({
                "status": "erro",
                "mensagem": format!("Falha ao carregar credenciais: {e}"),
            });
        }
    };

    tracing::info!(client_id = %ctx.client_id, "auth_govbr_otp: tentando login com OTP");
    let outcome = check_govbr_profile(&creds.cpf, &creds.password, Some(&otp_digits), None).await;
    dispatch_outcome(ctx, outcome).await
}

async fn dispatch_outcome(ctx: &ToolContext, outcome: Result<CheckOutcome, GovbrError>) -> Value {
    match outcome {
        Ok(ok) => {
            if let Err(e) = save_success(&ctx.pool, ctx.client_id, &ok).await {
                tracing::warn!(client_id = %ctx.client_id, error = %e, "govbr auth: falha ao salvar sessão/perfil");
                return json!({
                    "status": "erro",
                    "mensagem": format!("Autenticação OK mas falhou ao persistir: {e}"),
                });
            }
            tracing::info!(client_id = %ctx.client_id, fresh = ok.fresh_login, "govbr auth: sucesso");
            json!({
                "status": "ok",
                "perfil": profile_json(&ok.profile),
            })
        }
        Err(GovbrError::InvalidCredentials(detalhe)) => {
            // ERL0003900 é o código específico de "usuário e/ou senha
            // inválidos". Quando o gov.br confirma que a senha está
            // errada, apagamos a que estava salva — assim o LLM tem
            // que coletar uma nova antes de tentar de novo, e a gente
            // não fica repetindo tentativas com uma senha que o
            // próprio gov.br já rejeitou.
            let senha_confirmadamente_errada = detalhe.contains("ERL0003900");
            if senha_confirmadamente_errada
                && let Err(e) = clear_password(&ctx.pool, ctx.client_id).await
            {
                tracing::warn!(client_id = %ctx.client_id, error = %e, "govbr auth: falha ao apagar senha após ERL0003900");
            }
            tracing::info!(
                client_id = %ctx.client_id,
                %detalhe,
                senha_apagada = senha_confirmadamente_errada,
                "govbr auth: login recusado"
            );
            json!({
                "status": "login_recusado",
                "mensagem_govbr": detalhe,
                "senha_apagada": senha_confirmadamente_errada,
                "orientacao": "O gov.br recusou o login e mostrou a mensagem acima. Interprete o motivo (senha errada, conta bloqueada, código expirado, etc.), explique ao cliente em português claro e, se for caso de senha errada, peça a senha correta e chame auth_govbr de novo.",
            })
        }
        Err(GovbrError::MissingOtp) => {
            tracing::info!(client_id = %ctx.client_id, "govbr auth: 2FA exigido");
            json!({
                "status": "otp_necessario",
                "mensagem": "O gov.br pediu verificação em duas etapas. Peça ao cliente o código de 6 dígitos que ele recebeu no app/SMS e chame auth_govbr_otp com esse código.",
            })
        }
        Err(e) => {
            tracing::warn!(client_id = %ctx.client_id, error = %e, "govbr auth: falha");
            json!({
                "status": "erro",
                "mensagem": format!("Falha ao autenticar no gov.br: {e}"),
            })
        }
    }
}

fn profile_json(profile: &Profile) -> Value {
    json!({
        "nome": profile.nome,
        "email": profile.email,
        "telefone": profile.telefone,
        "nivel": profile.nivel.map(|n| n.as_str()),
    })
}

// ── DB helpers ─────────────────────────────────────────────────────────

struct GovbrCreds {
    cpf: String,
    password: String,
}

async fn save_credentials(
    pool: &Pool,
    client_id: Uuid,
    cpf: &str,
    password: &str,
) -> anyhow::Result<()> {
    let cpf: Option<&str> = Some(cpf);
    let password: Option<&str> = Some(password);
    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_cpf      = $cpf,
             govbr_password = $password,
             updated_at     = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

async fn clear_password(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_password = NULL,
             updated_at     = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

async fn load_cpf(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<String>> {
    let row = sql!(pool, "SELECT cpf FROM zain.clients WHERE id = $client_id")
        .fetch_optional()
        .await?;
    Ok(row.and_then(|r| r.cpf))
}

async fn load_credentials(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<GovbrCreds>> {
    let row = sql!(
        pool,
        "SELECT govbr_cpf, govbr_password
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };
    let (Some(cpf), Some(password)) = (r.govbr_cpf, r.govbr_password) else {
        return Ok(None);
    };
    Ok(Some(GovbrCreds { cpf, password }))
}

async fn save_success(pool: &Pool, client_id: Uuid, outcome: &CheckOutcome) -> anyhow::Result<()> {
    let session: SavedSession = outcome.session.clone();
    let nome: Option<&str> = Some(&outcome.profile.nome);
    let email = outcome.profile.email.as_deref();
    let telefone = outcome.profile.telefone.as_deref();
    let nivel = outcome.profile.nivel;

    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session          = $session!,
             govbr_session_valid_at = now(),
             govbr_nome             = $nome,
             govbr_email            = $email,
             govbr_telefone         = $telefone,
             govbr_nivel            = $nivel,
             updated_at             = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
