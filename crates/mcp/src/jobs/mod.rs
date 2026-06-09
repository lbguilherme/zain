//! Tarefas de background do servidor MCP. Cada submódulo expõe um
//! `run_forever(state)` que roda num `tokio::spawn` disparado em
//! [`crate::main`].

pub mod mei_refresh;
