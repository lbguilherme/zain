//! Login e verificação de perfil no gov.br.
//!
//! Tabela de apoio: [`zain.govbr`](../../../../migrations/0011_govbr.sql).
//! Guarda credenciais, OTP e uma sessão serializada (cookies + user-agent)
//! que permite reutilizar o "navegador confiável" entre execuções.

pub mod db;
pub mod extension;
pub mod flow;
pub mod launch;
pub mod session;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use flow::{CheckOutcome, GovbrError, Profile, check_govbr_profile};

/// Selos gov.br por ordem crescente de confiabilidade.
///
/// Mapeado para o enum `zain.govbr_nivel` do banco — a configuração em
/// `Cargo.toml` garante (de)serialização automática nas queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Nivel {
    Bronze,
    Prata,
    Ouro,
}

impl Nivel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Nivel::Bronze => "bronze",
            Nivel::Prata => "prata",
            Nivel::Ouro => "ouro",
        }
    }
}

impl fmt::Display for Nivel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Nivel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Aceita match frouxo: ignora case/acentos comuns, sufixos tipo
        // "Selo Prata".
        let norm = s.trim().to_lowercase().replace('á', "a").replace('ê', "e");
        if norm.contains("ouro") {
            Ok(Nivel::Ouro)
        } else if norm.contains("prata") {
            Ok(Nivel::Prata)
        } else if norm.contains("bronze") {
            Ok(Nivel::Bronze)
        } else {
            Err(format!("nível gov.br desconhecido: {s:?}"))
        }
    }
}
