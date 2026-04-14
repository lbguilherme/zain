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
mod done;
mod govbr;
mod iniciar_pagamento;
mod pgfn;
mod recusar_lead;
mod save_atividade;
mod save_cnpj;
mod save_cpf;
mod save_endereco;
mod save_quer_abrir_mei;
mod send_whatsapp_message;

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
    pub fn as_chat_tool(&self) -> ai::ChatTool<'_> {
        ai::ChatTool {
            name: self.name,
            description: self.description,
            parameters: &self.parameters,
        }
    }
}

/// Resultado de uma chamada de tool: o valor retornado ao LLM e o
/// `memory` potencialmente modificado. Os campos estruturados (cpf,
/// cnpj, etc.) moram em colunas dedicadas de `zain.clients` e são
/// persistidos pelas próprias tools via UPDATE direto — só o `memory`
/// ainda é JSONB freestyle e flui pelo workflow.
pub struct ToolOutput {
    pub value: Value,
    pub memory: Value,
    /// Se `true`, o workflow garante que o LLM seja chamado ao menos
    /// mais uma vez antes de encerrar o turno — mesmo efeito de
    /// `Tool::must_use_tool_result`, só que decidido por chamada em
    /// vez de por tool. Útil quando uma tool às vezes devolve sucesso
    /// (pode seguir pro done) e às vezes devolve um erro que exige
    /// reação do LLM (ex: `save_cnpj` quando o CNPJ não é MEI).
    pub is_error: bool,
}

impl ToolOutput {
    /// Construtor padrão: sucesso, `is_error = false`.
    pub fn new(value: Value, memory: Value) -> Self {
        Self {
            value,
            memory,
            is_error: false,
        }
    }

    /// Marca este resultado como erro (força mais uma rodada do LLM).
    pub fn err(value: Value, memory: Value) -> Self {
        Self {
            value,
            memory,
            is_error: true,
        }
    }
}

pub type ToolFuture = Pin<Box<dyn Future<Output = ToolOutput> + Send>>;

/// Assinatura do handler: recebe o contexto de execução e os valores
/// `(args, memory)` por valor e devolve um futuro que produz o
/// [`ToolOutput`]. Trabalhar com valores owned evita brigas com HRTB;
/// o `ToolContext` é `Clone` (campos são `Arc`/`Pool`) e deve ser
/// clonado pelo caller antes de invocar o handler.
pub type ToolHandler = Box<dyn Fn(ToolContext, Value, Value) -> ToolFuture + Send + Sync>;

/// Uma tool instalada no agent: definição para o LLM + lambda que
/// executa a chamada.
pub struct Tool {
    pub def: ToolDef,
    pub handler: ToolHandler,
    /// Se `true`, o workflow garante que o LLM seja chamado ao menos
    /// mais uma vez depois desta tool antes de encerrar o turno: mesmo
    /// que `done` tenha sido chamado na mesma leva de tool calls, ele é
    /// ignorado e o loop segue para que o LLM veja o resultado da tool.
    pub must_use_tool_result: bool,
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
        save_cpf::tool(),
        save_quer_abrir_mei::tool(),
        save_cnpj::tool(),
        save_atividade::tool(),
        save_endereco::tool(),
        govbr::auth_tool(),
        govbr::otp_tool(),
        anotar::tool(),
        iniciar_pagamento::tool(),
        recusar_lead::tool(),
        consultar_cnae_por_codigo::tool(),
        buscar_cnae_por_atividade::tool(),
    ]
}

/// Gera o JSON Schema dos parâmetros de uma tool a partir de um tipo
/// Rust. O tipo precisa derivar `JsonSchema` (via `schemars`) — doc
/// comments `///` em cada campo viram `description` no schema.
///
/// Remove o ruído que o Ollama/OpenAI não esperam na raiz
/// (`$schema`, `title`) e normaliza campos opcionais: o `schemars`
/// emite `type: ["string", "null"]` para `Option<String>`, mas o
/// Gemini rejeita `type` como lista. Convertemos para o padrão
/// OpenAPI `type: "string", nullable: true`, que Gemini aceita e os
/// demais providers toleram.
pub fn params_for<T: JsonSchema>() -> Value {
    let schema = schema_for!(T);
    let mut v = serde_json::to_value(&schema).unwrap_or_else(|_| json!({ "type": "object" }));
    if let Some(obj) = v.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }
    normalize_nullable(&mut v);
    v
}

/// Caminha recursivamente pelo JSON Schema trocando
/// `type: [X, "null"]` por `type: X, nullable: true`. Lida com tipos
/// em qualquer posição — `properties`, `items`, `additionalProperties`,
/// dentro de `anyOf`/`allOf`/`oneOf`, etc. — por isso não inspeciona
/// chaves específicas, só a forma de cada objeto.
fn normalize_nullable(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            if let Some(Value::Array(arr)) = obj.get("type").cloned() {
                let mut non_null: Vec<Value> = Vec::new();
                let mut had_null = false;
                for item in arr {
                    if item.as_str() == Some("null") {
                        had_null = true;
                    } else {
                        non_null.push(item);
                    }
                }
                if had_null && non_null.len() == 1 {
                    obj.insert("type".into(), non_null.into_iter().next().unwrap());
                    obj.insert("nullable".into(), Value::Bool(true));
                } else if had_null {
                    // Mais de um tipo além de `null` — mantém como array sem
                    // o "null" e marca nullable, já que providers sérios não
                    // lidam com union types de qualquer jeito.
                    obj.insert("type".into(), Value::Array(non_null));
                    obj.insert("nullable".into(), Value::Bool(true));
                }
            }
            for (_, v) in obj.iter_mut() {
                normalize_nullable(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                normalize_nullable(v);
            }
        }
        _ => {}
    }
}

/// Handler tipado assíncrono: os `args` brutos são desserializados
/// para `T` antes de invocar `f`. Se a desserialização falhar, o
/// handler devolve um erro estruturado ao LLM sem chamar `f`.
pub(crate) fn typed_handler<T, F, Fut>(f: F) -> ToolHandler
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(ToolContext, T, Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ToolOutput> + Send + 'static,
{
    Box::new(move |ctx, raw_args, memory| {
        let fut_or_err = match serde_json::from_value::<T>(raw_args) {
            Ok(typed) => Ok(f(ctx, typed, memory)),
            Err(e) => Err((e, memory)),
        };
        Box::pin(async move {
            match fut_or_err {
                Ok(fut) => fut.await,
                Err((e, memory)) => ToolOutput::err(
                    json!({
                        "status": "erro",
                        "mensagem": format!("args inválidos: {e}"),
                    }),
                    memory,
                ),
            }
        })
    })
}

/// Variante síncrona do [`typed_handler`]. Para tools que só mexem em
/// `memory` sem precisar de await nem do contexto de execução.
pub(crate) fn typed_sync_handler<T, F>(f: F) -> ToolHandler
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(T, Value) -> ToolOutput + Send + Sync + 'static,
{
    Box::new(move |_ctx, raw_args, memory| {
        let out = match serde_json::from_value::<T>(raw_args) {
            Ok(typed) => f(typed, memory),
            Err(e) => ToolOutput::err(
                json!({
                    "status": "erro",
                    "mensagem": format!("args inválidos: {e}"),
                }),
                memory,
            ),
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
    /// Também checa que nenhum `type` é uma lista — o Gemini rejeita
    /// `type: ["string", "null"]` e exige `nullable: true`.
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
            assert_no_array_type(name, &tool.def.parameters);
        }
    }

    fn assert_no_array_type(tool_name: &str, value: &Value) {
        match value {
            Value::Object(obj) => {
                if let Some(t) = obj.get("type") {
                    assert!(
                        !t.is_array(),
                        "{tool_name}: type como array não é aceito por Gemini — use nullable: true. Offending node: {obj:?}"
                    );
                }
                for v in obj.values() {
                    assert_no_array_type(tool_name, v);
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    assert_no_array_type(tool_name, v);
                }
            }
            _ => {}
        }
    }
}
