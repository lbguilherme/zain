use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use deadpool_postgres::Pool;
use schemars::{JsonSchema, schema_for};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::dispatch::{ClientRow, Models};

mod anotar;
mod buscar_cnae_por_atividade;
mod consultar_cnae_por_codigo;
mod consultar_divida_pgfn;
mod consultar_simei_cnpj;
mod done;
mod iniciar_pagamento;
mod recusar_lead;
mod send_whatsapp_message;
mod set_atividade;
mod set_cnpj;
mod set_dados_pessoais;
mod set_endereco;
mod set_gov_br;
mod set_tem_mei;

/// Definição de uma tool — o que o LLM vê.
#[derive(Debug)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
    /// Tools com efeitos externos (enviar msg, consulta lenta).
    /// Antes da primeira tool consequencial, o workflow verifica se chegou
    /// mensagem nova e reinicia o processamento se necessário.
    pub consequential: bool,
}

impl ToolDef {
    pub fn to_ollama_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// Resultado de uma chamada de tool: o valor retornado ao LLM, somado
/// ao estado (props + memory) potencialmente modificado. O workflow
/// aplica os valores retornados sobre o estado in-memory do client.
pub struct ToolOutput {
    pub value: Value,
    pub props: Value,
    pub memory: Value,
}

pub type ToolFuture = Pin<Box<dyn Future<Output = ToolOutput> + Send>>;

/// Assinatura do handler: recebe o contexto de execução e os valores
/// `(args, props, memory)` por valor e devolve um futuro que produz o
/// [`ToolOutput`]. Trabalhar com valores owned evita brigas com HRTB;
/// o `ToolContext` é `Clone` (campos são `Arc`/`Pool`) e deve ser
/// clonado pelo caller antes de invocar o handler.
pub type ToolHandler = Box<dyn Fn(ToolContext, Value, Value, Value) -> ToolFuture + Send + Sync>;

/// Uma tool instalada no agent: definição para o LLM + lambda que
/// executa a chamada.
pub struct Tool {
    pub def: ToolDef,
    pub handler: ToolHandler,
}

/// Contexto de execução de uma tool call — passado como argumento pro
/// handler no momento da invocação. Carrega tudo o que uma tool pode
/// precisar de recursos compartilhados ou de identidade do cliente.
#[derive(Clone)]
pub struct ToolContext {
    pub pool: Pool,
    pub ai: Arc<ai::Client>,
    pub models: Arc<Models>,
    pub client_id: Uuid,
    pub chat_id: String,
}

impl ToolContext {
    pub fn new(pool: Pool, ai: Arc<ai::Client>, models: Arc<Models>, client: &ClientRow) -> Self {
        Self {
            pool,
            ai,
            models,
            client_id: client.id,
            chat_id: client.chat_id.clone(),
        }
    }
}

/// Retorna a lista completa de tools. Cada factory é pura — não
/// captura nada; o contexto flui pelo handler em cada chamada.
pub fn all_tools() -> Vec<Tool> {
    vec![
        send_whatsapp_message::tool(),
        done::tool(),
        set_dados_pessoais::tool(),
        set_tem_mei::tool(),
        set_cnpj::tool(),
        set_atividade::tool(),
        set_endereco::tool(),
        set_gov_br::tool(),
        anotar::tool(),
        iniciar_pagamento::tool(),
        recusar_lead::tool(),
        consultar_simei_cnpj::tool(),
        consultar_cnae_por_codigo::tool(),
        buscar_cnae_por_atividade::tool(),
        consultar_divida_pgfn::tool(),
    ]
}

/// Gera o JSON Schema dos parâmetros de uma tool a partir de um tipo
/// Rust. O tipo precisa derivar `JsonSchema` (via `schemars`) — doc
/// comments `///` em cada campo viram `description` no schema.
///
/// Remove o ruído que o Ollama/OpenAI não esperam na raiz
/// (`$schema`, `title`).
pub fn params_for<T: JsonSchema>() -> Value {
    let schema = schema_for!(T);
    let mut v = serde_json::to_value(&schema).unwrap_or_else(|_| json!({ "type": "object" }));
    if let Some(obj) = v.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    v
}

/// Handler tipado assíncrono: os `args` brutos são desserializados
/// para `T` antes de invocar `f`. Se a desserialização falhar, o
/// handler devolve um erro estruturado ao LLM sem chamar `f`.
pub(crate) fn typed_handler<T, F, Fut>(f: F) -> ToolHandler
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(ToolContext, T, Value, Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ToolOutput> + Send + 'static,
{
    Box::new(move |ctx, raw_args, props, memory| {
        let fut_or_err = match serde_json::from_value::<T>(raw_args) {
            Ok(typed) => Ok(f(ctx, typed, props, memory)),
            Err(e) => Err((e, props, memory)),
        };
        Box::pin(async move {
            match fut_or_err {
                Ok(fut) => fut.await,
                Err((e, props, memory)) => ToolOutput {
                    value: json!({
                        "status": "erro",
                        "mensagem": format!("args inválidos: {e}"),
                    }),
                    props,
                    memory,
                },
            }
        })
    })
}

/// Variante síncrona do [`typed_handler`]. Para tools que só mexem em
/// `props`/`memory` sem precisar de await nem do contexto de execução.
pub(crate) fn typed_sync_handler<T, F>(f: F) -> ToolHandler
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(T, Value, Value) -> ToolOutput + Send + Sync + 'static,
{
    Box::new(move |_ctx, raw_args, props, memory| {
        let out = match serde_json::from_value::<T>(raw_args) {
            Ok(typed) => f(typed, props, memory),
            Err(e) => ToolOutput {
                value: json!({
                    "status": "erro",
                    "mensagem": format!("args inválidos: {e}"),
                }),
                props,
                memory,
            },
        };
        Box::pin(async move { out })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garante que todos os schemas gerados são objetos JSON com
    /// `type: "object"` e sem ruído na raiz — alguns providers de LLM
    /// rejeitam schemas com `$schema`, `title` ou `$defs` no topo.
    #[test]
    fn schemas_have_clean_root() {
        for tool in all_tools() {
            let name = tool.def.name;
            let obj = tool
                .def
                .parameters
                .as_object()
                .unwrap_or_else(|| panic!("{name}: parameters deve ser um object"));
            assert!(!obj.contains_key("$schema"), "{name}: raiz tem $schema");
            assert!(!obj.contains_key("title"), "{name}: raiz tem title");
            assert!(!obj.contains_key("$defs"), "{name}: raiz tem $defs");
            assert_eq!(
                obj.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "{name}: raiz sem type=object"
            );
        }
    }
}
