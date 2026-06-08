//! Tool para abrir empresa MEI no Portal do Empreendedor via
//! [`rpa::mei::inscrever_mei`].
//!
//! Requer uma sessão gov.br já validada (coluna `govbr_session` em
//! `zain.clients`, tipicamente populada via `auth_govbr` +
//! `auth_govbr_otp`). Todos os dados do cadastro (RG, contato, CNAEs,
//! endereços) vêm como argumentos da própria chamada — o caller MCP é
//! responsável por coletá-los antes de invocar a tool.

use deadpool_postgres::Pool;
use pgsafe::sql;
use rpa::mei::inscricao::InscricaoMeiError;
use rpa::mei::{Endereco, InscricaoMei, InscricaoMeiOutcome, inscrever_mei};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::govbr;
use crate::state::AppState;

#[derive(Deserialize, JsonSchema)]
pub struct Args {
    /// Número do RG (identidade civil) do titular.
    pub rg_identidade: String,
    /// Órgão emissor do RG (ex: "SSP", "DETRAN").
    pub rg_orgao_emissor: String,
    /// Sigla da UF do órgão emissor do RG, 2 letras (ex: "BA", "SP").
    pub rg_uf_emissor: String,
    /// DDD do telefone de contato (2 dígitos).
    pub telefone_ddd: String,
    /// Número do telefone de contato (8 dígitos, com ou sem hífen).
    pub telefone_numero: String,
    /// E-mail de contato.
    pub email: String,
    /// CNAE da ocupação principal (7 dígitos, com ou sem pontuação).
    /// Precisa ser um CNAE permitido para MEI — valide antes via
    /// `buscar_cnae`.
    pub ocupacao_principal_cnae: String,
    /// CNAEs das ocupações secundárias (até 15). Todos precisam ser da
    /// mesma família do CNAE principal (A = Geral, B = MEI Caminhoneiro).
    #[serde(default)]
    pub ocupacoes_secundarias_cnaes: Vec<String>,
    /// Códigos numéricos das formas de atuação do MEI — pelo menos uma
    /// obrigatória. Valores válidos: 1 (estabelecimento fixo), 2 (internet),
    /// 3 (em local fixo fora da loja), 4 (correio), 5 (porta a porta /
    /// ambulantes), 6 (televenda), 7 (máquinas automáticas).
    pub formas_atuacao: Vec<i32>,
    /// Endereço comercial onde o MEI exercerá a atividade.
    pub endereco_comercial: EnderecoArgs,
    /// Endereço residencial do titular. Se omitido, usa o mesmo do
    /// comercial.
    pub endereco_residencial: Option<EnderecoArgs>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EnderecoArgs {
    /// CEP com 8 dígitos (com ou sem hífen).
    pub cep: String,
    /// Número do endereço.
    pub numero: String,
    /// Complemento (opcional, ex: "ANDAR 2", "SALA 101").
    pub complemento: Option<String>,
    /// Nome do logradouro (sem o tipo — "XV de Novembro", não
    /// "Rua XV de Novembro"). Só é necessário quando o CEP é genérico
    /// (de cidade inteira) e o portal não auto-preenche o logradouro.
    pub logradouro: Option<String>,
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

pub async fn run(state: &AppState, client_id: Uuid, args: Args) -> Value {
    // Revalida a sessão gov.br antes de tudo: tenta reusar a sessão
    // salva e, se falhar, tenta relogar com a senha salva. Se precisar
    // de OTP, a senha ficar inválida, ou o gov.br estiver instável, o
    // helper já devolve um payload pronto com orientação — só
    // propagamos.
    let session = match govbr::ensure_valid_session(&state.pool, client_id).await {
        Ok(s) => s,
        Err(payload) => return payload,
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

    tracing::info!(%client_id, "abrir_empresa: iniciando RPA de inscrição MEI");
    let start = std::time::Instant::now();
    let outcome = inscrever_mei(
        &state.pool,
        state.ai.as_ref(),
        &state.models.chat,
        &session,
        params,
    )
    .await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match outcome {
        Ok(InscricaoMeiOutcome { cnpj, ccmei }) => {
            tracing::info!(
                %client_id,
                elapsed_ms,
                %cnpj,
                ccmei_ok = ccmei.is_some(),
                "abrir_empresa: inscrição concluída"
            );
            // Se o CCMEI veio junto, persiste tudo via save_mei (que
            // também grava CNPJ e zera quer_abrir_mei). Se não veio
            // (SIMEI instável), pelo menos grava o CNPJ pra o lead
            // virar qualificado.
            let persist_result = if let Some(cert) = &ccmei {
                govbr::save_mei(&state.pool, client_id, cert).await
            } else {
                save_cnpj(&state.pool, client_id, &cnpj).await
            };
            if let Err(e) = persist_result {
                tracing::warn!(
                    %client_id,
                    error = %e,
                    "abrir_empresa: falha ao persistir resultado da abertura"
                );
            }
            let mei_value = ccmei
                .as_ref()
                .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            json!({
                "status": "ok",
                "cnpj": cnpj,
                "mei": mei_value,
            })
        }
        Err(e) => {
            tracing::warn!(
                %client_id,
                elapsed_ms,
                error = %e,
                "abrir_empresa: inscrição falhou"
            );
            map_error(&e)
        }
    }
}

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

async fn save_cnpj(pool: &Pool, client_id: Uuid, cnpj: &str) -> anyhow::Result<()> {
    let cnpj_digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let quer_abrir_mei_false = false;
    sql!(
        pool,
        "UPDATE zain.clients
         SET cnpj           = $cnpj_digits,
             quer_abrir_mei = $quer_abrir_mei_false,
             updated_at     = now()
         WHERE id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
