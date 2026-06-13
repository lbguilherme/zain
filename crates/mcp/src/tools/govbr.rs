//! Tools de autenticação gov.br.
//!
//! Dividida em duas porque o fluxo é inerentemente interativo: primeiro
//! chegam `cpf + senha`, o login às vezes para num 2FA, e só depois
//! chega o código OTP que o usuário recebe no app/SMS. O caller MCP
//! conduz essa conversa ping-pong chamando as duas tools em turnos
//! diferentes.
//!
//! Persistência: `govbr_cpf` e `govbr_password` vão em colunas
//! dedicadas de `zain.clients` (para sobreviver a reinicializações
//! entre os dois passos). O OTP nunca é salvo — chega pelo argumento
//! da tool e é descartado após o uso.

use deadpool_postgres::Pool;
use pgsafe::sql;
use rpa::govbr::{CheckOutcome, GovbrError, Profile, check_govbr_profile, session::SavedSession};
use rpa::mei::{CertificadoMei, ElegibilidadeMei};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::pgfn;
use crate::errlog::{self, ErrChain};
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct AuthArgs {
    /// Senha do gov.br.
    pub senha: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OtpArgs {
    /// Código OTP de 6 dígitos recebido pelo app/SMS do gov.br.
    pub otp: String,
}

pub async fn run_auth(state: &AppState, client_id: Uuid, args: AuthArgs) -> Value {
    let cpf = match load_cpf(&state.pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return json!({
                "status": "erro",
                "mensagem": "CPF ainda não foi salvo — chame save_cpf com o CPF antes de tentar autenticar no gov.br.",
            });
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "auth_govbr: falha ao ler CPF");
            return json!({
                "status": "erro",
                "mensagem": "Não consegui ler o CPF do cadastro agora. Tente de novo em instantes.",
            });
        }
    };

    if let Err(e) = save_credentials(&state.pool, client_id, &cpf, &args.senha).await {
        tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "auth_govbr: falha ao salvar credenciais");
        return json!({
            "status": "erro",
            "mensagem": "Não consegui salvar as credenciais agora. Tente de novo em instantes.",
        });
    }

    tracing::info!(%client_id, "auth_govbr: iniciando login gov.br");
    let outcome = check_govbr_profile(&cpf, &args.senha, None, None).await;
    dispatch_outcome(state, client_id, &cpf, outcome).await
}

pub async fn run_otp(state: &AppState, client_id: Uuid, args: OtpArgs) -> Value {
    let otp_digits: String = args.otp.chars().filter(|c| c.is_ascii_digit()).collect();
    if otp_digits.len() != 6 {
        return json!({
            "status": "erro",
            "mensagem": "Código OTP inválido — deve ter exatamente 6 dígitos.",
        });
    }

    let creds = match load_credentials(&state.pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return json!({
                "status": "erro",
                "mensagem": "Ainda não há credenciais gov.br salvas — chame auth_govbr primeiro com a senha.",
            });
        }
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "auth_govbr_otp: falha ao carregar credenciais");
            return json!({
                "status": "erro",
                "mensagem": "Não consegui carregar as credenciais agora. Tente de novo em instantes.",
            });
        }
    };

    tracing::info!(%client_id, "auth_govbr_otp: tentando login com OTP");
    let outcome = check_govbr_profile(&creds.cpf, &creds.password, Some(&otp_digits), None).await;
    dispatch_outcome(state, client_id, &creds.cpf, outcome).await
}

async fn dispatch_outcome(
    state: &AppState,
    client_id: Uuid,
    cpf: &str,
    outcome: Result<CheckOutcome, GovbrError>,
) -> Value {
    match outcome {
        Ok(ok) => {
            if let Err(e) = save_success(&state.pool, client_id, &ok).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "govbr auth: falha ao salvar sessão/perfil");
                return json!({
                    "status": "erro",
                    "mensagem": "Autenticação OK, mas não consegui salvar a sessão agora. Tente de novo em instantes.",
                });
            }
            tracing::info!(%client_id, fresh = ok.fresh_login, "govbr auth: sucesso");
            // 1) Consulta o CCMEI pelo CPF. Se já é MEI, persiste e
            //    devolve os dados.
            // 2) Se não é MEI, checa no portal de inscrição se o CPF
            //    tem direito a abrir um (pode estar impedido por
            //    vínculo com outros CNPJs, por exemplo).
            // Falhas em qualquer etapa NÃO invalidam o login — apenas
            // logamos e seguimos. Passamos a sessão recém-validada pra
            // `refresh_mei_status` reusar direto (sem revalidar de novo).
            let extras = refresh_mei_status(state, client_id, cpf, Some(ok.session.clone())).await;
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
            // errada, apagamos a que estava salva — assim o caller tem
            // que coletar uma nova antes de tentar de novo, e a gente
            // não fica repetindo tentativas com uma senha que o
            // próprio gov.br já rejeitou.
            let senha_confirmadamente_errada = detalhe.contains("ERL0003900");
            if senha_confirmadamente_errada
                && let Err(e) = clear_password(&state.pool, client_id).await
            {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "govbr auth: falha ao apagar senha após ERL0003900");
            }
            tracing::info!(
                %client_id,
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
            // Marca OTP pendente: o login fresco parou no 2FA. Enquanto
            // o cliente não mandar o código, o worker de background NÃO
            // deve ficar tentando relogar (cada tentativa dispararia um
            // novo push de 2FA). A flag zera quando `save_success` rodar.
            if let Err(e) = mark_otp_pendente(&state.pool, client_id).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "govbr auth: falha ao marcar otp_pendente");
            }
            tracing::info!(%client_id, "govbr auth: 2FA exigido");
            json!({
                "status": "otp_necessario",
                "mensagem": "O gov.br pediu verificação em duas etapas. Oriente o cliente a abrir o aplicativo \"gov.br\" no celular e clicar em \"Gerar código de acesso\" na parte inferior da tela — isso vai mostrar um código de 6 dígitos. Peça esse código ao cliente e, quando receber, chame a tool auth_govbr_otp passando o código como argumento para concluir o login.",
            })
        }
        Err(e) => {
            // `e` é GovbrError (RPA/browser) — detalhe técnico só no log; o
            // LLM recebe uma mensagem genérica de instabilidade.
            tracing::warn!(%client_id, error = %e.chain_string(), "govbr auth: falha");
            json!({
                "status": "erro",
                "mensagem": "Não consegui concluir o login no gov.br agora — o sistema pode estar instável. Explique ao cliente e peça pra tentar de novo em alguns minutos.",
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
    otp_pendente: bool,
}

async fn load_full_state(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<GovbrFullState>> {
    let row = sql!(
        pool,
        "SELECT govbr_cpf, govbr_password, govbr_session, govbr_otp_pendente
         FROM zain.clients
         WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.map(|r| GovbrFullState {
        cpf: r.govbr_cpf,
        password: r.govbr_password,
        session: r.govbr_session,
        otp_pendente: r.govbr_otp_pendente,
    }))
}

// ── Revalidação de sessão ──────────────────────────────────────────────

/// Revalida a sessão gov.br salva antes de um fluxo consequencial
/// (tipicamente `abrir_empresa`): primeiro tenta reusar a sessão
/// existente; se ela foi rejeitada pelo portal, faz um login fresh com
/// a senha salva. Se tudo der certo, persiste a sessão renovada e
/// devolve. Se qualquer desfecho exigir ação do caller (OTP, senha
/// nova, portal instável), devolve um `Value` pronto pra mandar direto
/// como erro da tool — incluindo uma `mensagem`/`orientacao`
/// instruindo o próximo passo.
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
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_valid_session: falha ao ler estado gov.br");
            return Err(json!({
                "status": "erro",
                "mensagem": "Não consegui ler as credenciais gov.br agora. Tente de novo em instantes.",
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

    tracing::info!(%client_id, "ensure_valid_session: revalidando sessão gov.br");
    let start = std::time::Instant::now();
    let outcome = check_govbr_profile(&cpf, &password, None, Some(&session)).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match outcome {
        Ok(ok) => {
            tracing::info!(
                %client_id,
                elapsed_ms,
                fresh = ok.fresh_login,
                "ensure_valid_session: sessão válida"
            );
            if let Err(e) = save_success(pool, client_id, &ok).await {
                tracing::warn!(
                    %client_id,
                    error = %errlog::anyhow_chain(&e),
                    "ensure_valid_session: falha ao persistir sessão renovada"
                );
            }
            Ok(ok.session)
        }
        Err(GovbrError::MissingOtp) => {
            tracing::info!(%client_id, elapsed_ms, "ensure_valid_session: portal exigiu 2FA");
            if let Err(e) = mark_otp_pendente(pool, client_id).await {
                tracing::warn!(
                    %client_id,
                    error = %errlog::anyhow_chain(&e),
                    "ensure_valid_session: falha ao marcar otp_pendente após MissingOtp"
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
                %client_id,
                elapsed_ms,
                %detalhe,
                "ensure_valid_session: login recusado durante revalidação"
            );
            let senha_confirmadamente_errada = detalhe.contains("ERL0003900");
            if senha_confirmadamente_errada && let Err(e) = clear_password(pool, client_id).await {
                tracing::warn!(
                    %client_id,
                    error = %errlog::anyhow_chain(&e),
                    "ensure_valid_session: falha ao apagar senha após ERL0003900"
                );
            }
            if let Err(e) = clear_session(pool, client_id).await {
                tracing::warn!(
                    %client_id,
                    error = %errlog::anyhow_chain(&e),
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
                %client_id,
                elapsed_ms,
                error = %e.chain_string(),
                "ensure_valid_session: falha ao revalidar sessão gov.br"
            );
            Err(json!({
                "status": "erro",
                "motivo": "validacao_govbr_falhou",
                "mensagem": "Não consegui validar a sessão do gov.br agora. O sistema do gov.br pode estar instável — explique a situação ao cliente de forma direta e peça pra ele tentar de novo em alguns minutos.",
            }))
        }
    }
}

/// Desfecho de uma tentativa de garantir uma sessão gov.br válida pro
/// background, sem nunca falhar de forma ruidosa.
pub(super) enum GovbrSessionOutcome {
    /// Sessão válida (reusada ou relogada), já persistida via `save_success`.
    Valid(SavedSession),
    /// Login fresco parou no 2FA. `govbr_otp_pendente` foi marcado e a
    /// sessão limpa — o worker não tenta de novo; só o fluxo interativo.
    OtpNeeded,
    /// Senha confirmadamente errada (ERL0003900). A senha foi apagada — o
    /// worker para de tentar até o agente coletar uma nova.
    PasswordWrong,
    /// Sem CPF/senha salvos pra sequer tentar.
    NoCredentials,
    /// Portal gov.br instável/indisponível. Transitório — vale retentar.
    Unstable(String),
}

/// Garante uma sessão gov.br válida pra usos de background (worker), sem
/// interação humana. Diferente de [`ensure_valid_session`] (que exige uma
/// sessão pré-existente e devolve payload de erro pro caller MCP), aqui:
///
/// - Se `govbr_otp_pendente` já está marcado, devolve `OtpNeeded` sem
///   tentar nada (evita disparar 2FA repetido a cada ciclo).
/// - Senão, chama [`check_govbr_profile`], que reusa a sessão salva e, se
///   ela morreu, faz login fresco com a senha. 2FA → `OtpNeeded` (+marca a
///   flag); senha errada → `PasswordWrong` (+apaga a senha). O sucesso é
///   persistido e zera a flag (via `save_success`).
pub(super) async fn ensure_govbr_session(pool: &Pool, client_id: Uuid) -> GovbrSessionOutcome {
    let st = match load_full_state(pool, client_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return GovbrSessionOutcome::NoCredentials,
        Err(e) => {
            tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_govbr_session: falha ao ler estado gov.br");
            return GovbrSessionOutcome::Unstable(errlog::anyhow_chain(&e));
        }
    };
    let (Some(cpf), Some(password)) = (st.cpf, st.password) else {
        return GovbrSessionOutcome::NoCredentials;
    };
    if st.otp_pendente {
        // Última tentativa parou no 2FA e nada mudou desde então: não
        // reloga (dispararia outro push). Espera o fluxo interativo.
        return GovbrSessionOutcome::OtpNeeded;
    }

    match check_govbr_profile(&cpf, &password, None, st.session.as_ref()).await {
        Ok(ok) => {
            if let Err(e) = save_success(pool, client_id, &ok).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_govbr_session: falha ao persistir sessão");
            }
            GovbrSessionOutcome::Valid(ok.session)
        }
        Err(GovbrError::MissingOtp) => {
            if let Err(e) = mark_otp_pendente(pool, client_id).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_govbr_session: falha ao marcar otp_pendente");
            }
            GovbrSessionOutcome::OtpNeeded
        }
        Err(GovbrError::InvalidCredentials(detalhe)) => {
            let senha_errada = detalhe.contains("ERL0003900");
            if senha_errada && let Err(e) = clear_password(pool, client_id).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_govbr_session: falha ao apagar senha");
            }
            if let Err(e) = clear_session(pool, client_id).await {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "ensure_govbr_session: falha ao limpar sessão");
            }
            if senha_errada {
                GovbrSessionOutcome::PasswordWrong
            } else {
                GovbrSessionOutcome::Unstable(detalhe)
            }
        }
        Err(e) => GovbrSessionOutcome::Unstable(e.chain_string()),
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
    let nome = &outcome.profile.nome;
    let email = outcome.profile.email.as_deref();
    let telefone = outcome.profile.telefone.as_deref();
    let nivel = outcome.profile.nivel;

    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_session          = $session!,
             govbr_session_valid_at = now(),
             govbr_otp_pendente     = false,
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

/// Marca "OTP pendente": o último login fresco parou num 2FA ainda não
/// resolvido. A flag faz o worker de background parar de tentar relogar
/// sozinho; só o fluxo interativo (com o cliente pra digitar o código)
/// religa. `save_success` zera a flag de volta.
///
/// **NÃO zera `govbr_session`**: o cookie de "navegador confiável" (que
/// vive na sessão salva, separado do cookie de sessão e bem mais longevo)
/// pode ainda valer — jogar o bag inteiro fora forçaria 2FA na próxima e é
/// justamente o que deslogava o cliente. Mantemos o bag; se o cookie
/// confiável estiver bom, o `do_fresh_login` o reusa e pula o 2FA. Se
/// estiver morto, o próximo `save_success` o sobrescreve.
async fn mark_otp_pendente(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "UPDATE zain.clients
         SET govbr_otp_pendente = true,
             updated_at         = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

// ── MEI: consulta + persistência ───────────────────────────────────────

pub(crate) struct MeiExtras {
    mei: Value,
    pode_abrir: Option<bool>,
    motivo_impedimento: Option<String>,
    orientacao: Option<String>,
}

/// Atualiza a situação MEI do cliente e PERSISTE o resultado. Usado tanto
/// pelo login (`dispatch_outcome`, passando a sessão recém-validada em
/// `session`) quanto pelo worker de background (`jobs::mei_refresh`, com
/// `session = None` — nesse caso a sessão é obtida via
/// [`ensure_govbr_session`], que tenta reusar/relogar respeitando a flag
/// `govbr_otp_pendente`).
///
/// Com a sessão em mãos: Tier 1 (`consultar_certificado`) decide "tem MEI?".
/// Se não tem, Tier 2 (`checar_pode_abrir_mei`) decide a elegibilidade —
/// ambos rodam no portal da Receita logado. Sem sessão utilizável, nada é
/// verificado. `mei_consultado_em` é carimbado nos desfechos conclusivos
/// (tem MEI / elegibilidade checada); em instabilidade do portal NÃO é
/// carimbado (pra retentar no próximo ciclo). Nunca propaga erro — o worker
/// ignora o retorno e o caller interativo injeta os campos no prompt.
pub(crate) async fn refresh_mei_status(
    state: &AppState,
    client_id: Uuid,
    cpf: &str,
    session: Option<SavedSession>,
) -> MeiExtras {
    // O portal do CCMEI deixou de ser público — TUDO no MEI agora exige
    // sessão gov.br. No login ela já vem pronta em `session`; no background
    // a gente obtém via `ensure_govbr_session` (reusa/reloga respeitando a
    // flag `govbr_otp_pendente`, sem loop de re-login).
    let saved = match session {
        Some(s) => s,
        None => match ensure_govbr_session(&state.pool, client_id).await {
            GovbrSessionOutcome::Valid(s) => s,
            outcome => {
                // Sem sessão utilizável: não dá pra checar NADA (nem
                // tem-MEI). Deixa "não verificado" sem carimbar nem mexer
                // em dados. OTP pendente / senha errada / sem-credenciais
                // já travam o worker (flag / ausência de senha) e nenhum
                // desses sobe browser nos próximos ciclos.
                let motivo = match &outcome {
                    GovbrSessionOutcome::OtpNeeded => "otp_pendente",
                    GovbrSessionOutcome::PasswordWrong => "senha_invalida",
                    GovbrSessionOutcome::NoCredentials => "sem_credenciais",
                    GovbrSessionOutcome::Unstable(detalhe) => {
                        tracing::warn!(%client_id, %detalhe, "refresh_mei_status: gov.br instável ao obter sessão");
                        // Transitório: espaça a próxima tentativa do worker.
                        // OTP-pendente/senha-errada/sem-credenciais já são
                        // excluídos pela query do worker (flag / colunas), então
                        // só o caso instável precisa de backoff aqui.
                        bump_refresh_backoff(&state.pool, client_id).await;
                        "govbr_instavel"
                    }
                    GovbrSessionOutcome::Valid(_) => "ok",
                };
                tracing::info!(%client_id, motivo, "refresh_mei_status: sem sessão gov.br utilizável; situação MEI não verificada");
                return MeiExtras {
                    mei: Value::Null,
                    pode_abrir: None,
                    motivo_impedimento: None,
                    orientacao: None,
                };
            }
        },
    };

    // Tier 1 — consulta o CCMEI (agora exige login). Tem MEI ativo?
    tracing::info!(%client_id, "refresh_mei_status: consultando CCMEI");
    let start = std::time::Instant::now();
    match rpa::mei::consultar_certificado(&saved, cpf).await {
        Ok(Some(cert)) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let pdf_bytes = cert.pdf.len();
            tracing::info!(
                %client_id,
                elapsed_ms,
                pdf_bytes,
                cnpj = %cert.cnpj,
                "govbr auth: MEI ativo encontrado"
            );
            if let Err(e) = save_mei(&state.pool, client_id, &cert).await {
                tracing::warn!(
                    %client_id,
                    error = %errlog::anyhow_chain(&e),
                    "govbr auth: falha ao persistir dados do MEI"
                );
            }
            let cnpj_digits: String = cert.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
            let orientacao_pgfn = checar_pgfn_cnpj_mei(state, client_id, &cnpj_digits).await;

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
                %client_id,
                elapsed_ms,
                "refresh_mei_status: CPF não tem MEI ativo, checando elegibilidade"
            );
        }
        Err(e) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::warn!(
                %client_id,
                elapsed_ms,
                error = %errlog::anyhow_chain(&e),
                "refresh_mei_status: SIMEI indisponível durante consulta do CCMEI"
            );
            // Instabilidade do portal: NÃO carimba mei_consultado_em (não há
            // resultado conclusivo), mas aplica backoff pra o worker não
            // retentar de hora em hora enquanto o SIMEI estiver fora.
            bump_refresh_backoff(&state.pool, client_id).await;
            return MeiExtras {
                mei: Value::Null,
                pode_abrir: None,
                motivo_impedimento: None,
                orientacao: Some(SIMEI_INDISPONIVEL_ORIENTACAO.to_string()),
            };
        }
    }

    // Tier 2 — não tem MEI ativo: checa elegibilidade pra abrir um, reusando
    // a mesma sessão já validada acima.
    let start = std::time::Instant::now();
    match rpa::mei::checar_pode_abrir_mei(&saved).await {
        Ok(ElegibilidadeMei { pode_abrir, motivo }) => {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            tracing::info!(
                %client_id,
                elapsed_ms,
                pode_abrir,
                motivo = motivo.as_deref().unwrap_or(""),
                "refresh_mei_status: elegibilidade MEI checada"
            );
            if let Err(e) =
                save_elegibilidade(&state.pool, client_id, pode_abrir, motivo.as_deref()).await
            {
                tracing::warn!(%client_id, error = %errlog::anyhow_chain(&e), "refresh_mei_status: falha ao persistir elegibilidade");
            }
            let orientacao = (!pode_abrir).then(|| {
                "O cliente NÃO tem MEI ativo e TAMBÉM NÃO pode abrir um novo — o portal da Receita recusou o acesso ao form de inscrição com o impedimento acima (tipicamente porque o CPF está vinculado a outro CNPJ que bloqueia MEI). Comunique o motivo ao cliente em português claro e, em seguida, chame `recusar_lead` com um motivo curto (ex: 'CPF impedido de abrir MEI: vínculo com outro CNPJ').".to_string()
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
                %client_id,
                elapsed_ms,
                error = %e.chain_string(),
                "govbr auth: SIMEI indisponível durante checagem de elegibilidade"
            );
            MeiExtras {
                mei: Value::Null,
                pode_abrir: None,
                motivo_impedimento: None,
                orientacao: Some(SIMEI_INDISPONIVEL_ORIENTACAO.to_string()),
            }
        }
    }
}

const SIMEI_INDISPONIVEL_ORIENTACAO: &str = "Não foi possível confirmar a situação do MEI do cliente agora: o sistema do governo responsável pelo MEI (SIMEI) está com indisponibilidade. Isso NÃO é um impedimento e o cliente NÃO deve ser recusado. Explique a situação ao cliente de forma direta e honesta — pode citar que o sistema do SIMEI/governo está fora do ar no momento — e peça pra ele tentar de novo em alguns minutos.";

async fn checar_pgfn_cnpj_mei(_state: &AppState, client_id: Uuid, cnpj: &str) -> Option<String> {
    match pgfn::check_debt(&_state.pool, client_id, cnpj).await {
        Ok(()) => None,
        Err(payload) => {
            let motivo = payload.get("motivo").and_then(|v| v.as_str()).unwrap_or("");
            if motivo == "pendencia_cadastral_acima_do_limite" {
                tracing::info!(
                    %client_id,
                    cnpj = %cnpj,
                    "govbr auth: PGFN apontou pendência no CNPJ do MEI — instruindo recusa"
                );
                payload
                    .get("mensagem")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            } else {
                tracing::warn!(
                    %client_id,
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
    let quer_abrir_mei_false = false;
    let mei_ccmei: CertificadoMei = cert.clone();
    let mei_ccmei_pdf = cert.pdf.clone();

    // Cadência do mei_refresh — MEI ATIVO confirmado: base 30 dias (a
    // situação quase nunca muda) × fator de atividade (1/2/4/6) → 30d ativo
    // … 180d inativo. Espaça o gov.br (caro e que desloga). Ver "Cadência
    // das crons" no FLUXOS.md.
    sql!(
        pool,
        "UPDATE zain.clients
         SET cnpj                     = $cnpj_digits,
             quer_abrir_mei           = $quer_abrir_mei_false,
             mei_ccmei                = $mei_ccmei!,
             mei_ccmei_pdf            = $mei_ccmei_pdf,
             mei_consultado_em        = now(),
             mei_refresh_falhas       = 0,
             mei_proxima_tentativa_em = now() + (30 * (CASE
                 WHEN last_activity_at IS NULL                      THEN 6
                 WHEN last_activity_at > now() - interval '7 days'  THEN 1
                 WHEN last_activity_at > now() - interval '30 days' THEN 2
                 WHEN last_activity_at > now() - interval '90 days' THEN 4
                 ELSE 6
             END)) * interval '1 day',
             updated_at               = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

/// Persiste o resultado da checagem de elegibilidade (Tier 2) quando o
/// cliente NÃO tem MEI ativo: grava `mei_pode_abrir`/`mei_impedimento_motivo`,
/// carimba `mei_consultado_em` e limpa qualquer CCMEI obsoleto (sem MEI
/// ativo não há certificado válido).
async fn save_elegibilidade(
    pool: &Pool,
    client_id: Uuid,
    pode_abrir: bool,
    motivo: Option<&str>,
) -> anyhow::Result<()> {
    // Cadência do mei_refresh — SEM MEI ativo (lead em qualificação ou
    // impedido): base 7 dias × fator de atividade → 7d ativo … 42d inativo.
    // Mais curto que o MEI confirmado porque a elegibilidade pode mudar
    // (lead resolve pendência, abre MEI etc.). Ver "Cadência" no FLUXOS.md.
    sql!(
        pool,
        "UPDATE zain.clients
         SET mei_ccmei                 = NULL,
             mei_ccmei_pdf             = NULL,
             mei_pode_abrir            = $pode_abrir,
             mei_impedimento_motivo    = $motivo,
             mei_consultado_em         = now(),
             mei_refresh_falhas        = 0,
             mei_proxima_tentativa_em  = now() + (7 * (CASE
                 WHEN last_activity_at IS NULL                      THEN 6
                 WHEN last_activity_at > now() - interval '7 days'  THEN 1
                 WHEN last_activity_at > now() - interval '30 days' THEN 2
                 WHEN last_activity_at > now() - interval '90 days' THEN 4
                 ELSE 6
             END)) * interval '1 day',
             updated_at                = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

/// Registra uma falha transitória do refresh MEI e espaça a próxima tentativa
/// do worker com backoff exponencial: 1h, 2h, 4h, … saturando em 72h.
///
/// Só afeta a seleção do worker `jobs::mei_refresh` (que filtra por
/// `mei_proxima_tentativa_em`). O fluxo interativo `auth_govbr` nunca consulta
/// essa coluna, então o cliente presente pode tentar de novo na hora. Um
/// desfecho conclusivo (via `save_mei`/`save_elegibilidade`) zera o contador.
///
/// Best-effort: loga e segue em caso de erro de escrita (o backoff é uma
/// otimização, não pode derrubar o ciclo do worker).
async fn bump_refresh_backoff(pool: &Pool, client_id: Uuid) {
    // O cálculo usa o valor ANTIGO de `mei_refresh_falhas` (o Postgres avalia
    // o RHS do SET com os valores correntes da linha): 0 → 1h, 1 → 2h,
    // 2 → 4h, … até o teto de 72h.
    let res = sql!(
        pool,
        "UPDATE zain.clients
         SET mei_refresh_falhas       = mei_refresh_falhas + 1,
             mei_proxima_tentativa_em = now() + LEAST(
                 interval '1 hour' * power(2, mei_refresh_falhas),
                 interval '72 hours'
             ),
             updated_at = now()
         WHERE id = $client_id"
    )
    .execute()
    .await;
    if let Err(e) = res {
        tracing::warn!(%client_id, error = %e.chain_string(), "refresh_mei_status: falha ao gravar backoff");
    }
}
