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
use rpa::mei::{CertificadoMei, ElegibilidadeMei};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, pgfn, typed_handler};

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
            description: "Faz o login do cliente no gov.br com a senha que ele forneceu e o CPF já salvo via `save_cpf`. É a ÚNICA forma de descobrir se o cliente já tem um MEI ativo (e, nesse caso, puxar o CNPJ + dados completos do certificado) e também a porta de entrada pra abrir um MEI novo depois via `abrir_empresa`. Chame assim que o cliente mandar a senha do gov.br.",
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
        // Some quando o lead ainda não tem sessão gov.br ativa. Se já
        // tem (sessão válida), esconder a tool evita pedir senha de
        // novo e empurra o agent pro próximo passo do fluxo.
        enabled_when: Some(|client| !client.govbr_autenticado),
    }
}

pub fn otp_tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "auth_govbr_otp",
            description: "Completa o login gov.br quando a chamada anterior de `auth_govbr` retornou pedindo 2FA. Recebe o código de 6 dígitos que o cliente gerou no app gov.br e, se o login der certo, descobre o MEI atual (se houver) igual ao `auth_govbr`. Chame assim que o cliente mandar o código.",
            consequential: true,
            parameters: params_for::<OtpArgs>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: OtpArgs, memory| async move {
            ToolOutput::new(run_otp(&ctx, &args.otp).await, memory)
        }),
        must_use_tool_result: true,
        // Só exposta quando o lead já forneceu a senha do gov.br
        // (via `auth_govbr`) mas ainda não completou o login — i.e.
        // tem password salvo e não tem session. Fora desse estado, a
        // tool não tem o que fazer.
        enabled_when: Some(|client| client.govbr_has_password && !client.govbr_autenticado),
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
    dispatch_outcome(ctx, &cpf, outcome).await
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
    dispatch_outcome(ctx, &creds.cpf, outcome).await
}

async fn dispatch_outcome(
    ctx: &ToolContext,
    cpf: &str,
    outcome: Result<CheckOutcome, GovbrError>,
) -> Value {
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
            // 1) Consulta o CCMEI pelo CPF. Se já é MEI, persiste e
            //    devolve os dados.
            // 2) Se não é MEI, checa no portal de inscrição se o CPF
            //    tem direito a abrir um (pode estar impedido por
            //    vínculo com outros CNPJs, por exemplo).
            // Falhas em qualquer etapa NÃO invalidam o login — apenas
            // logamos e seguimos.
            let extras = consultar_mei_e_elegibilidade(ctx, cpf, &ok.session).await;
            let mut response = json!({
                "status": "ok",
                "perfil": profile_json(&ok.profile),
                "mei": extras.mei,
            });
            if let Some(obj) = response.as_object_mut() {
                if let Some(pode) = extras.pode_abrir {
                    obj.insert("pode_abrir_mei".into(), json!(pode));
                }
                if let Some(motivo) = extras.motivo_impedimento {
                    obj.insert("motivo_impedimento".into(), json!(motivo));
                }
                if let Some(orientacao) = extras.orientacao {
                    obj.insert("orientacao".into(), json!(orientacao));
                }
            }
            response
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
                "mensagem": "O gov.br pediu verificação em duas etapas. Oriente o cliente a abrir o aplicativo \"gov.br\" no celular e clicar em \"Gerar código de acesso\" na parte inferior da tela — isso vai mostrar um código de 6 dígitos. Peça esse código ao cliente e, quando receber, chame a tool auth_govbr_otp passando o código como argumento para concluir o login.",
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

async fn clear_session(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session = NULL,
             updated_at    = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

struct GovbrFullState {
    cpf: Option<String>,
    password: Option<String>,
    session: Option<SavedSession>,
}

async fn load_full_state(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<GovbrFullState>> {
    let row = sql!(
        pool,
        "SELECT govbr_cpf, govbr_password, govbr_session
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| GovbrFullState {
        cpf: r.govbr_cpf,
        password: r.govbr_password,
        session: r.govbr_session,
    }))
}

// ── Revalidação de sessão ──────────────────────────────────────────────

/// Revalida a sessão gov.br salva antes de um fluxo consequencial
/// (tipicamente `abrir_empresa`): primeiro tenta reusar a sessão
/// existente; se ela foi rejeitada pelo portal, faz um login fresh com
/// a senha salva. Se tudo der certo, persiste a sessão renovada e
/// devolve. Se qualquer desfecho exigir ação do LLM (OTP, senha nova,
/// portal instável), devolve um `Value` pronto pra mandar direto como
/// erro da tool — incluindo uma `mensagem`/`orientacao` instruindo o
/// próximo passo.
pub(super) async fn ensure_valid_session(
    pool: &Pool,
    client_id: Uuid,
) -> Result<SavedSession, Value> {
    let state = match load_full_state(pool, client_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err(json!({
                "status": "erro",
                "motivo": "cliente_nao_encontrado",
                "mensagem": "Cliente não encontrado no cadastro.",
            }));
        }
        Err(e) => {
            tracing::warn!(client_id = %client_id, error = %e, "ensure_valid_session: falha ao ler estado gov.br");
            return Err(json!({
                "status": "erro",
                "mensagem": format!("Falha ao ler credenciais gov.br: {e}"),
            }));
        }
    };
    let (Some(cpf), Some(password), Some(session)) = (state.cpf, state.password, state.session)
    else {
        return Err(json!({
            "status": "erro",
            "motivo": "sessao_govbr_ausente",
            "mensagem": "Não há sessão gov.br completa (CPF/senha/cookies) salva pra este cliente. Peça a senha do gov.br ao cliente e chame `auth_govbr` antes de tentar de novo.",
        }));
    };

    tracing::info!(client_id = %client_id, "ensure_valid_session: revalidando sessão gov.br");
    let start = std::time::Instant::now();
    let outcome = check_govbr_profile(&cpf, &password, None, Some(&session)).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match outcome {
        Ok(ok) => {
            tracing::info!(
                client_id = %client_id,
                elapsed_ms,
                fresh = ok.fresh_login,
                "ensure_valid_session: sessão válida"
            );
            if let Err(e) = save_success(pool, client_id, &ok).await {
                tracing::warn!(
                    client_id = %client_id,
                    error = %e,
                    "ensure_valid_session: falha ao persistir sessão renovada"
                );
            }
            Ok(ok.session)
        }
        Err(GovbrError::MissingOtp) => {
            tracing::info!(client_id = %client_id, elapsed_ms, "ensure_valid_session: portal exigiu 2FA");
            // Zera a sessão no DB pra destravar o predicado da tool
            // `auth_govbr_otp` (que só fica visível quando não há
            // sessão ativa + senha salva).
            if let Err(e) = clear_session(pool, client_id).await {
                tracing::warn!(
                    client_id = %client_id,
                    error = %e,
                    "ensure_valid_session: falha ao limpar sessão após MissingOtp"
                );
            }
            Err(json!({
                "status": "erro",
                "motivo": "otp_necessario",
                "mensagem": "A sessão do gov.br expirou e o portal está pedindo verificação em duas etapas pra reabrir. Explique isso ao cliente, oriente ele a abrir o app gov.br, clicar em 'Gerar código de acesso' e mandar os 6 dígitos. Assim que ele mandar, chame `auth_govbr_otp` com o código.",
            }))
        }
        Err(GovbrError::InvalidCredentials(detalhe)) => {
            tracing::info!(
                client_id = %client_id,
                elapsed_ms,
                %detalhe,
                "ensure_valid_session: login recusado durante revalidação"
            );
            // Mesma lógica do auth_govbr: ERL0003900 confirma senha
            // errada, aí apaga a senha. Caso contrário mantém (pode
            // ser conta bloqueada, etc.).
            let senha_confirmadamente_errada = detalhe.contains("ERL0003900");
            if senha_confirmadamente_errada && let Err(e) = clear_password(pool, client_id).await {
                tracing::warn!(
                    client_id = %client_id,
                    error = %e,
                    "ensure_valid_session: falha ao apagar senha após ERL0003900"
                );
            }
            if let Err(e) = clear_session(pool, client_id).await {
                tracing::warn!(
                    client_id = %client_id,
                    error = %e,
                    "ensure_valid_session: falha ao limpar sessão após InvalidCredentials"
                );
            }
            Err(json!({
                "status": "erro",
                "motivo": "credenciais_invalidas",
                "mensagem_govbr": detalhe,
                "senha_apagada": senha_confirmadamente_errada,
                "mensagem": "A sessão do gov.br expirou e o portal recusou um novo login com a senha que estava salva. Explique isso ao cliente (pode ter sido troca de senha), peça a senha atualizada e chame `auth_govbr`.",
            }))
        }
        Err(e) => {
            tracing::warn!(
                client_id = %client_id,
                elapsed_ms,
                error = %e,
                "ensure_valid_session: falha ao revalidar sessão gov.br"
            );
            Err(json!({
                "status": "erro",
                "motivo": "validacao_govbr_falhou",
                "mensagem": format!("Não consegui validar a sessão do gov.br agora: {e}. O sistema do gov.br pode estar instável — explique a situação ao cliente de forma direta e peça pra ele tentar de novo em alguns minutos."),
            }))
        }
    }
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

// ── MEI: consulta + persistência ───────────────────────────────────────

/// Campos MEI-relacionados a serem mesclados na resposta da tool de
/// auth. Montado em [`consultar_mei_e_elegibilidade`].
struct MeiExtras {
    /// Valor do campo `mei` (objeto com os dados quando é MEI ativo;
    /// `null` quando não é ou a consulta falhou).
    mei: Value,
    /// `Some(true/false)` quando a checagem de elegibilidade rodou;
    /// `None` quando não checamos (já tem MEI, ou a checagem falhou).
    pode_abrir: Option<bool>,
    /// Texto do banner de impedimento do portal, quando `pode_abrir ==
    /// Some(false)`.
    motivo_impedimento: Option<String>,
    /// Orientação livre para o LLM quando existe motivo de recusa —
    /// seja por impedimento da inscrição, seja por pendência cadastral
    /// encontrada na PGFN sobre o CNPJ do MEI ativo. Quando presente,
    /// o LLM deve interpretar e chamar `recusar_lead` após informar
    /// o cliente.
    orientacao: Option<String>,
}

/// Depois do login gov.br bem-sucedido: (1) consulta o CCMEI pelo CPF
/// e persiste se achar, (2) se não achar, checa no portal de inscrição
/// se o CPF tem direito a abrir MEI. Falhas em qualquer etapa só viram
/// warning; não propagam — o login em si já deu certo e o LLM precisa
/// saber disso.
async fn consultar_mei_e_elegibilidade(
    ctx: &ToolContext,
    cpf: &str,
    saved: &SavedSession,
) -> MeiExtras {
    tracing::info!(client_id = %ctx.client_id, "govbr auth: consultando CCMEI pelo CPF");
    let start = std::time::Instant::now();
    match rpa::mei::consultar_certificado(cpf).await {
        Ok(Some(cert)) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let pdf_bytes = cert.pdf.len();
            tracing::info!(
                client_id = %ctx.client_id,
                elapsed_ms,
                pdf_bytes,
                cnpj = %cert.cnpj,
                "govbr auth: MEI ativo encontrado"
            );
            if let Err(e) = save_mei(&ctx.pool, ctx.client_id, &cert).await {
                tracing::warn!(
                    client_id = %ctx.client_id,
                    error = %e,
                    "govbr auth: falha ao persistir dados do MEI"
                );
            }
            // Checa PGFN pelo CNPJ do MEI. Dívida acima do limite →
            // orientação de recusa pro LLM. Falhas de consulta (portal
            // offline, etc.) ficam só como warning.
            let cnpj_digits: String = cert.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
            let orientacao_pgfn = checar_pgfn_cnpj_mei(ctx, &cnpj_digits).await;

            let mut v = serde_json::to_value(&cert).unwrap_or(Value::Null);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("pdf_bytes_persistidos".into(), json!(pdf_bytes));
            }
            return MeiExtras {
                mei: v,
                pode_abrir: None,
                motivo_impedimento: None,
                orientacao: orientacao_pgfn,
            };
        }
        Ok(None) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::info!(
                client_id = %ctx.client_id,
                elapsed_ms,
                "govbr auth: CPF não tem MEI, checando elegibilidade"
            );
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::warn!(
                client_id = %ctx.client_id,
                elapsed_ms,
                error = %e,
                "govbr auth: SIMEI indisponível durante consulta do CCMEI"
            );
            // SIMEI fora do ar: não rodamos a checagem de elegibilidade
            // (o mesmo portal costuma derrubar os dois), e sobretudo
            // NÃO declaramos impedimento. Orienta o LLM a tentar mais
            // tarde — um erro não-fatal que não bloqueia o login.
            return MeiExtras {
                mei: Value::Null,
                pode_abrir: None,
                motivo_impedimento: None,
                orientacao: Some(SIMEI_INDISPONIVEL_ORIENTACAO.to_string()),
            };
        }
    }

    // Sem MEI — checa direito a abrir no portal de inscrição.
    let start = std::time::Instant::now();
    match rpa::mei::checar_pode_abrir_mei(saved).await {
        Ok(ElegibilidadeMei { pode_abrir, motivo }) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::info!(
                client_id = %ctx.client_id,
                elapsed_ms,
                pode_abrir,
                motivo = motivo.as_deref().unwrap_or(""),
                "govbr auth: elegibilidade MEI checada"
            );
            let orientacao = (!pode_abrir).then(|| {
                "O cliente NÃO tem MEI ativo e TAMBÉM NÃO pode abrir um novo — o portal da Receita recusou o acesso ao form de inscrição com o impedimento acima (tipicamente porque o CPF está vinculado a outro CNPJ que bloqueia MEI). Comunique o motivo ao cliente em português claro via `send_whatsapp_message` e, em seguida, chame `recusar_lead` com um motivo curto (ex: 'CPF impedido de abrir MEI: vínculo com outro CNPJ').".to_string()
            });
            MeiExtras {
                mei: Value::Null,
                pode_abrir: Some(pode_abrir),
                motivo_impedimento: motivo,
                orientacao,
            }
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::warn!(
                client_id = %ctx.client_id,
                elapsed_ms,
                error = %e,
                "govbr auth: SIMEI indisponível durante checagem de elegibilidade"
            );
            // Mesma lógica do branch CCMEI Err: não declaramos
            // impedimento quando o portal está fora. Só orienta retry.
            MeiExtras {
                mei: Value::Null,
                pode_abrir: None,
                motivo_impedimento: None,
                orientacao: Some(SIMEI_INDISPONIVEL_ORIENTACAO.to_string()),
            }
        }
    }
}

const SIMEI_INDISPONIVEL_ORIENTACAO: &str = "Não foi possível confirmar a situação do MEI do cliente agora: o sistema do governo responsável pelo MEI (SIMEI) está com indisponibilidade. Isso NÃO é um impedimento e o cliente NÃO deve ser recusado. Explique a situação ao cliente de forma direta e honesta — pode citar que o sistema do SIMEI/governo está fora do ar no momento — e peça pra ele tentar de novo em alguns minutos. Em seguida chame `wait_client_message()`. Quando ele voltar, a consulta vai ser refeita automaticamente no próximo `auth_govbr`.";

/// Consulta PGFN pelo CNPJ do MEI ativo. Retorna:
/// - `Some(mensagem)` quando o PGFN apontou pendência acima do limite —
///   o texto já vem pronto de [`pgfn::check_debt`] com instrução ao LLM
///   pra chamar `recusar_lead` sem mencionar PGFN/dívida ao cliente.
/// - `None` quando a consulta foi OK (sem pendência relevante) OU
///   quando ela falhou. Em falha a gente só loga warn — não faz sentido
///   recusar um lead por timeout de scraper logo após um login bem-
///   sucedido; o próximo ponto de checagem (ex: um novo `auth_govbr`)
///   pode tentar de novo.
async fn checar_pgfn_cnpj_mei(ctx: &ToolContext, cnpj: &str) -> Option<String> {
    match pgfn::check_debt(&ctx.pool, ctx.client_id, cnpj).await {
        Ok(()) => None,
        Err(payload) => {
            let motivo = payload.get("motivo").and_then(|v| v.as_str()).unwrap_or("");
            if motivo == "pendencia_cadastral_acima_do_limite" {
                tracing::info!(
                    client_id = %ctx.client_id,
                    cnpj = %cnpj,
                    "govbr auth: PGFN apontou pendência no CNPJ do MEI — instruindo recusa"
                );
                payload
                    .get("mensagem")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            } else {
                tracing::warn!(
                    client_id = %ctx.client_id,
                    cnpj = %cnpj,
                    ?payload,
                    "govbr auth: consulta PGFN do CNPJ MEI falhou"
                );
                None
            }
        }
    }
}

/// Persiste o CCMEI em `zain.clients`: dados estruturados num único
/// JSONB (`mei_ccmei`, mapeado pro próprio struct [`CertificadoMei`])
/// e o PDF numa `bytea` separada. Também força `quer_abrir_mei = false`
/// — se já tem MEI ativo, não tem motivo pra abrir outro.
pub(super) async fn save_mei(
    pool: &Pool,
    client_id: Uuid,
    cert: &CertificadoMei,
) -> anyhow::Result<()> {
    let cnpj_digits: String = cert.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let cnpj_opt: Option<String> = Some(cnpj_digits);
    let quer_abrir_mei_false: Option<bool> = Some(false);
    let mei_ccmei: CertificadoMei = cert.clone();
    let mei_ccmei_pdf: Option<Vec<u8>> = Some(cert.pdf.clone());

    sql!(
        pool,
        "UPDATE zain.clients
         SET cnpj              = $cnpj_opt,
             quer_abrir_mei    = $quer_abrir_mei_false,
             mei_ccmei         = $mei_ccmei!,
             mei_ccmei_pdf     = $mei_ccmei_pdf,
             mei_consultado_em = now(),
             updated_at        = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
