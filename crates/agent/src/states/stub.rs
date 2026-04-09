use serde_json::{Value, json};

use crate::dispatch::ClientRow;
use crate::tools::{ToolDef, ToolResult};

use super::{ConversationMessage, StateHandler, format_history};

/// Handler genérico para estados ainda não implementados.
pub struct StubHandler {
    pub state: String,
}

impl StateHandler for StubHandler {
    fn system_prompt(&self, _client: &ClientRow, history: &[ConversationMessage]) -> String {
        let history_text = format_history(history);

        format!(
            r#"Você é a Zain Gestão. O cliente está no estado "{state}" que ainda está em fase de implementação.

IMPORTANTE — Como se comunicar:
- A ÚNICA forma de falar com o cliente é usando a ferramenta send_whatsapp_message.
- Quando terminar de agir, chame done() para encerrar.

Informe educadamente que esse fluxo ainda não está disponível e que a equipe está trabalhando nisso.
Peça desculpas pela inconveniência e diga que em breve estará funcionando.
Responda em português brasileiro, de forma curta e simpática.

Histórico da conversa no WhatsApp:
{history_text}"#,
            state = self.state
        )
    }

    fn tool_definitions(&self) -> Vec<ToolDef> {
        vec![]
    }

    fn execute_tool(
        &self,
        name: &str,
        _args: &Value,
        _state_props: &mut Value,
        _memory: &mut Value,
    ) -> ToolResult {
        ToolResult::Ok(json!({
            "status": "erro",
            "mensagem": format!("Ferramenta '{name}' não disponível no estado '{}'", self.state)
        }))
    }
}
