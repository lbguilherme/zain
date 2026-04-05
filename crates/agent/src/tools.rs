use serde_json::{Value, json};

/// Definição de uma tool para enviar ao Ollama.
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

/// Resultado da execução de uma tool.
pub enum ToolResult {
    /// Tool executada com sucesso, retorna valor para o LLM.
    Ok(Value),
    /// Tool que transiciona o estado da máquina de estados.
    StateTransition { new_state: String, new_props: Value },
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

/// Tool global disponível em todos os estados.
pub fn send_whatsapp_message_tool() -> ToolDef {
    ToolDef {
        name: "send_whatsapp_message",
        description: "Envia uma mensagem de texto para o cliente no WhatsApp. Esta é a ÚNICA forma de se comunicar com o cliente. Toda resposta deve ser enviada através desta ferramenta.",
        parameters: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Texto da mensagem a enviar para o cliente"
                }
            },
            "required": ["message"]
        }),
    }
}
