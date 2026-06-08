//! Recursos MCP servidos por este crate. Cada submódulo define seu
//! próprio URI scheme + lookup. O [`crate::server`] despacha
//! `resources/list` e `resources/read` pra cá.

pub mod ccmei;
