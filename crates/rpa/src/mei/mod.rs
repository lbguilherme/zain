//! RPA para consulta de optantes do Simples Nacional / SIMEI.

pub mod consulta;

pub use consulta::consultar_optante;

use serde::{Deserialize, Serialize};

const CONSULTA_URL: &str =
    "https://www8.receita.fazenda.gov.br/SimplesNacional/aplicacoes.aspx?id=21";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultaOptante {
    pub data_consulta: String,
    pub nome_empresarial: String,
    pub situacao_simples: Situacao,
    pub situacao_simei: Situacao,
    pub periodos_simples: Vec<Periodo>,
    pub periodos_simei: Vec<Periodo>,
    pub eventos_futuros_simples: Option<String>,
    pub eventos_futuros_simei: Option<String>,
    pub mei_transportador: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Situacao {
    pub optante: bool,
    /// Date in YYYY-MM-DD format, present when optante is true.
    pub desde: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Periodo {
    /// Date in YYYY-MM-DD format.
    pub data_inicial: String,
    /// Date in YYYY-MM-DD format.
    pub data_final: String,
    pub detalhamento: String,
}
