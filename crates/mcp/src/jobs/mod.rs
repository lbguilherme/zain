//! Tarefas de background do servidor MCP. Um único worker
//! ([`refresh::run_forever`]) cuida de todos os refreshes (MEI/DAS/DASN),
//! disparado num `tokio::spawn` em [`crate::main`].

pub mod refresh;
