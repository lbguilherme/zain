//! Tools expostas pelo servidor MCP. Cada submódulo define um
//! `pub struct Args` (`Deserialize + JsonSchema`) e uma `pub async fn
//! run(state: &AppState, client_id: Uuid, args: Args) -> Value`. A
//! integração com o protocolo MCP (extração do `_meta`, conversão
//! pra `CallToolResult`) fica em [`crate::server`].

pub mod abrir_empresa;
pub mod buscar_cnae;
pub mod das;
pub mod dasn;
pub mod get_ccmei;
pub mod get_client_state;
pub mod govbr;
mod pgfn;
pub mod recusar_lead;
pub mod save_cpf;
pub mod save_quer_abrir_mei;
