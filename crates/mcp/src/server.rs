//! Servidor MCP: expõe cada tool de [`crate::tools`] como uma rota do
//! protocolo MCP via macros do `rmcp`. A identidade do cliente é
//! extraída do `_meta` da chamada (campo `client_id` em formato UUID).

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, tool, tool_handler, tool_router};

use crate::client_state::{self, ClientSnapshot, require_enabled};
use crate::meta::{extract_and_ensure_client_id, extract_client_id_opt};
use crate::state::AppState;
use crate::tools;

#[derive(Clone)]
pub struct ZainMcpServer {
    state: Arc<AppState>,
}

impl ZainMcpServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tool_router]
impl ZainMcpServer {
    #[tool(
        description = "Devolve o estado atual do cliente: contato (chat_id, telefone, nome), dados coletados (CPF, CNPJ, intent de MEI, recusa), estado da sessão gov.br (autenticado / aguardando OTP / sessão expirada / não autenticado, nível, nome), situação MEI (já tem MEI ativo / impedido de abrir + motivo / elegível a abrir / não verificada) e situação DAS (meses em atraso com valores, próximo vencimento) e situação DASN (declaração anual: anos em atraso/a declarar/entregues, já cruzados com a vigência do MEI). Leitura barata — situação MEI/DAS/DASN são mantidas frescas por workers de background; pra forçar reconsulta ao vivo use `consultar_das` (cliente disse que pagou) ou `consultar_dasn` (disse que declarou). Use no início de cada turno pra entender onde o lead parou.",
        annotations(
            title = "Obter estado do cliente",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    async fn get_client_state(
        &self,
        Parameters(args): Parameters<tools::get_client_state::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        Ok(tools::get_client_state::run(&self.state, client_id, args).await)
    }

    #[tool(
        description = "Salva o CPF do lead no cadastro.",
        annotations(
            title = "Salvar CPF do lead",
            read_only_hint = false,
            idempotent_hint = true,
            open_world_hint = true,
        )
    )]
    async fn save_cpf(
        &self,
        Parameters(args): Parameters<tools::save_cpf::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        let value = tools::save_cpf::run(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }

    #[tool(
        description = "Registra se a pessoa tem intenção de abrir um MEI novo. Use `true` quando ela disser que quer abrir/começar um MEI (e ainda não tem CNPJ). Use `false` quando ela desistir. Quando a pessoa diz que já tem MEI, NÃO chame esta tool — o `auth_govbr` já persiste o CNPJ automaticamente quando encontra um MEI ativo no CPF do cliente.",
        annotations(
            title = "Registrar intenção de abrir MEI",
            read_only_hint = false,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    async fn save_quer_abrir_mei(
        &self,
        Parameters(args): Parameters<tools::save_quer_abrir_mei::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "save_quer_abrir_mei", client_id).await {
            return Ok(err);
        }
        let value = tools::save_quer_abrir_mei::run(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }

    #[tool(
        description = "Faz o login do cliente no gov.br com a senha que ele forneceu e o CPF já salvo via `save_cpf`. É a ÚNICA forma de descobrir se o cliente já tem um MEI ativo (e, nesse caso, puxar o CNPJ + dados completos do certificado) e também a porta de entrada pra abrir um MEI novo depois via `abrir_empresa`. Chame assim que o cliente mandar a senha do gov.br.",
        annotations(
            title = "Login gov.br",
            read_only_hint = false,
            idempotent_hint = false,
            open_world_hint = true,
        )
    )]
    async fn auth_govbr(
        &self,
        Parameters(args): Parameters<tools::govbr::AuthArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "auth_govbr", client_id).await {
            return Ok(err);
        }
        let value = tools::govbr::run_auth(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }

    #[tool(
        description = "Completa o login gov.br quando a chamada anterior de `auth_govbr` retornou pedindo 2FA. Recebe o código de 6 dígitos que o cliente gerou no app gov.br e, se o login der certo, descobre o MEI atual (se houver) igual ao `auth_govbr`. Chame assim que o cliente mandar o código.",
        annotations(
            title = "Confirmar 2FA gov.br",
            read_only_hint = false,
            idempotent_hint = false,
            open_world_hint = true,
        )
    )]
    async fn auth_govbr_otp(
        &self,
        Parameters(args): Parameters<tools::govbr::OtpArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "auth_govbr_otp", client_id).await {
            return Ok(err);
        }
        let value = tools::govbr::run_otp(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }


    #[tool(
        description = "Marca o lead como recusado — decisão PERMANENTE e irreversível: encerra o caso pra sempre e o lead nunca mais será atendido pela Zain. Use APENAS com sinal claro de impedimento definitivo do próprio cliente (ex: tool retornou pedindo pra recusar, atividade não permitida pra MEI, outro regime empresarial). NUNCA use por falha de sistema/integração (SIMEI fora do ar, gov.br instável, consulta sem resultado, timeout) — nesses casos agende uma retentativa. Na dúvida, NÃO recuse. Antes de chamar, comunique o motivo ao cliente de forma gentil.",
        annotations(
            title = "Recusar lead",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    async fn recusar_lead(
        &self,
        Parameters(args): Parameters<tools::recusar_lead::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "recusar_lead", client_id).await {
            return Ok(err);
        }
        let value = tools::recusar_lead::run(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }

    #[tool(
        description = "Devolve o PDF do Certificado da Condição de MEI (CCMEI) do lead como resource inline no resultado (mime application/pdf). Disponível quando o lead já tem MEI ativo confirmado (via `auth_govbr`) ou recém-aberto (via `abrir_empresa`). Use pra anexar o certificado pro cliente na rodada de confirmação inicial.",
        annotations(
            title = "Obter PDF do CCMEI",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    async fn get_ccmei(
        &self,
        Parameters(args): Parameters<tools::get_ccmei::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "get_ccmei", client_id).await {
            return Ok(err);
        }
        Ok(tools::get_ccmei::run(&self.state, client_id, args).await)
    }

    #[tool(
        description = "Emite a guia mensal do MEI (DAS) no PGMEI e devolve o PDF como resource inline — o documento traz código de barras (boleto) e QR code PIX. `periodo` é a competência YYYYMM (ex: '202604' = abril/2026); omita pra emitir o mês mais antigo em atraso (ou, sem atraso, o próximo a vencer). A guia é sempre emitida na hora, com multa/juros recalculados — nunca reaproveite um PDF antigo. Pode demorar ~30-60s (automação no portal do governo).",
        annotations(
            title = "Emitir guia DAS (boleto/PIX)",
            read_only_hint = false,
            idempotent_hint = true,
            open_world_hint = true,
        )
    )]
    async fn emitir_das(
        &self,
        Parameters(args): Parameters<tools::das::EmitirArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "emitir_das", client_id).await {
            return Ok(err);
        }
        Ok(tools::das::run_emitir(&self.state, client_id, args).await)
    }

    #[tool(
        description = "Reconsulta AO VIVO a situação do DAS (mensalidade do MEI) no portal e atualiza o estado do cliente — devolve meses em atraso e o próximo vencimento, já frescos. Use quando o cliente disser que pagou (pra confirmar se já consta) ou quando quiser o valor mais atual de um mês em atraso. É uma operação cara (sobe browser + captcha, ~30-60s): chame só sob pedido/contexto explícito, NÃO a cada turno — o estado já vem atualizado em background. Atenção: pagamento leva 1-2 dias úteis pra compensar, então logo após pagar o mês ainda pode constar em atraso (é normal).",
        annotations(
            title = "Consultar situação do DAS",
            read_only_hint = false,
            idempotent_hint = true,
            open_world_hint = true,
        )
    )]
    async fn consultar_das(
        &self,
        Parameters(args): Parameters<tools::das::ConsultarArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "consultar_das", client_id).await {
            return Ok(err);
        }
        Ok(tools::das::run_consultar(&self.state, client_id, args).await)
    }

    #[tool(
        description = "Reconsulta AO VIVO o status da declaração anual do MEI (DASN-SIMEI) no portal e atualiza o estado — devolve os anos em atraso, a declarar e já entregues. A DASN é o relatório anual de faturamento (vence 31/05 do ano seguinte). Use quando o cliente disser que entregou a declaração (pra confirmar) ou pedir a situação atualizada. Caro (~30-60s, browser + captcha): chame só sob pedido explícito — o estado já vem atualizado em background (a DASN muda raríssimo). Só marca atraso em ano dentro da vigência do MEI.",
        annotations(
            title = "Consultar declaração anual (DASN)",
            read_only_hint = false,
            idempotent_hint = true,
            open_world_hint = true,
        )
    )]
    async fn consultar_dasn(
        &self,
        Parameters(args): Parameters<tools::dasn::ConsultarArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "consultar_dasn", client_id).await {
            return Ok(err);
        }
        Ok(tools::dasn::run_consultar(&self.state, client_id, args).await)
    }

    #[tool(
        description = "Busca ocupações MEI-compatíveis a partir de um código CNAE (ex: '4520-0/01', '4520001') ou de uma descrição livre da atividade (ex: 'doces artesanais', 'conserto celular'). Use quando o cliente descrever o que ele faz pra validar se a atividade encaixa como MEI e pra achar o CNAE correto antes de chamar `abrir_empresa`.",
        annotations(
            title = "Buscar CNAE MEI",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false,
        )
    )]
    async fn buscar_cnae(
        &self,
        Parameters(args): Parameters<tools::buscar_cnae::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        let value = tools::buscar_cnae::run(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }

    #[tool(
        description = "Abre o MEI do cliente e gera o CNPJ no Portal do Empreendedor. Pode demorar vários minutos. Chame quando o cliente já está logado no gov.br (via `auth_govbr`) e você já coletou TODOS os dados exigidos pelo cadastro.\n\n\
**PRÉ-REQUISITO 1 — autenticação gov.br:** o cliente precisa estar autenticado no gov.br ANTES de chamar esta tool, isto é, `auth_govbr` (e, se for o caso, `auth_govbr_otp`) já concluiu com sucesso.\n\n\
**PRÉ-REQUISITO 2 — TODOS os dados do cadastro coletados.** A tool não lê nada do banco além da sessão gov.br; tudo vai como argumento direto. NÃO chame com campos faltando — colete todos antes, um a um, na conversa natural com o cliente, e use `anotar` pra preservar cada dado entre turnos. Dados obrigatórios:\n\n\
1. **`rg_identidade`** — número do RG (identidade civil) do titular.\n\
2. **`rg_orgao_emissor`** — órgão emissor do RG (ex: SSP, DETRAN).\n\
3. **`rg_uf_emissor`** — sigla UF do órgão emissor, 2 letras (ex: BA, SP).\n\
4. **`telefone_ddd`** — DDD do telefone de contato, 2 dígitos.\n\
5. **`telefone_numero`** — número do telefone de contato.\n\
6. **`email`** — e-mail de contato do titular.\n\
7. **`ocupacao_principal_cnae`** — CNAE da ocupação principal (7 dígitos). **NUNCA peça código nem nome exato ao cliente.** Pergunte em linguagem natural o que ele faz, use `buscar_cnae` com a descrição pra encontrar a ocupação que encaixa, e **confirme com o cliente** o nome da ocupação antes de chamar esta tool.\n\
8. **`formas_atuacao`** — pelo menos um código das formas de atuação. **Não peça código nem título literal.** Infira a partir de como o cliente descreveu o trabalho dele (ex: \"vendo pelo Instagram\" → internet; \"tenho loja\" → estabelecimento fixo).\n\
9. **`endereco_comercial`** — objeto com `cep`, `numero`, e `complemento` (opcional). O portal auto-preenche logradouro/bairro/cidade pelo CEP; só passe `logradouro` se o cliente avisar que o CEP é genérico.\n\n\
Dados opcionais:\n\n\
- **`ocupacoes_secundarias_cnaes`** — até 15 CNAEs adicionais, todos da mesma família do principal. A grande maioria dos MEIs tem só UMA atividade — não pergunte proativamente.\n\
- **`endereco_residencial`** — só preencha se for DIFERENTE do comercial.",
        annotations(
            title = "Abrir MEI no Portal do Empreendedor",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true,
        )
    )]
    async fn abrir_empresa(
        &self,
        Parameters(args): Parameters<tools::abrir_empresa::Args>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let client_id = extract_and_ensure_client_id(&self.state.pool, &ctx.meta).await?;
        if let Some(err) = require_enabled(&self.state, "abrir_empresa", client_id).await {
            return Ok(err);
        }
        let value = tools::abrir_empresa::run(&self.state, client_id, args).await;
        Ok(CallToolResult::structured(value))
    }
}

#[tool_handler]
impl ServerHandler for ZainMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let all_tools = Self::tool_router().list_all();
        // Sem `_meta.client_id` o caller fica em modo "anônimo": lista
        // completa pra inspeção. Caller real sempre manda o client_id.
        let Some(client_id) = extract_client_id_opt(&context.meta) else {
            return Ok(ListToolsResult {
                tools: all_tools,
                next_cursor: None,
                meta: None,
            });
        };
        let snapshot = match client_state::load_snapshot(&self.state.pool, client_id).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                // Cliente desconhecido: trata como cliente novo "vazio"
                // (snapshot tudo `false`/`None`) e filtra pelos mesmos
                // predicados. Equivalente a um lead que acabou de
                // chegar — sobram só as tools que não dependem de
                // estado.
                ClientSnapshot::default()
            }
            Err(e) => {
                tracing::warn!(%client_id, error = %crate::errlog::anyhow_chain(&e), "list_tools: falha ao carregar snapshot");
                return Err(ErrorData::internal_error(
                    "Não consegui carregar o estado do cliente agora.".to_string(),
                    None,
                ));
            }
        };
        let filtered: Vec<_> = all_tools
            .into_iter()
            .filter(|t| client_state::tool_enabled(&t.name, &snapshot))
            .collect();
        Ok(ListToolsResult {
            tools: filtered,
            next_cursor: None,
            meta: None,
        })
    }
}
