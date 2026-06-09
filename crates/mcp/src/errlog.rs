//! Formatação de erros com a cadeia completa de `source()` numa linha.
//!
//! Motivação: o `Display` de [`tokio_postgres::Error`] é só `"db error"` —
//! a mensagem real do servidor (ex: `operator does not exist: halfvec <=>
//! vector`) fica na `source()` (`DbError`). Como `pgsafe::Error::Database`
//! usa `#[error("database error: {0}")]`, logar o erro com o `Display`
//! padrão (`%e`) dá `"database error: db error"` e **esconde a causa**.
//!
//! Política de erro do servidor:
//! - **Console (`tracing`)**: detalhe COMPLETO via [`ErrChain::chain_string`]
//!   (std errors) ou [`anyhow_chain`] (anyhow) — pro dev diagnosticar.
//! - **Retorno da tool (vai pro LLM)**: mensagem GENÉRICA, sem internals de
//!   DB/infra. Erros de NEGÓCIO (gov.br recusou, impedimento MEI) continuam
//!   detalhados porque são informativos pro LLM.
//!
//! ```ignore
//! use crate::errlog::ErrChain;
//! tracing::warn!(error = %e.chain_string(), "falha ao consultar X"); // std error
//! tracing::warn!(error = %errlog::anyhow_chain(&e), "falha ao salvar"); // anyhow
//! ```

/// Renderiza um erro (`std::error::Error`) com toda a cadeia de `source()`
/// numa linha. Um blanket impl cobre `pgsafe::Error`, os erros do `rpa`
/// (`GovbrError`, `InscricaoMeiError`), `tokio_postgres::Error`, etc. —
/// qualquer tipo que implemente `Error`. Para `anyhow::Error` (que NÃO
/// implementa `Error`), use [`anyhow_chain`].
pub trait ErrChain {
    /// `self` seguido de cada `source()` subsequente, separados por `": "`.
    fn chain_string(&self) -> String;
}

impl<E: std::error::Error + ?Sized> ErrChain for E {
    fn chain_string(&self) -> String {
        use std::fmt::Write;
        let mut out = self.to_string();
        let mut source = self.source();
        while let Some(e) = source {
            let _ = write!(out, ": {e}");
            source = e.source();
        }
        out
    }
}

/// Versão de [`ErrChain::chain_string`] para [`anyhow::Error`], que não
/// implementa `std::error::Error` e por isso fica fora do blanket impl.
/// Equivale a `format!("{e:#}")`.
pub fn anyhow_chain(e: &anyhow::Error) -> String {
    e.chain()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(": ")
}

#[cfg(test)]
mod tests {
    use super::{ErrChain, anyhow_chain};

    #[test]
    fn anyhow_chain_descends_sources() {
        let base = std::io::Error::other("disco cheio");
        let err = anyhow::Error::new(base).context("falha ao gravar");
        assert_eq!(anyhow_chain(&err), "falha ao gravar: disco cheio");
    }

    #[test]
    fn pgsafe_chain_exposes_inner() {
        // `pgsafe::Error::Io` embrulha um `io::Error` via `#[from]`; a cadeia
        // deve revelar a mensagem interna — smoke test do blanket impl.
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "sumiu");
        let err = pgsafe::Error::Io(io);
        assert!(err.chain_string().contains("sumiu"));
    }
}
