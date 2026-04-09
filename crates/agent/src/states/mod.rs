mod lead;
mod recusado;
mod stub;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::dispatch::ClientRow;
use crate::tools::{ToolDef, ToolResult};

/// Mensagem do histórico de conversa do WhatsApp.
pub struct ConversationMessage {
    pub from_me: bool,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
}

pub trait StateHandler: Send + Sync {
    /// Gera o system prompt incluindo histórico de conversa.
    fn system_prompt(&self, client: &ClientRow, history: &[ConversationMessage]) -> String;

    /// Tools específicas deste estado (sem send_whatsapp_message, que é global).
    fn tool_definitions(&self) -> Vec<ToolDef>;

    /// Executa uma tool específica deste estado.
    fn execute_tool(
        &self,
        name: &str,
        args: &Value,
        state_props: &mut Value,
        memory: &mut Value,
    ) -> ToolResult;
}

pub fn get_handler(state: &str) -> Box<dyn StateHandler> {
    match state {
        "LEAD" => Box::new(lead::LeadHandler),
        "RECUSADO" => Box::new(recusado::RecusadoHandler),
        _ => Box::new(stub::StubHandler {
            state: state.to_owned(),
        }),
    }
}

/// Formata o histórico de conversa como texto para incluir no system prompt.
/// Inclui headers de data/hora quando há intervalo > 1h entre mensagens,
/// e sempre antes da primeira mensagem.
pub fn format_history(history: &[ConversationMessage]) -> String {
    if history.is_empty() {
        return "(sem histórico de conversa)".into();
    }

    let mut lines = Vec::new();
    let mut last_ts: Option<DateTime<Utc>> = None;

    for msg in history {
        // Header de data/hora: antes da primeira msg, ou quando intervalo > 1h
        if let Some(ts) = msg.timestamp {
            let should_add_header = match last_ts {
                None => true,
                Some(prev) => (ts - prev).num_hours() >= 1,
            };

            if should_add_header {
                let formatted = ts.format("── %d/%m/%Y %H:%M ──");
                lines.push(format!("\n{formatted}"));
            }

            last_ts = Some(ts);
        }

        let sender = if msg.from_me { "Zain" } else { "Cliente" };
        lines.push(format!(
            "[{sender}]: <message_text>{}</message_text>",
            msg.text
        ));
    }

    lines.join("\n")
}
