use serde_json::{Value, json};

use crate::tools::{ToolDef, ToolResult};

use super::StateHandler;

/// Handler genérico para estados ainda não implementados.
pub struct StubHandler {
    pub state: String,
}

impl StateHandler for StubHandler {
    fn state_prompt(&self) -> String {
        format!(
            r#"O cliente está no estado "{state}" que ainda está em fase de implementação.

Informe educadamente que esse fluxo ainda não está disponível e que a equipe está trabalhando nisso.
Peça desculpas pela inconveniência e diga que em breve estará funcionando."#,
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
