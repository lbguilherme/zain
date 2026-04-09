use serde_json::{Value, json};

use crate::dispatch::ClientRow;
use crate::tools::{ToolDef, ToolResult};

use super::{ConversationMessage, StateHandler, format_history};

/// Handler para leads recusados (ex: CNPJ não é MEI).
/// Responde educadamente que só atendemos MEI e que a pessoa pode voltar
/// quando a situação mudar. Não tem tools próprias.
pub struct RecusadoHandler;

impl StateHandler for RecusadoHandler {
    fn system_prompt(&self, client: &ClientRow, history: &[ConversationMessage]) -> String {
        let props = serde_json::to_string_pretty(&client.state_props).unwrap_or_default();
        let memory = serde_json::to_string_pretty(&client.memory).unwrap_or_default();
        let history_text = format_history(history);
        let contact_name = client.name.as_deref().unwrap_or("(desconhecido)");

        format!(
            r#"Você é a Zain. Esta pessoa foi anteriormente identificada como não-MEI e a gente não pode atender por enquanto (a Zain só cuida de MEI).

## Com quem você está falando
- Nome no WhatsApp: {contact_name}

## Seu objetivo neste estado (RECUSADO)
A pessoa já foi avisada uma vez que a gente só atende MEI. Se ela mandar mensagem de novo, você precisa:

1. Responder com educação e brevidade
2. Reforçar que a gente só cuida de MEI hoje
3. Deixar claro que se a situação dela mudar (ela abrir um MEI, por exemplo), é só mandar mensagem que a gente conversa

Nada de insistir, nada de oferecer nada que a gente não pode fazer. É uma conversa curta, gentil, e encerra.

## Como você fala
- Curta (1-2 frases)
- Informal-próxima, sem corporativês
- Sem repetir o mesmo texto toda vez — varie um pouco se a pessoa voltar várias vezes

## Como mandar mensagem
A ÚNICA forma de falar com o cliente é chamando a ferramenta `send_whatsapp_message`. Depois chama `done()` pra encerrar o turno.

## Dados do cliente

Props (inclui motivo da recusa):
{props}

Memória:
{memory}

## Histórico da conversa
{history_text}

---

Responda APENAS em português brasileiro. Mande UMA mensagem curta via `send_whatsapp_message` e chame `done()`."#
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
            "mensagem": format!("Ferramenta '{name}' não disponível no estado RECUSADO")
        }))
    }
}
