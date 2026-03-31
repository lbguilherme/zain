use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio_postgres::Transaction;

use crate::schema::Table;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub data_version: String,
    pub extractor_version: u32,
}

pub struct Download {
    pub url: String,
    pub filename: String,
}

pub trait DataSource {
    fn schema_name(&self) -> &str;
    fn data_version(&self) -> &str;
    fn extractor_version(&self) -> u32;
    fn tables(&self) -> &'static [Table];
    fn downloads(&self) -> Vec<Download>;

    /// DDL executado antes da criação das tabelas (ex: CREATE TYPE).
    fn setup_ddl(&self) -> &'static [&'static str] {
        &[]
    }

    /// Importa dados para o schema temporário.
    /// Default: pipeline ZIP/CSV via import::import_all.
    /// Sources podem sobrescrever para lógica customizada (ex: API JSON).
    fn import_data<'a>(
        &self,
        tx: &'a Transaction<'a>,
        temp_schema: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        let data_dir = self.data_dir();
        let tables = self.tables();
        Box::pin(async move {
            crate::import::import_all(tx, temp_schema, &data_dir, tables).await
        })
    }

    fn data_dir(&self) -> PathBuf {
        PathBuf::from(".dados_abertos")
            .join(self.schema_name())
            .join(self.data_version())
    }

    fn current_version(&self) -> SchemaVersion {
        SchemaVersion {
            data_version: self.data_version().to_string(),
            extractor_version: self.extractor_version(),
        }
    }

    fn needs_update(&self, installed: Option<&SchemaVersion>) -> bool {
        match installed {
            None => true,
            Some(v) => {
                v.data_version != self.data_version()
                    || v.extractor_version != self.extractor_version()
            }
        }
    }
}
