mod lead;
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
    /// Retorna o trecho de system prompt específico deste estado.
    fn state_prompt(&self) -> String;

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
        _ => Box::new(stub::StubHandler {
            state: state.to_owned(),
        }),
    }
}

/// Monta o system prompt completo: base + estado.
pub fn build_system_prompt(handler: &dyn StateHandler) -> String {
    let now = chrono::Local::now().format("%d/%m/%Y %H:%M");
    let state_section = handler.state_prompt();

    format!(
        r#"Você é a Zain Gestão, uma assistente de gestão de MEI que funciona 100% pelo WhatsApp.

Data e hora atual: {now}

Sobre o serviço Zain Gestão:
- Primeiro mês GRÁTIS, depois R$ 19,90/mês no cartão de crédito
- Serviços inclusos: abertura de MEI, emissão de nota fiscal, DAS mensal, DASN anual, baixa de MEI, dúvidas contábeis/fiscais
- Tudo funciona por mensagem no WhatsApp, sem portal do governo, sem app extra
- Proativo: a Zain lembra do DAS, da DASN, monitora o teto de faturamento

IMPORTANTE — Como se comunicar:
- A ÚNICA forma de falar com o cliente é usando a ferramenta send_whatsapp_message.
- Você pode chamar múltiplas ferramentas na mesma resposta (ex: salvar dados E responder).
- Quando terminar de agir (enviou mensagem, salvou dados), chame done() para encerrar.
- Um fluxo típico: salvar dados → enviar mensagem → done().

Regras:
- Seja natural, simpática e direta. Use linguagem informal mas profissional.
- Responda APENAS em português brasileiro.
- Seja concisa. Mensagens de WhatsApp devem ser curtas e diretas.

{state_section}"#
    )
}

/// Monta a primeira user message com contexto dinâmico.
pub fn build_context_message(
    client: &ClientRow,
    history: &[ConversationMessage],
    new_message_count: usize,
    new_messages_summary: &str,
) -> String {
    let contact_name = client.name.as_deref().unwrap_or("(desconhecido)");
    let contact_phone = client.phone.as_deref().unwrap_or("(desconhecido)");
    let props = serde_json::to_string_pretty(&client.state_props).unwrap_or_default();
    let memory = serde_json::to_string_pretty(&client.memory).unwrap_or_default();
    let history_text = format_history(history);

    format!(
        r#"Informações do contato:
- Nome no WhatsApp: {contact_name}
- Telefone: {contact_phone}

Dados coletados até agora:
{props}

Memória do cliente:
{memory}

Histórico da conversa no WhatsApp:
{history_text}

O cliente enviou {new_message_count} nova(s) mensagem(ns):

{new_messages_summary}

Responda ao cliente usando send_whatsapp_message."#
    )
}

/// Formata o histórico de conversa como texto.
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
