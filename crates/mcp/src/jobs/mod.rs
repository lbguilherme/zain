//! Tarefas de background do servidor MCP. Cada submódulo expõe um
//! `run_forever(state)` que roda num `tokio::spawn` disparado em
//! [`crate::main`].

pub mod das_refresh;
pub mod dasn_refresh;
pub mod mei_refresh;
