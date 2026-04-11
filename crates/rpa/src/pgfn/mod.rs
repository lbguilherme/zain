//! RPA para consulta de dívida ativa na PGFN.

pub mod consulta;

pub use consulta::consultar_divida;

use serde::{Deserialize, Serialize};

const PGFN_URL: &str = "https://www.listadevedores.pgfn.gov.br/";

/// Resultado da consulta de dívida ativa na PGFN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultaDivida {
    /// CPF ou CNPJ consultado (apenas dígitos).
    pub documento: String,
    /// Se encontrou registro de dívida ativa.
    pub tem_divida: bool,
    /// Valor total da dívida em R$ (0.0 se sem dívida).
    pub total_divida: f64,
    /// Nome do devedor, se encontrado.
    pub nome: Option<String>,
}
