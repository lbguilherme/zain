//! Tool para abrir empresa MEI no Portal do Empreendedor via
//! [`rpa::mei::inscrever_mei`].
//!
//! Requer uma sessão gov.br já validada (coluna `govbr_session` em
//! `zain.clients`, tipicamente populada via `auth_govbr` +
//! `auth_govbr_otp`). Todos os dados do cadastro (RG, contato, CNAEs,
//! endereços) vêm como argumentos da própria chamada — o agent é
//! responsável por coletá-los do cliente via WhatsApp antes de
//! invocar a tool.

use cubos_sql::sql;
use deadpool_postgres::Pool;
use rpa::govbr::session::SavedSession;
use rpa::mei::inscricao::InscricaoMeiError;
use rpa::mei::{Endereco, InscricaoMei, inscrever_mei};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Tool, ToolContext, ToolDef, ToolOutput, params_for, typed_handler};
use crate::dispatch::ClientRow;

/// Predicado que decide se `abrir_empresa` está disponível pro lead
/// neste turno. Exportado pra que o prompt possa esconder partes
/// específicas (ex: a lista de formas de atuação) quando a tool não
/// está ativa, garantindo que os dois lugares fiquem sempre em sincronia.
pub fn is_enabled(client: &ClientRow) -> bool {
    client.quer_abrir_mei == Some(true) && client.cnpj.is_none()
}

#[derive(Deserialize, JsonSchema)]
struct Args {
    /// Número do RG (identidade civil) do titular.
    rg_identidade: String,
    /// Órgão emissor do RG (ex: "SSP", "DETRAN").
    rg_orgao_emissor: String,
    /// Sigla da UF do órgão emissor do RG, 2 letras (ex: "BA", "SP").
    rg_uf_emissor: String,
    /// DDD do telefone de contato (2 dígitos).
    telefone_ddd: String,
    /// Número do telefone de contato (8 dígitos, com ou sem hífen).
    telefone_numero: String,
    /// E-mail de contato.
    email: String,
    /// CNAE da ocupação principal (7 dígitos, com ou sem pontuação).
    /// Precisa ser um CNAE permitido para MEI — valide antes via
    /// `consultar_cnae_por_codigo` ou `buscar_cnae_por_atividade`.
    ocupacao_principal_cnae: String,
    /// CNAEs das ocupações secundárias (até 15). Todos precisam ser da
    /// mesma família do CNAE principal (A = Geral, B = MEI Caminhoneiro).
    #[serde(default)]
    ocupacoes_secundarias_cnaes: Vec<String>,
    /// Códigos numéricos das formas de atuação do MEI — pelo menos uma
    /// obrigatória. Valores válidos: 1 (estabelecimento fixo), 2 (internet),
    /// 3 (em local fixo fora da loja), 4 (correio), 5 (porta a porta /
    /// ambulantes), 6 (televenda), 7 (máquinas automáticas).
    formas_atuacao: Vec<i32>,
    /// Endereço comercial onde o MEI exercerá a atividade.
    endereco_comercial: EnderecoArgs,
    /// Endereço residencial do titular. Se omitido, usa o mesmo do
    /// comercial.
    endereco_residencial: Option<EnderecoArgs>,
}

#[derive(Deserialize, JsonSchema)]
struct EnderecoArgs {
    /// CEP com 8 dígitos (com ou sem hífen).
    cep: String,
    /// Número do endereço.
    numero: String,
    /// Complemento (opcional, ex: "ANDAR 2", "SALA 101").
    complemento: Option<String>,
    /// Nome do logradouro (sem o tipo — "XV de Novembro", não
    /// "Rua XV de Novembro"). Só é necessário quando o CEP é genérico
    /// (de cidade inteira) e o portal não auto-preenche o logradouro.
    logradouro: Option<String>,
}

impl From<EnderecoArgs> for Endereco {
    fn from(e: EnderecoArgs) -> Self {
        Endereco {
            cep: e.cep,
            numero: e.numero,
            complemento: e.complemento,
            logradouro: e.logradouro,
        }
    }
}

pub fn tool() -> Tool {
    Tool {
        def: ToolDef {
            name: "abrir_empresa",
            description: "Abre o MEI do cliente e gera o CNPJ. Retorna `status: ok` + `cnpj` quando a abertura foi concluída. Pode demorar vários minutos.\n\n\
**PRÉ-REQUISITO 1 — autenticação gov.br:** o cliente precisa estar autenticado no gov.br ANTES de chamar esta tool, isto é, `auth_govbr` (e, se for o caso, `auth_govbr_otp`) já retornou `status: ok`.\n\n\
**PRÉ-REQUISITO 2 — TODOS os dados do cadastro coletados.** A tool não lê nada do banco além da sessão gov.br; tudo vai como argumento direto. NÃO chame com campos faltando — colete todos antes, um a um, na conversa natural do WhatsApp, e use `anotar` pra preservar cada dado entre turnos. Dados obrigatórios:\n\n\
1. **`rg_identidade`** — número do RG (identidade civil) do titular.\n\
2. **`rg_orgao_emissor`** — órgão emissor do RG (ex: SSP, DETRAN).\n\
3. **`rg_uf_emissor`** — sigla UF do órgão emissor, 2 letras (ex: BA, SP).\n\
4. **`telefone_ddd`** — DDD do telefone de contato, 2 dígitos.\n\
5. **`telefone_numero`** — número do telefone de contato.\n\
6. **`email`** — e-mail de contato do titular.\n\
7. **`ocupacao_principal_cnae`** — CNAE da ocupação principal (7 dígitos). **NUNCA peça código nem nome exato ao cliente.** Pergunte em linguagem natural o que ele faz, use `buscar_cnae` com a descrição pra encontrar a ocupação que encaixa, e **confirme com o cliente** o nome da ocupação antes de chamar esta tool.\n\
8. **`formas_atuacao`** — pelo menos um código das formas de atuação. **Não peça código nem título literal.** Infira a partir de como o cliente descreveu o trabalho dele (ex: \"vendo pelo Instagram\" → internet; \"tenho loja\" → estabelecimento fixo). Se ainda estiver ambíguo, faça UMA pergunta natural (\"você atende na sua casa, numa loja ou só pela internet?\") e **confirme com o cliente** antes de chamar.\n\
9. **`endereco_comercial`** — objeto com `cep`, `numero`, e `complemento` (opcional). O portal auto-preenche logradouro/bairro/cidade pelo CEP; só passe `logradouro` se o cliente avisar que o CEP é genérico.\n\n\
Dados opcionais:\n\n\
- **`ocupacoes_secundarias_cnaes`** — até 15 CNAEs adicionais, todos da mesma família do principal. **A grande maioria dos MEIs tem só UMA atividade — não pergunte proativamente.** Só inclua se o cliente espontaneamente contar que faz mais de uma coisa diferente (ex: \"vendo doces e faço bolo de casamento por encomenda\"). Se incluir, confirme cada ocupação inferida com o cliente.\n\
- **`endereco_residencial`** — só preencha se for DIFERENTE do comercial. Pergunte explicitamente ao cliente: \"seu endereço residencial é o mesmo do comercial?\". Se for igual (caso mais comum), omita o campo.",
            consequential: true,
            parameters: params_for::<Args>(),
        },
        handler: typed_handler(|ctx: ToolContext, args: Args, memory| async move {
            let session = match load_session(&ctx.pool, ctx.client_id).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    return ToolOutput::err(
                        json!({
                            "status": "erro",
                            "mensagem": "Nenhuma sessão gov.br ativa para este cliente. Chame auth_govbr (e auth_govbr_otp se necessário) antes de tentar abrir a empresa."
                        }),
                        memory,
                    );
                }
                Err(e) => {
                    tracing::warn!(client_id = %ctx.client_id, error = %e, "abrir_empresa: falha ao ler sessão gov.br");
                    return ToolOutput::err(
                        json!({
                            "status": "erro",
                            "mensagem": format!("Falha ao ler sessão gov.br: {e}")
                        }),
                        memory,
                    );
                }
            };

            let params = InscricaoMei {
                rg_identidade: args.rg_identidade,
                rg_orgao_emissor: args.rg_orgao_emissor,
                rg_uf_emissor: args.rg_uf_emissor,
                telefone_ddd: args.telefone_ddd,
                telefone_numero: args.telefone_numero,
                email: args.email,
                ocupacao_principal_cnae: args.ocupacao_principal_cnae,
                ocupacoes_secundarias_cnaes: args.ocupacoes_secundarias_cnaes,
                formas_atuacao: args.formas_atuacao,
                endereco_comercial: args.endereco_comercial.into(),
                endereco_residencial: args.endereco_residencial.map(Into::into),
            };

            tracing::info!(client_id = %ctx.client_id, "abrir_empresa: iniciando RPA de inscrição MEI");
            let start = std::time::Instant::now();
            let outcome = inscrever_mei(
                &ctx.pool,
                ctx.ai.as_ref(),
                &ctx.models.chat,
                &session,
                params,
            )
            .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            match outcome {
                Ok(cnpj) => {
                    tracing::info!(
                        client_id = %ctx.client_id,
                        elapsed_ms,
                        %cnpj,
                        "abrir_empresa: inscrição concluída"
                    );
                    if let Err(e) = save_cnpj(&ctx.pool, ctx.client_id, &cnpj).await {
                        tracing::warn!(client_id = %ctx.client_id, error = %e, "abrir_empresa: falha ao persistir CNPJ gerado");
                    }
                    ToolOutput::new(
                        json!({
                            "status": "ok",
                            "cnpj": cnpj,
                        }),
                        memory,
                    )
                }
                Err(e) => {
                    tracing::warn!(
                        client_id = %ctx.client_id,
                        elapsed_ms,
                        error = %e,
                        "abrir_empresa: inscrição falhou"
                    );
                    ToolOutput::err(map_error(&e), memory)
                }
            }
        }),
        must_use_tool_result: true,
        // Só exposta quando faz sentido abrir um MEI novo: o lead
        // declarou intenção e ainda não tem CNPJ. Ver [`is_enabled`].
        enabled_when: Some(is_enabled),
    }
}

/// Traduz o [`InscricaoMeiError`] num payload JSON com `motivo`
/// estruturado + `mensagem` em texto livre, para o LLM conseguir
/// reagir de forma específica (ex: pedir pra revalidar gov.br vs.
/// pedir outro CEP).
fn map_error(err: &InscricaoMeiError) -> Value {
    let motivo = match err {
        InscricaoMeiError::SessaoInvalida => "sessao_govbr_invalida",
        InscricaoMeiError::Impedimento(_) => "impedimento",
        InscricaoMeiError::CnaeNaoMapeado(_) => "cnae_nao_permitido",
        InscricaoMeiError::OcupacaoIndisponivel(_) => "ocupacao_indisponivel",
        InscricaoMeiError::CnaesFamiliasMistas(_) => "cnaes_familias_mistas",
        InscricaoMeiError::FamiliaDesconhecida(_) => "familia_desconhecida",
        InscricaoMeiError::FormaAtuacaoDesconhecida(_) => "forma_atuacao_desconhecida",
        InscricaoMeiError::FormaAtuacaoIndisponivel { .. } => "forma_atuacao_indisponivel",
        InscricaoMeiError::CepInvalido(_) => "cep_invalido",
        InscricaoMeiError::Cdp(_) => "erro_browser",
        InscricaoMeiError::Other(_) => "erro",
    };
    json!({
        "status": "erro",
        "motivo": motivo,
        "mensagem": err.to_string(),
    })
}

async fn load_session(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<SavedSession>> {
    let row = sql!(
        pool,
        "SELECT govbr_session FROM zain.clients WHERE id = $client_id"
    )
    .fetch_optional()
    .await?;
    Ok(row.and_then(|r| r.govbr_session))
}

async fn save_cnpj(pool: &Pool, client_id: Uuid, cnpj: &str) -> anyhow::Result<()> {
    let cnpj_digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let cnpj_opt: Option<&str> = Some(&cnpj_digits);
    let quer_abrir_mei_false: Option<bool> = Some(false);
    sql!(
        pool,
        "UPDATE zain.clients
         SET cnpj           = $cnpj_opt,
             quer_abrir_mei = $quer_abrir_mei_false,
             updated_at     = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
