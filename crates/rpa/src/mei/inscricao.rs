//! RPA para inscrição de MEI (Microempreendedor Individual) no Portal do
//! Empreendedor.
//!
//! Depende de uma sessão autenticada no gov.br (ver [`crate::govbr`]). O
//! formulário é Angular — por isso muita coisa é feita via JS via
//! [`PageSession::eval_value`], já que interagir com `<br-select>` por
//! clique puro é instável.
//!
//! Fluxo resumido:
//! 1. Restaurar sessão gov.br + navegar para a página de inscrição.
//! 2. Preencher identificação, atividades, endereços e declarações.
//! 3. Submeter o formulário → modal de conferência → confirmar.
//! 4. Capturar o CNPJ gerado.

use std::time::Duration;

use chromium_driver::PageSession;
use chromium_driver::dom::Dom;
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::certificado::{CertificadoMei, consultar_certificado};
use crate::govbr::launch;
use crate::govbr::session::{self, SavedSession};

#[derive(Debug, Error)]
pub enum InscricaoMeiError {
    /// A sessão gov.br passada expirou/foi revogada: ao clicar em
    /// "Entrar com gov.br" o portal redirecionou para o SSO em vez de
    /// abrir o form. O caller deve revalidar a sessão (rodar
    /// [`crate::govbr::check_govbr_profile`]) e tentar de novo.
    #[error("sessão gov.br inválida ou expirada")]
    SessaoInvalida,
    /// O portal recusou o acesso à inscrição com uma mensagem de erro
    /// terminal (ex.: "Consta impedimento para sua inscrição como MEI
    /// decorrente de vínculo do seu CPF com um CNPJ."). O texto exato
    /// vem do banner `.br-message.danger` e é repassado para o caller
    /// poder relatar ao cliente sem adivinhar.
    #[error("{0}")]
    Impedimento(String),
    /// O CNAE informado não está na tabela `mei_cnaes.ocupacoes` — ou
    /// seja, não é uma atividade permitida para MEI.
    #[error("CNAE {0} não está mapeado para nenhuma ocupação MEI")]
    CnaeNaoMapeado(String),
    /// A ocupação (casada pelo nome) não aparece no `<br-select>` ou
    /// no picklist do portal. Indica que a tabela local está fora de
    /// sincronia com o que a Receita aceita hoje, ou que a família do
    /// CNAE não bate com o regime selecionado.
    #[error("ocupação (CNAE {0}) não está disponível no portal")]
    OcupacaoIndisponivel(String),
    /// Os CNAEs informados pertencem a famílias diferentes — o portal
    /// exige que o principal e todos os secundários compartilhem a mesma
    /// família (determina o radio de regime de tributação).
    #[error("CNAEs de famílias diferentes não podem ser combinados: {0:?}")]
    CnaesFamiliasMistas(Vec<(String, char)>),
    /// A família do CNAE não é reconhecida pelo portal. Hoje só existem
    /// `A` (Geral) e `B` (MEI Caminhoneiro).
    #[error("família de CNAE desconhecida: {0}")]
    FamiliaDesconhecida(char),
    /// O código de forma de atuação passado não existe em
    /// `mei_cnaes.formas_atuacao`.
    #[error("forma de atuação código {0} não está mapeada")]
    FormaAtuacaoDesconhecida(i32),
    /// O CEP foi rejeitado pelo portal (a mensagem "CEP não encontrado"
    /// apareceu após o blur). Pode ser CEP digitado errado ou que saiu
    /// da base dos Correios.
    #[error("CEP {0} rejeitado pelo portal (\"CEP não encontrado\")")]
    CepInvalido(String),
    /// O checkbox correspondente à forma de atuação não existe no DOM
    /// do portal. Indica que a tabela local está fora de sincronia com
    /// o que a Receita renderiza hoje.
    #[error("forma de atuação {titulo:?} (código {codigo}) não está disponível no portal")]
    FormaAtuacaoIndisponivel { codigo: i32, titulo: String },
    #[error(transparent)]
    Cdp(#[from] chromium_driver::CdpError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

const INSCRICAO_URL: &str = "https://mei.receita.economia.gov.br/inscricao/login";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// ─── Tipos públicos ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InscricaoMei {
    // ─── Identificação ─────────────────────────────────────────
    pub rg_identidade: String,
    pub rg_orgao_emissor: String,
    /// Sigla UF com 2 letras (ex.: "BA").
    pub rg_uf_emissor: String,
    /// DDD (2 dígitos).
    pub telefone_ddd: String,
    /// Número do telefone fixo, 8 dígitos (com ou sem hífen).
    pub telefone_numero: String,
    // Nota: os campos de celular (`dddCelular`/`numeroCelular`) são
    // `disabled` no form — o portal pré-preenche pelo perfil gov.br e não
    // permite edição. Por isso não são expostos nos params.
    pub email: String,
    // Nota: capital social é sempre preenchido com R$ 1,00. O portal
    // aceita esse valor mínimo e raramente faz sentido expor isso como
    // parâmetro.

    // ─── Atividades ────────────────────────────────────────────
    /// CNAE (7 dígitos, com ou sem pontuação) da ocupação principal.
    /// Resolvido em runtime para o código numérico interno do portal
    /// via `mei_cnaes.ocupacoes`. A família do CNAE (`A` = Geral,
    /// `B` = MEI Caminhoneiro) determina o regime de tributação — o
    /// principal e todos os secundários precisam ser da mesma família.
    pub ocupacao_principal_cnae: String,
    /// CNAEs das ocupações secundárias (máximo 15).
    pub ocupacoes_secundarias_cnaes: Vec<String>,
    /// Códigos das formas de atuação — pelo menos um obrigatório.
    /// Resolvidos em runtime para o título via `mei_cnaes.formas_atuacao`
    /// e casados no HTML pelo texto do `<label>`.
    pub formas_atuacao: Vec<i32>,

    // ─── Endereço ──────────────────────────────────────────────
    pub endereco_comercial: Endereco,
    /// Se `None`, usa o mesmo endereço comercial (checkbox).
    pub endereco_residencial: Option<Endereco>,
    // Nota: todas as declarações (incluindo a opcional "dados
    // empresariais são públicos") são sempre marcadas — não há motivo
    // pra expor como parâmetro.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endereco {
    /// CEP com 8 dígitos (com ou sem hífen).
    pub cep: String,
    pub numero: String,
    /// Complemento livre (ex.: "ANDAR 2"). O formulário oferece uma modal
    /// estruturada (tipo+valor), mas preenchemos direto o campo de texto —
    /// mais simples e aceito pelo portal.
    pub complemento: Option<String>,
    /// Logradouro (só o nome, sem o tipo). Usado quando o CEP é genérico
    /// (de cidade inteira) e o portal não auto-preenche. Se `None` e o
    /// portal pedir manualmente, falha com [`InscricaoMeiError`].
    ///
    /// Nota: `tipoLogradouro`, `bairro`, `municipio` e `uf` ainda não
    /// são expostos — quando o CEP é genérico o formulário fica inválido
    /// até que essas duas sejam adicionadas. Por ora só cobrimos o
    /// caminho com CEP auto-preenchido (mais comum).
    pub logradouro: Option<String>,
}

/// Resultado da checagem de elegibilidade para abertura de MEI.
/// Quando `pode_abrir == false`, `motivo` carrega a mensagem exata do
/// banner de impedimento do portal (ex: "CPF já vinculado a outro
/// CNPJ"), pra o caller poder repassar ao cliente sem adivinhar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElegibilidadeMei {
    pub pode_abrir: bool,
    pub motivo: Option<String>,
}

/// Resultado de uma inscrição MEI bem-sucedida: o CNPJ gerado + o
/// CCMEI consultado logo na sequência. A consulta é `best effort` —
/// se o portal do CCMEI estiver instável, `ccmei` fica `None` e o
/// caller lida com isso (tipicamente tentando de novo depois). A
/// abertura em si foi confirmada pelo CNPJ, então isso não é erro.
#[derive(Debug, Clone)]
pub struct InscricaoMeiOutcome {
    pub cnpj: String,
    pub ccmei: Option<CertificadoMei>,
}

// ─── Entry point ──────────────────────────────────────────────────────────

/// Executa a inscrição de MEI no Portal do Empreendedor e, na sequência,
/// consulta o CCMEI pelo CNPJ recém-gerado pra devolver o certificado
/// completo.
///
/// Requer uma [`SavedSession`] válida do gov.br — tipicamente obtida via
/// [`crate::govbr::check_govbr_profile`]. O CPF do titular vem implícito
/// na sessão; os dados pessoais já estão pré-preenchidos pelo portal.
///
/// - `pool` é usado pra resolver CNAEs → códigos numéricos via
///   `mei_cnaes.ocupacoes` antes de abrir o browser.
/// - `ai` + `ai_model` são usados quando o CEP é genérico e o portal
///   pede o `tipoLogradouro` manualmente — nesse caso consultamos o LLM
///   com a lista de opções e deixamos ele escolher uma.
pub async fn inscrever_mei(
    pool: &Pool,
    ai: &ai::Client,
    ai_model: &str,
    saved: &SavedSession,
    params: InscricaoMei,
) -> Result<InscricaoMeiOutcome, InscricaoMeiError> {
    // Sanity checks — pegamos aqui pra falhar rápido antes de subir o browser.
    if params.formas_atuacao.is_empty() {
        return Err(anyhow::anyhow!(
            "formas_atuacao não pode ser vazio (pelo menos uma é obrigatória)"
        )
        .into());
    }
    if params.ocupacoes_secundarias_cnaes.len() > 15 {
        return Err(anyhow::anyhow!("máximo 15 ocupações secundárias").into());
    }

    // Resolve CNAEs → códigos do portal. Se qualquer um falhar, volta
    // sem ter subido o Chromium.
    let principal = resolver_cnae(pool, &params.ocupacao_principal_cnae).await?;
    let familia = principal.familia;
    let mut mistura: Vec<(String, char)> = Vec::new();
    if familia != 'A' && familia != 'B' {
        return Err(InscricaoMeiError::FamiliaDesconhecida(familia));
    }
    let mut secundarios: Vec<OcupacaoSecundaria> =
        Vec::with_capacity(params.ocupacoes_secundarias_cnaes.len());
    for cnae in &params.ocupacoes_secundarias_cnaes {
        let r = resolver_cnae(pool, cnae).await?;
        if r.familia != familia {
            mistura.push((cnae.clone(), r.familia));
        }
        secundarios.push(OcupacaoSecundaria {
            cnae: cnae.clone(),
            nome: r.nome,
        });
    }
    if !mistura.is_empty() {
        // Inclui o principal pra facilitar o debug do lado do caller.
        mistura.insert(0, (params.ocupacao_principal_cnae.clone(), familia));
        return Err(InscricaoMeiError::CnaesFamiliasMistas(mistura));
    }
    // Resolve formas de atuação → (codigo, titulo). Falha cedo se
    // qualquer código não estiver na tabela.
    let mut formas_atuacao = Vec::with_capacity(params.formas_atuacao.len());
    for codigo in &params.formas_atuacao {
        let titulo = resolver_forma_atuacao(pool, *codigo).await?;
        formas_atuacao.push((*codigo, titulo));
    }

    let atividades = AtividadesResolvidas {
        familia,
        principal_cnae: params.ocupacao_principal_cnae.clone(),
        principal_nome: principal.nome,
        secundarias: secundarios,
        formas_atuacao,
    };

    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result: Result<String, InscricaoMeiError> = async {
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;

        session::restore(&browser, &page, saved).await?;

        page.navigate(INSCRICAO_URL).await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();

        // Tela inicial: card "Login" com dois botões. Clicar em
        // "Entrar com gov.br" dispara o SSO; com a sessão restaurada, o
        // gov.br devolve direto pra tela do form de inscrição. Sem sessão
        // válida, cairíamos de volta na página de CPF do SSO — detectamos
        // isso abaixo e falhamos rápido.
        let dom = page.dom().await?;
        // Aguarda o botão aparecer pelo DOM cache, mas clica via JS —
        // manter o handle do nodeId entre wait_for e click é frágil
        // durante a hidratação Angular do portal (CDP devolve "Could
        // not find node with given id" quando o nó é re-renderizado
        // antes do click).
        dom.wait_for(
            "app-login-inscricao button.br-button[primary]",
            DEFAULT_TIMEOUT,
        )
        .await?;
        click_js(&page, "app-login-inscricao button.br-button[primary]").await?;

        wait_for_inscrever_form(&page, dom).await?;

        fill_identificacao(&page, &params).await?;
        fill_atividades(&page, &atividades).await?;
        fill_endereco(
            &page,
            ai,
            ai_model,
            &params.endereco_comercial,
            "mei-endereco[name=enderecoComercial]",
        )
        .await?;
        fill_residencial(
            &page,
            dom,
            ai,
            ai_model,
            params.endereco_residencial.as_ref(),
        )
        .await?;
        marcar_declaracoes(&page).await?;

        // Clica "Continuar" para submeter o form. O botão fica no
        // `.button-row` ao lado do "Cancelar" (que é um `<a>` disfarçado
        // de botão) — casamos pelo `type=submit` pra desambiguar.
        tracing::info!("clicando em Continuar para submeter a inscrição");
        click_js(&page, ".button-row button[type=submit].br-button.primary").await?;

        // Aguarda a modal "Tela para conferência de dados" abrir
        // (`#modalConferenciaDados` ganha `.show`) e clica em "Confirmar".
        // A modal tem dois primary buttons — o "Corrigir" dentro de
        // `.quadro-conferencia-botoes` e o "Confirmar" no
        // `.br-modal-footer` — então escopamos pelo id do modal + footer.
        tracing::info!("aguardando modal de conferência abrir");
        wait_for_modal_open(&page, "modalConferenciaDados").await?;
        // Pequeno settle pro Angular terminar de popular os `<p>` de
        // conferência antes de clicar — evita confirmar enquanto os
        // dados ainda estão sendo renderizados.
        tokio::time::sleep(Duration::from_millis(500)).await;
        tracing::info!("clicando em Confirmar na modal de conferência");
        click_js(
            &page,
            "#modalConferenciaDados .br-modal-footer button.br-button.primary",
        )
        .await?;

        // Após confirmar, o portal redireciona para /inscricao/conclusao
        // e abre uma modal de aviso ("A INSCRIÇÃO DO MEI É GRATUITA!")
        // com um único botão "Ciente". A modal não tem id, então
        // localizamos pelo texto do botão dentro de um `.br-modal-footer`
        // visível — `offsetParent` descarta modais escondidas no DOM.
        tracing::info!("aguardando modal de aviso 'Ciente' na tela de conclusão");
        let ciente_js = r#"
            (() => {
                const footers = document.querySelectorAll('.br-modal-footer');
                for (const f of footers) {
                    if (f.offsetParent === null) continue;
                    const btns = f.querySelectorAll('button.br-button.primary');
                    for (const b of btns) {
                        if ((b.textContent || '').trim() === 'Ciente') {
                            b.click();
                            return 'ok';
                        }
                    }
                }
                return 'not_found';
            })()
        "#;
        let ciente_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let v = page.eval_value(ciente_js).await?;
            match v.as_str().unwrap_or("") {
                "ok" => break,
                "not_found" => {}
                other => {
                    return Err(anyhow::anyhow!("click Ciente conclusão: {other}").into());
                }
            }
            if tokio::time::Instant::now() >= ciente_deadline {
                let _ = page.debug_dump("mei-inscricao-ciente-timeout").await;
                return Err(anyhow::anyhow!(
                    "timeout aguardando botão 'Ciente' na tela de conclusão"
                )
                .into());
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        // Extrai o CNPJ da página de conclusão. O texto é do tipo
        // "Sua inscrição ... o seu número do CNPJ é <strong>XX.XXX.XXX/XXXX-XX</strong>."
        // — casamos pelo `<p class="conclusao">` que contém um `<strong>`
        // cujo texto normalizado tem 14 dígitos.
        tracing::info!("extraindo CNPJ da tela de conclusão");
        let cnpj_js = r#"
            (() => {
                const ps = document.querySelectorAll('p.conclusao');
                for (const p of ps) {
                    const strong = p.querySelector('strong');
                    if (!strong) continue;
                    const digits = (strong.textContent || '').replace(/\D/g, '');
                    if (digits.length === 14) return digits;
                }
                return null;
            })()
        "#;
        let cnpj_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let cnpj = loop {
            let v = page.eval_value(cnpj_js).await?;
            if let Some(cnpj) = v.as_str() {
                break cnpj.to_string();
            }
            if tokio::time::Instant::now() >= cnpj_deadline {
                let _ = page.debug_dump("mei-inscricao-cnpj-timeout").await;
                return Err(anyhow::anyhow!(
                    "timeout aguardando CNPJ aparecer na tela de conclusão"
                )
                .into());
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        };
        tracing::info!(%cnpj, "inscrição MEI concluída");

        Ok(cnpj)
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    let cnpj = result?;

    // Após o browser da inscrição estar fechado, sobe um novo pra
    // consultar o CCMEI pelo CNPJ recém-gerado. Best effort: se o
    // portal do certificado estiver instável ou o CNPJ ainda não
    // estiver indexado, guardamos `None` e deixamos o caller tentar
    // de novo — a abertura em si já deu certo.
    tracing::info!(%cnpj, "consultando CCMEI após abertura");
    let ccmei = match consultar_certificado(&cnpj).await {
        Ok(Some(cert)) => Some(cert),
        Ok(None) => {
            tracing::warn!(
                %cnpj,
                "CCMEI retornou vazio logo após inscrição — CNPJ pode ainda não estar indexado"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                %cnpj,
                error = %e,
                "falha ao consultar CCMEI após inscrição"
            );
            None
        }
    };

    Ok(InscricaoMeiOutcome { cnpj, ccmei })
}

/// Checa se um CPF tem direito a abrir MEI. Abre o form de inscrição
/// com uma sessão gov.br previamente validada e observa o que acontece:
///
/// - Formulário aparece (`app-inscrever input[name=identidade]`) →
///   `pode_abrir: true`.
/// - Portal exibe banner `.br-message.danger` com impedimento (ex: CPF
///   já vinculado a outro CNPJ) → `pode_abrir: false` + `motivo` com o
///   texto exato do banner.
/// - Sessão gov.br inválida → [`InscricaoMeiError::SessaoInvalida`], o
///   caller deve revalidar antes de tentar de novo.
///
/// Reusa a mesma navegação + [`wait_for_inscrever_form`] do fluxo de
/// inscrição — por isso os consentimentos OAuth/LGPD eventuais também
/// são aceitos aqui, sem efeitos colaterais (o formulário só abre e
/// depois fechamos o browser sem submeter nada).
pub async fn checar_pode_abrir_mei(
    saved: &SavedSession,
) -> Result<ElegibilidadeMei, InscricaoMeiError> {
    let opts = launch::options_with_extensions().await?;
    let (mut process, browser) = chromium_driver::launch(opts).await?;

    let result: Result<ElegibilidadeMei, InscricaoMeiError> = async {
        let page = browser.create_page("about:blank").await?.attach().await?;
        page.enable().await?;

        session::restore(&browser, &page, saved).await?;

        page.navigate(INSCRICAO_URL).await?;
        page.wait_for_load(DEFAULT_TIMEOUT).await.ok();

        let dom = page.dom().await?;
        dom.wait_for(
            "app-login-inscricao button.br-button[primary]",
            DEFAULT_TIMEOUT,
        )
        .await?;
        click_js(&page, "app-login-inscricao button.br-button[primary]").await?;

        // [`wait_for_inscrever_form`] retorna `Impedimento(msg)` quando
        // o banner de erro terminal aparece — pra nós isso é sucesso da
        // checagem (sabemos a resposta). Converte em `pode_abrir: false`
        // com o motivo. Qualquer outro erro (Sessão inválida, CDP, ...)
        // é propagado.
        match wait_for_inscrever_form(&page, dom).await {
            Ok(()) => Ok(ElegibilidadeMei {
                pode_abrir: true,
                motivo: None,
            }),
            Err(InscricaoMeiError::Impedimento(msg)) => Ok(ElegibilidadeMei {
                pode_abrir: false,
                motivo: Some(msg),
            }),
            Err(e) => Err(e),
        }
    }
    .await;

    let _ = browser.close().await;
    let _ = process.wait().await;
    result
}

// ─── Seções do formulário ─────────────────────────────────────────────────

async fn fill_identificacao(page: &PageSession, p: &InscricaoMei) -> anyhow::Result<()> {
    fill_text(page, "input[name=identidade]", &p.rg_identidade).await?;
    fill_text(page, "input[name=orgaoEmissor]", &p.rg_orgao_emissor).await?;
    select_br_select_by_value(page, "ufEmissor", &p.rg_uf_emissor).await?;

    // DDD e telefone usam ngx-mask, que não reconhece o valor setado
    // diretamente via `HTMLInputElement.value` setter + dispatchEvent —
    // a máscara precisa ver keystrokes reais para aplicar a formatação
    // e sincronizar o FormControl. Usamos teclas reais via CDP.
    type_text_human(
        page,
        "input[name=dddContato]",
        &digits_only(&p.telefone_ddd),
    )
    .await?;
    // O campo aceita só 8 dígitos. Se o cliente passou um celular de
    // 9 dígitos (com o "9" na frente), descartamos os excedentes pegando
    // os últimos 8 — o portal valida pelo tamanho e rejeita mais que isso.
    let numero_contato = last_n_digits(&p.telefone_numero, 8);
    type_text_human(page, "input[name=numeroContato]", &numero_contato).await?;
    fill_text(page, "input[name=email]", &p.email).await?;
    // Capital social fixo em R$ 1,00 — valor mínimo aceito pelo portal.
    fill_text(page, "input[name=capitalSocial]", "1,00").await?;
    Ok(())
}

async fn fill_atividades(
    page: &PageSession,
    atividades: &AtividadesResolvidas,
) -> Result<(), InscricaoMeiError> {
    // Regime tributação deriva da família do CNAE. A = Geral (radio 0),
    // B = Transportador Autônomo de Cargas (radio 1). Outras famílias
    // são rejeitadas antes de chegar aqui por [`FamiliaDesconhecida`].
    let regime_id = match atividades.familia {
        'A' => "familia-ocupacao-0",
        'B' => "familia-ocupacao-1",
        other => return Err(InscricaoMeiError::FamiliaDesconhecida(other)),
    };
    click_js(page, &format!("#{regime_id}")).await?;

    select_ocupacao(
        page,
        "ocupacaoPrincipal",
        &atividades.principal_cnae,
        &atividades.principal_nome,
    )
    .await?;

    // Ocupações secundárias: picklist com `<select name="selecionadosAdicao">`
    // à esquerda, `<select name="atividadesSecundarias">` à direita e
    // botões "Inserir"/"Remover" no meio. O `value` dos `<option>` é um
    // índice Angular opaco (ex.: `"467: Object"`), então casamos pelo
    // texto do option — que é `mei_cnaes.ocupacoes.nome`.
    for oc in &atividades.secundarias {
        add_ocupacao_secundaria(page, oc).await?;
    }

    // Formas de atuação (checkboxes) — casados pelo texto exato do
    // label (com trim) dentro de `[ngmodelgroup=formasDeAtuacao]`. O
    // título vem de `mei_cnaes.formas_atuacao.titulo` (fonte canônica).
    for (codigo, titulo) in &atividades.formas_atuacao {
        click_forma_atuacao(page, *codigo, titulo).await?;
    }
    Ok(())
}

async fn fill_endereco(
    page: &PageSession,
    ai: &ai::Client,
    ai_model: &str,
    end: &Endereco,
    scope: &str,
) -> Result<(), InscricaoMeiError> {
    // Os inputs dentro de `mei-endereco` usam `name="cep"`, `name="numero"`,
    // etc. — sem qualificador. Quando existem dois `mei-endereco` no DOM
    // (comercial + residencial), o `scope` (ex.:
    // `mei-endereco[name=enderecoComercial]`) desambigua qual conjunto
    // preenchemos.
    let cep = digits_only(&end.cep);
    fill_text(page, &format!("{scope} input[name=cep]"), &cep).await?;

    // Dispara blur no input de CEP pra Angular iniciar o lookup (ViaCEP
    // ou similar). O atributo `cepvalidoonblur` no HTML sugere que o
    // handler é no blur.
    let scope_js = serde_json::to_string(scope).map_err(anyhow::Error::from)?;
    let blur_js = format!(
        r#"(() => {{
            const el = document.querySelector({scope_js} + ' input[name=cep]');
            if (el) {{
                el.blur();
                el.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            }}
            return 'ok';
        }})()"#
    );
    page.eval_value(&blur_js).await?;

    // Aguarda o lookup do CEP terminar: o loader interno do
    // `mei-endereco` ganha `.is-loading` enquanto busca e perde quando
    // termina.
    wait_cep_lookup(page, scope).await?;

    // Após o lookup o portal pode estar em 3 estados:
    // 1. CEP rejeitado → aparece `<mensagem-campo-invalido>` como irmão
    //    do input[name=cep] dentro do mesmo `.form-group`.
    // 2. CEP válido com logradouro conhecido → `input[name=logradouro]`
    //    já veio populado pelo portal.
    // 3. CEP válido sem logradouro (CEPs genéricos de cidade) →
    //    logradouro vazio e o usuário precisa preencher.
    let cep_status = page
        .eval_value(&format!(
            r#"(() => {{
                const cep = document.querySelector({scope_js} + ' input[name=cep]');
                if (!cep) return {{ state: 'cep_missing' }};
                const group = cep.closest('.form-group');
                const erro = group && group.querySelector('mensagem-campo-invalido');
                if (erro) return {{ state: 'invalid' }};
                const lograd = document.querySelector({scope_js} + ' input[name=logradouro]');
                const valor = lograd ? (lograd.value || '').trim() : '';
                return {{ state: valor ? 'autofilled' : 'empty_logradouro' }};
            }})()"#
        ))
        .await?;
    let state = cep_status
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("cep_missing");
    match state {
        "invalid" => {
            return Err(InscricaoMeiError::CepInvalido(end.cep.clone()));
        }
        "empty_logradouro" => {
            let Some(lograd) = end.logradouro.as_deref() else {
                return Err(anyhow::anyhow!(
                    "CEP {} é genérico (sem logradouro na base dos Correios) e nenhum `logradouro` foi informado",
                    end.cep
                )
                .into());
            };
            // `tipoLogradouro` vira obrigatório nesse cenário. Consulta
            // o LLM com a lista de opções do `<br-select>` e deixa ele
            // escolher uma compatível com o logradouro informado.
            let tipo = pick_tipo_logradouro(page, ai, ai_model, scope, end, lograd).await?;
            select_br_select_by_value(page, "tipoLogradouro", &tipo).await?;
            fill_text(page, &format!("{scope} input[name=logradouro]"), lograd).await?;
        }
        "autofilled" => {}
        other => {
            return Err(anyhow::anyhow!("estado inesperado do CEP ({end:?}): {other}").into());
        }
    }

    fill_text(page, &format!("{scope} input[name=numero]"), &end.numero).await?;
    if let Some(comp) = &end.complemento {
        fill_text(page, &format!("{scope} input[name=complemento]"), comp).await?;
    }
    Ok(())
}

async fn fill_residencial(
    page: &PageSession,
    dom: &Dom,
    ai: &ai::Client,
    ai_model: &str,
    residencial: Option<&Endereco>,
) -> Result<(), InscricaoMeiError> {
    match residencial {
        None => {
            // Checkbox "Endereço residencial igual ao comercial" vem
            // marcado por default — só garantimos que está assim.
            page.eval_value(
                r#"(() => {
                    const cb = document.querySelector('#endereco-residencial-igual-comercial-check');
                    if (cb && !cb.checked) cb.click();
                    return 'ok';
                })()"#,
            )
            .await?;
            Ok(())
        }
        Some(end) => {
            // Desmarca o checkbox pra o form renderizar o segundo
            // `mei-endereco[name=enderecoResidencial]`.
            page.eval_value(
                r#"(() => {
                    const cb = document.querySelector('#endereco-residencial-igual-comercial-check');
                    if (cb && cb.checked) cb.click();
                    return 'ok';
                })()"#,
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
            dom.invalidate();
            fill_endereco(
                page,
                ai,
                ai_model,
                end,
                "mei-endereco[name=enderecoResidencial]",
            )
            .await?;
            Ok(())
        }
    }
}

async fn marcar_declaracoes(page: &PageSession) -> anyhow::Result<()> {
    // As 6 declarações obrigatórias + a opcional "dados públicos"
    // (sempre aceita).
    for id in [
        "declaracaoDesimpedimento",
        "declaracaoOpcaoSimples",
        "declaracaoEnquadramentoME",
        "declaracaoCienciaDispensa",
        "autorizacaoFiscalizacao",
        "declaracaoCienciaDispensaRevogavel",
        "declaracaoCienciaDadosPublicos",
    ] {
        click_checkbox(page, id).await?;
    }
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Últimos `n` dígitos de `s` (ignora não-dígitos). Se o input tiver
/// menos que `n` dígitos, devolve todos.
fn last_n_digits(s: &str, n: usize) -> String {
    let all = digits_only(s);
    if all.len() <= n {
        all
    } else {
        all[all.len() - n..].to_string()
    }
}

/// Dispara `element.click()` via JS em vez de mouse event do CDP. O
/// click do CDP depende de um `nodeId` obtido antes e despacha mouse
/// events em coordenadas de tela — se o Angular re-renderiza o nó
/// entre o match e o click, o CDP devolve "Could not find node with
/// given id". `el.click()` via JS pega o elemento na hora e dispara
/// o evento `click` que os listeners Angular escutam.
async fn click_js(page: &PageSession, selector: &str) -> anyhow::Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.click();
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => Ok(()),
        "not_found" => anyhow::bail!("elemento não encontrado: {selector}"),
        other => anyhow::bail!("click_js({selector}): {other}"),
    }
}

/// Preenche um `<input>` via JS em vez de CDP click+type. Usar clique
/// real do CDP depende de scroll/viewport (o mouse event é despachado em
/// coordenadas de tela) — para um form longo como o do MEI, muitos
/// campos ficam abaixo da dobra e o clique cai fora do input. A
/// abordagem JS independe de viewport e, usando o setter nativo de
/// `HTMLInputElement.value`, funciona com ngx-mask/Angular reactive
/// forms (que interceptam o evento `input` pra revalidar e reformatar).
async fn fill_text(page: &PageSession, selector: &str, value: &str) -> anyhow::Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let value_js = serde_json::to_string(value)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            ).set;
            el.focus();
            setter.call(el, {value_js});
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            el.blur();
            el.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => Ok(()),
        "not_found" => anyhow::bail!("elemento não encontrado: {selector}"),
        other => anyhow::bail!("fill_text({selector}): {other}"),
    }
}

/// Preenche um `<input>` simulando digitação real via CDP
/// (`Input.dispatchKeyEvent`). Necessário para campos com ngx-mask ou
/// máscaras custom, onde o setter nativo + `dispatchEvent('input')` não
/// aciona o handler da máscara — o campo fica visualmente preenchido
/// mas o FormControl do Angular não enxerga o valor, e o portal rejeita
/// como vazio. Focar via JS é estável; o dispatch de tecla vai para o
/// elemento focado, independente de coordenadas/scroll.
async fn type_text_human(page: &PageSession, selector: &str, value: &str) -> anyhow::Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let focus_js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.focus();
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype, 'value'
            ).set;
            setter.call(el, '');
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&focus_js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => {}
        "not_found" => anyhow::bail!("elemento não encontrado: {selector}"),
        other => anyhow::bail!("type_text_human focus({selector}): {other}"),
    }

    let dom = page.dom().await?;
    let element = dom.query_selector(selector).await?;
    element.type_text(value).await?;

    let blur_js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return 'not_found';
            el.blur();
            el.dispatchEvent(new Event('blur', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    page.eval_value(&blur_js).await?;
    Ok(())
}

async fn click_checkbox(page: &PageSession, id: &str) -> anyhow::Result<()> {
    // Os checkboxes do design system gov.br têm o `<input>` visualmente
    // escondido (`opacity: 0`) — o click do CDP no centro do elemento
    // não registra. `input.click()` via JS dispara o toggle + change
    // listener do Angular corretamente. Idempotente: não re-clica se
    // já estiver marcado.
    let id_js = serde_json::to_string(id)?;
    let js = format!(
        r#"(() => {{
            const cb = document.getElementById({id_js});
            if (!cb) return 'not_found';
            if (!cb.checked) cb.click();
            return cb.checked === true ? 'ok' : 'not_checked';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    match v.as_str().unwrap_or("") {
        "ok" => Ok(()),
        "not_found" => anyhow::bail!("checkbox #{id} não encontrado"),
        "not_checked" => anyhow::bail!("não consegui marcar o checkbox #{id}"),
        other => anyhow::bail!("click_checkbox(#{id}): {other}"),
    }
}

/// Marca um checkbox de forma de atuação casando pelo texto exato do
/// `<label>` (só aplicamos `trim()` no texto do HTML pra remover os
/// espaços que o Angular deixa ao redor — nenhuma outra normalização).
/// Idempotente: se o checkbox já estiver marcado, não reclica.
///
/// O `titulo` deve vir de `mei_cnaes.formas_atuacao.titulo`, que é a
/// fonte canônica — qualquer divergência com o HTML é sinal de
/// dessincronização e vira [`InscricaoMeiError::FormaAtuacaoIndisponivel`].
async fn click_forma_atuacao(
    page: &PageSession,
    codigo: i32,
    titulo: &str,
) -> Result<(), InscricaoMeiError> {
    let titulo_js = serde_json::to_string(titulo).map_err(anyhow::Error::from)?;
    let js = format!(
        r#"(() => {{
            const wanted = {titulo_js};
            const container = document.querySelector('[ngmodelgroup=formasDeAtuacao]');
            if (!container) return 'container_missing';
            const labels = container.querySelectorAll('label');
            for (const lab of labels) {{
                if ((lab.textContent || "").trim() !== wanted) continue;
                const id = lab.getAttribute('for');
                const input = id ? container.querySelector('#' + CSS.escape(id)) : null;
                if (!input) return 'input_missing';
                if (!input.checked) input.click();
                return 'ok';
            }}
            return 'not_found';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    let msg = v.as_str().unwrap_or("");
    match msg {
        "ok" => Ok(()),
        "not_found" => Err(InscricaoMeiError::FormaAtuacaoIndisponivel {
            codigo,
            titulo: titulo.to_string(),
        }),
        "container_missing" => {
            Err(anyhow::anyhow!("grupo formasDeAtuacao não encontrado no DOM").into())
        }
        "input_missing" => Err(anyhow::anyhow!(
            "checkbox correspondente ao label {titulo:?} não encontrado"
        )
        .into()),
        other => Err(anyhow::anyhow!("click_forma_atuacao({titulo:?}): {other}").into()),
    }
}

/// Consulta o LLM para escolher o `tipoLogradouro` quando o CEP é
/// genérico (sem logradouro na base dos Correios). Lê todas as opções
/// do `<br-select name="tipoLogradouro">`, monta um prompt com o
/// endereço e a lista de opções, e pede ao modelo um JSON estruturado
/// contendo exatamente um dos `value` disponíveis. Se o modelo devolver
/// algo fora da lista, tenta de novo (até 3 vezes).
async fn pick_tipo_logradouro(
    page: &PageSession,
    ai: &ai::Client,
    ai_model: &str,
    scope: &str,
    end: &Endereco,
    logradouro: &str,
) -> Result<String, InscricaoMeiError> {
    const MAX_TENTATIVAS: usize = 5;

    #[derive(serde::Deserialize, schemars::JsonSchema)]
    struct Escolha {
        /// Label escolhido — precisa bater exatamente com um dos itens
        /// da lista fornecida.
        tipo: String,
    }

    // Scrape opções (value + label). O LLM só vê os labels; o value é
    // usado na volta pra selecionar o radio correto.
    let scope_js = serde_json::to_string(scope).map_err(anyhow::Error::from)?;
    let js = format!(
        r#"(() => {{
            const root = document.querySelector({scope_js});
            if (!root) return [];
            const radios = root.querySelectorAll(
                'br-select[name="tipoLogradouro"] input[type=radio]'
            );
            const out = [];
            for (const r of radios) {{
                const lab = root.querySelector('label[for="' + r.id + '"]');
                out.push({{
                    value: r.value,
                    label: (lab ? (lab.textContent || '') : '').trim(),
                }});
            }}
            return out;
        }})()"#
    );
    let raw = page.eval_value(&js).await?;
    let opcoes: Vec<(String, String)> = raw
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let value = item.get("value")?.as_str()?.to_string();
                    let label = item.get("label")?.as_str()?.to_string();
                    Some((value, label))
                })
                .collect()
        })
        .unwrap_or_default();
    if opcoes.is_empty() {
        return Err(anyhow::anyhow!(
            "nenhuma opção encontrada em br-select[name=tipoLogradouro] para escolher via LLM"
        )
        .into());
    }

    let lista_formatada: String = opcoes
        .iter()
        .map(|(_, l)| format!("- {l}"))
        .collect::<Vec<_>>()
        .join("\n");

    let complemento = end.complemento.as_deref().unwrap_or("(nenhum)");
    let user_prompt = format!(
        "Escolha o tipo de logradouro correspondente ao endereço abaixo. \
         Responda exatamente com um dos itens da lista.\n\
         \n\
         CEP: {cep}\n\
         Logradouro: {logradouro}\n\
         Número: {numero}\n\
         Complemento: {complemento}\n\
         \n\
         Opções:\n{lista_formatada}",
        cep = end.cep,
        numero = end.numero,
    );

    let mut ultimo_erro: Option<String> = None;
    for tentativa in 1..=MAX_TENTATIVAS {
        let mensagens = vec![ai::ChatMessage::InputText {
            text: user_prompt.clone(),
        }];
        let request = ai::StructuredRequest {
            model: ai_model,
            system: "",
            messages: &mensagens,
        };
        let resp = match ai.chat_structured::<Escolha>(request).await {
            Ok(r) => r,
            Err(e) => {
                ultimo_erro = Some(format!("erro na chamada ao LLM: {e:#}"));
                tracing::warn!(tentativa, "{}", ultimo_erro.as_deref().unwrap());
                continue;
            }
        };
        let escolhido = resp.value.tipo.trim();
        if let Some((value, label)) = opcoes.iter().find(|(_, l)| l == escolhido) {
            tracing::info!(
                tentativa,
                cep = %end.cep,
                %value,
                %label,
                "LLM escolheu tipoLogradouro"
            );
            return Ok(value.clone());
        }
        ultimo_erro = Some(format!(
            "LLM devolveu {escolhido:?}, que não está na lista de opções"
        ));
        tracing::warn!(tentativa, "{}", ultimo_erro.as_deref().unwrap());
    }

    Err(anyhow::anyhow!(
        "não foi possível escolher tipoLogradouro após {MAX_TENTATIVAS} tentativas: {}",
        ultimo_erro.unwrap_or_else(|| "sem erro capturado".to_string())
    )
    .into())
}

/// Aguarda o lookup do CEP terminar dentro de um `mei-endereco`. O
/// componente mostra um `div.is-loading` enquanto busca e remove a
/// classe quando termina. Damos um pequeno tempo inicial para a
/// classe aparecer (a busca pode levar alguns ms pra começar) e então
/// pollamos até sumir.
async fn wait_cep_lookup(page: &PageSession, scope: &str) -> Result<(), InscricaoMeiError> {
    const INITIAL_WAIT: Duration = Duration::from_millis(200);
    const POLL_INTERVAL: Duration = Duration::from_millis(150);
    const LOOKUP_TIMEOUT: Duration = Duration::from_secs(15);

    tokio::time::sleep(INITIAL_WAIT).await;

    let scope_js = serde_json::to_string(scope).map_err(anyhow::Error::from)?;
    let js = format!(
        r#"(() => {{
            const root = document.querySelector({scope_js});
            if (!root) return false;
            return root.querySelector('.is-loading') !== null;
        }})()"#
    );
    let deadline = tokio::time::Instant::now() + LOOKUP_TIMEOUT;
    loop {
        let carregando = page.eval_value(&js).await?.as_bool().unwrap_or(false);
        if !carregando {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow::anyhow!("timeout aguardando lookup do CEP terminar").into());
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Resolve um código de forma de atuação → título via
/// `mei_cnaes.formas_atuacao`. Falha com
/// [`InscricaoMeiError::FormaAtuacaoDesconhecida`] se não achar.
async fn resolver_forma_atuacao(pool: &Pool, codigo: i32) -> Result<String, InscricaoMeiError> {
    let row = sql!(
        pool,
        "SELECT titulo
         FROM mei_cnaes.formas_atuacao
         WHERE codigo = $codigo
         LIMIT 1"
    )
    .fetch_optional()
    .await
    .map_err(anyhow::Error::from)?;
    match row {
        Some(r) => Ok(r.titulo),
        None => Err(InscricaoMeiError::FormaAtuacaoDesconhecida(codigo)),
    }
}

enum BrSelectOutcome {
    Ok,
    OptionNotFound,
}

/// Seleciona uma opção dentro de um `<br-select name="...">` pelo valor do
/// radio interno. Dispara um click no label (que o Angular Forms reconhece
/// como change). Usar click puro no componente é instável porque o
/// dropdown abre via JS e os itens são virtualizados.
///
/// Retorna [`BrSelectOutcome::OptionNotFound`] se o value não existir nos
/// radios — o caller decide se isso é erro ou não.
async fn try_select_br_select(
    page: &PageSession,
    name: &str,
    value: &str,
) -> anyhow::Result<BrSelectOutcome> {
    // Usa serde_json pra escapar os argumentos e evitar injeção de aspas.
    let name_js = serde_json::to_string(name)?;
    let value_js = serde_json::to_string(value)?;
    let js = format!(
        r#"(() => {{
            const name = {name_js};
            const value = {value_js};
            const sel = document.querySelector('br-select[name="' + name + '"]');
            if (!sel) return 'br-select não encontrado: ' + name;
            const radio = sel.querySelector('input[type=radio][value="' + value + '"]');
            if (!radio) return 'not_found';
            const label = sel.querySelector('label[for="' + radio.id + '"]');
            if (label) {{
                label.click();
            }} else {{
                radio.click();
            }}
            radio.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    let msg = v.as_str().unwrap_or("");
    let outcome = match msg {
        "ok" => BrSelectOutcome::Ok,
        "not_found" => BrSelectOutcome::OptionNotFound,
        other => anyhow::bail!("try_select_br_select({name}={value}): {other}"),
    };
    // Pequeno settle pra Angular reagir ao change.
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(outcome)
}

async fn select_br_select_by_value(
    page: &PageSession,
    name: &str,
    value: &str,
) -> anyhow::Result<()> {
    match try_select_br_select(page, name, value).await? {
        BrSelectOutcome::Ok => Ok(()),
        BrSelectOutcome::OptionNotFound => {
            anyhow::bail!("opção não encontrada em br-select[name={name}]: {value}")
        }
    }
}

/// Adiciona uma ocupação secundária via picklist: localiza o
/// `<option>` em `select[name=selecionadosAdicao]` cujo texto bate com
/// `oc.nome` (após `trim()`), seleciona e clica "Inserir". Falha com
/// [`InscricaoMeiError::OcupacaoIndisponivel`] se o nome não aparecer —
/// isso sinaliza dessincronização entre a tabela local e a lista do
/// portal (ou escolha de CNAE de família errada).
async fn add_ocupacao_secundaria(
    page: &PageSession,
    oc: &OcupacaoSecundaria,
) -> Result<(), InscricaoMeiError> {
    let nome_js = serde_json::to_string(&oc.nome).map_err(anyhow::Error::from)?;
    let js = format!(
        r#"(() => {{
            const wanted = {nome_js};
            const sel = document.querySelector('select[name=selecionadosAdicao]');
            if (!sel) return 'select_missing';
            let target = null;
            for (const opt of sel.options) {{
                if ((opt.textContent || '').trim() === wanted) {{
                    target = opt;
                    break;
                }}
            }}
            if (!target) return 'not_found';
            for (const opt of sel.options) opt.selected = false;
            target.selected = true;
            sel.dispatchEvent(new Event('change', {{ bubbles: true }}));
            // "Inserir" está no mesmo container `.picklist` — pega o
            // primeiro button cujo texto começa com "Inserir".
            const picklist = sel.closest('.picklist');
            if (!picklist) return 'picklist_missing';
            const btn = Array.from(picklist.querySelectorAll('button'))
                .find(b => (b.textContent || '').trim().startsWith('Inserir'));
            if (!btn) return 'inserir_missing';
            btn.click();
            return 'ok';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    let msg = v.as_str().unwrap_or("");
    match msg {
        "ok" => {
            // Pequeno settle pra Angular reagir ao click.
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(())
        }
        "not_found" => Err(InscricaoMeiError::OcupacaoIndisponivel(oc.cnae.clone())),
        other => Err(anyhow::anyhow!("add_ocupacao_secundaria({:?}): {other}", oc.nome).into()),
    }
}

/// Seleciona uma ocupação dentro de um `<br-select name="...">` casando
/// pelo texto do `<label>` (após `trim()`). O `nome` vem de
/// `mei_cnaes.ocupacoes.nome` — fonte canônica. Falha com
/// [`InscricaoMeiError::OcupacaoIndisponivel`] se o nome não aparecer.
async fn select_ocupacao(
    page: &PageSession,
    br_select_name: &str,
    cnae: &str,
    nome: &str,
) -> Result<(), InscricaoMeiError> {
    let name_js = serde_json::to_string(br_select_name).map_err(anyhow::Error::from)?;
    let nome_js = serde_json::to_string(nome).map_err(anyhow::Error::from)?;
    let js = format!(
        r#"(() => {{
            const name = {name_js};
            const wanted = {nome_js};
            const sel = document.querySelector('br-select[name="' + name + '"]');
            if (!sel) return 'select_missing';
            const labels = sel.querySelectorAll('label');
            for (const lab of labels) {{
                if ((lab.textContent || '').trim() !== wanted) continue;
                const id = lab.getAttribute('for');
                const input = id ? sel.querySelector('#' + CSS.escape(id)) : null;
                if (!input) return 'input_missing';
                lab.click();
                input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'ok';
            }}
            return 'not_found';
        }})()"#
    );
    let v = page.eval_value(&js).await?;
    let msg = v.as_str().unwrap_or("");
    match msg {
        "ok" => {
            // Settle pra Angular reagir ao change.
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(())
        }
        "not_found" => Err(InscricaoMeiError::OcupacaoIndisponivel(cnae.to_string())),
        other => Err(anyhow::anyhow!("select_ocupacao({br_select_name}={nome:?}): {other}").into()),
    }
}

struct OcupacaoResolvida {
    familia: char,
    /// Nome canônico (`mei_cnaes.ocupacoes.nome`) — usado para casar com
    /// o texto do `<label>`/`<option>` do portal, já que os values são
    /// índices Angular opacos.
    nome: String,
}

struct OcupacaoSecundaria {
    cnae: String,
    nome: String,
}

/// Dados da seção "Atividades" já resolvidos (CNAEs → nomes, formas de
/// atuação → títulos) antes de abrir o browser. Evita passar muitos
/// parâmetros separados para [`fill_atividades`].
struct AtividadesResolvidas {
    familia: char,
    principal_cnae: String,
    principal_nome: String,
    secundarias: Vec<OcupacaoSecundaria>,
    formas_atuacao: Vec<(i32, String)>,
}

/// Resolve um CNAE → (código numérico, família) via `mei_cnaes.ocupacoes`.
/// O input é normalizado pra 7 dígitos (ignora pontuação). Falha com
/// [`InscricaoMeiError::CnaeNaoMapeado`] se não achar.
async fn resolver_cnae(pool: &Pool, cnae: &str) -> Result<OcupacaoResolvida, InscricaoMeiError> {
    let normalizado = digits_only(cnae);
    if normalizado.len() != 7 {
        return Err(anyhow::anyhow!(
            "CNAE deve ter 7 dígitos após remover pontuação, recebido {cnae:?}"
        )
        .into());
    }
    let cnae_param = normalizado.clone();
    let row = sql!(
        pool,
        "SELECT familia, nome
         FROM mei_cnaes.ocupacoes
         WHERE cnae = $cnae_param
         LIMIT 1"
    )
    .fetch_optional()
    .await
    .map_err(anyhow::Error::from)?;
    let Some(r) = row else {
        return Err(InscricaoMeiError::CnaeNaoMapeado(normalizado));
    };
    // `familia` é CHAR(1); cubos_sql devolve como String. Pega o
    // primeiro char — se vier vazio (não deveria, NOT NULL), devolvemos
    // desconhecida pra sinalizar dado corrompido na tabela.
    let familia = r.familia.trim().chars().next().unwrap_or(' ');
    Ok(OcupacaoResolvida {
        familia,
        nome: r.nome,
    })
}

/// Aguarda o form `app-inscrever` aparecer após clicar em "Entrar com gov.br".
///
/// Durante o polling pode acontecer:
/// - Sessão expirada → SSO redireciona para `sso.acesso.gov.br/login?...`.
///   Falha com [`InscricaoMeiError::SessaoInvalida`].
/// - Primeiro acesso ao portal MEI → SSO mostra a tela de consentimento
///   OAuth em `sso.acesso.gov.br/authorize?...`. Clicamos "Autorizar" e
///   seguimos aguardando o form.
/// - Já no portal, o `app-inscrever` pode renderizar antes o consentimento
///   LGPD (`mei-declaracao-lgpd`) — marcamos o checkbox obrigatório e
///   clicamos "Continuar".
/// - Banner `.br-message.danger` (ex.: CPF já vinculado a CNPJ) → falha
///   com [`InscricaoMeiError::Impedimento`].
async fn wait_for_inscrever_form(page: &PageSession, dom: &Dom) -> Result<(), InscricaoMeiError> {
    let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT;
    let mut autorizou_oauth = false;
    let mut aceitou_lgpd = false;
    loop {
        dom.invalidate();
        if dom
            .try_query_selector("app-inscrever input[name=identidade]")
            .await?
            .is_some()
        {
            return Ok(());
        }

        let url = page
            .eval_value("location.href")
            .await?
            .as_str()
            .unwrap_or("")
            .to_string();

        if url.starts_with("https://sso.acesso.gov.br/login") {
            let _ = page.debug_dump("mei-inscricao-sessao-invalida").await;
            return Err(InscricaoMeiError::SessaoInvalida);
        }

        // Banner de erro terminal no topo do portal MEI (ex.: CPF já
        // vinculado a CNPJ). O componente renderiza `mei-acessar-inscricao`
        // em vez de `app-inscrever`, então o form nunca aparece.
        if let Some(msg) = read_danger_message(page).await? {
            let _ = page.debug_dump("mei-inscricao-impedimento").await;
            return Err(InscricaoMeiError::Impedimento(msg));
        }

        // Tela de consentimento OAuth — aparece no primeiro acesso ao
        // MEI pelo SSO. O form tem dois `<button type=submit>`; o de
        // `value="true"` é "Autorizar".
        if url.starts_with("https://sso.acesso.gov.br/authorize") && !autorizou_oauth {
            dom.wait_for(
                "button[name=user_oauth_approval][value=true]",
                DEFAULT_TIMEOUT,
            )
            .await?;
            click_js(page, "button[name=user_oauth_approval][value=true]").await?;
            autorizou_oauth = true;
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        // Consentimento LGPD — o `app-inscrever` renderiza
        // `mei-declaracao-lgpd` antes do form real. Marca o checkbox
        // obrigatório e clica "Continuar".
        //
        // Marcar o checkbox no design system gov.br é frágil: o `<input>`
        // tem `opacity: 0` e a interação humana acontece via `<label>`,
        // então usamos uma cascata de tentativas (label click → input
        // click → setter + dispatch manual) pra garantir que o
        // ControlValueAccessor do Angular pegue a mudança e o form saia
        // de `ng-invalid`.
        if !aceitou_lgpd
            && dom
                .try_query_selector("mei-declaracao-lgpd input#declaracaoLGPD")
                .await?
                .is_some()
        {
            let resultado = page
                .eval_value(
                    r#"(() => {
                        const root = document.querySelector('mei-declaracao-lgpd');
                        if (!root) return 'root_missing';
                        const cb = root.querySelector('input#declaracaoLGPD');
                        if (!cb) return 'checkbox_missing';
                        if (!cb.checked) {
                            const label = root.querySelector('label[for=declaracaoLGPD]');
                            if (label) label.click();
                        }
                        if (!cb.checked) cb.click();
                        if (!cb.checked) {
                            const setter = Object.getOwnPropertyDescriptor(
                                window.HTMLInputElement.prototype, 'checked'
                            ).set;
                            setter.call(cb, true);
                            cb.dispatchEvent(new Event('input', { bubbles: true }));
                            cb.dispatchEvent(new Event('change', { bubbles: true }));
                        }
                        return cb.checked === true ? 'ok' : 'not_checked';
                    })()"#,
                )
                .await?;
            match resultado.as_str().unwrap_or("") {
                "ok" => {}
                outro => {
                    return Err(
                        anyhow::anyhow!("não consegui marcar o checkbox LGPD: {outro}").into(),
                    );
                }
            }
            // Dá tempo pro Angular rodar change detection e tirar o
            // form de ng-invalid antes do click em Continuar. 500ms
            // é generoso mas esse fluxo só roda uma vez por inscrição.
            tokio::time::sleep(Duration::from_millis(500)).await;
            click_js(page, "mei-declaracao-lgpd button.br-button.primary").await?;
            aceitou_lgpd = true;
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        if tokio::time::Instant::now() >= deadline {
            let _ = page.debug_dump("mei-inscricao-form-timeout").await;
            return Err(InscricaoMeiError::Other(anyhow::anyhow!(
                "timeout aguardando form app-inscrever aparecer (url atual: {url})"
            )));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Lê o texto do banner `.br-message.danger` se estiver presente. O
/// `<br>` dentro do texto vira um espaço pra facilitar a leitura pelo
/// caller.
async fn read_danger_message(page: &PageSession) -> Result<Option<String>, InscricaoMeiError> {
    let js = r#"
        (() => {
            const el = document.querySelector('br-message .br-message.danger .content span');
            if (!el) return null;
            return (el.innerText || el.textContent || "").replace(/\s+/g, " ").trim();
        })()
    "#;
    let v = page.eval_value(js).await?;
    Ok(v.as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string))
}

/// Poll até a modal Bootstrap ficar com a classe `.show`.
async fn wait_for_modal_open(page: &PageSession, id: &str) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT;
    let id_js = serde_json::to_string(id)?;
    let js = format!(
        r#"(() => {{
            const m = document.getElementById({id_js});
            return !!(m && m.classList.contains('show'));
        }})()"#
    );
    loop {
        let v = page.eval_value(&js).await?;
        if v.as_bool().unwrap_or(false) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timeout aguardando modal {id} abrir");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
