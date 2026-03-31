use anyhow::{Context, Result};
use cubos_sql::sql;
use tokio_postgres::{Client, NoTls, Transaction};

use crate::schema::Table;
use crate::source::SchemaVersion;

pub async fn connect(url: &str) -> Result<Client> {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .with_context(|| format!("falha ao conectar em {url}"))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("erro na conexão: {e}");
        }
    });

    Ok(client)
}

pub async fn read_schema_version(
    client: &Client,
    schema: &str,
) -> Result<Option<SchemaVersion>> {
    let row = sql!(
        client,
        "SELECT obj_description(oid) as comment FROM pg_namespace WHERE nspname = $schema"
    )
    .fetch_optional()
    .await?;

    match row.and_then(|r| r.comment) {
        Some(json) => match serde_json::from_str(&json) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

pub async fn create_temp_schema(
    tx: &Transaction<'_>,
    tables: &[Table],
    setup_ddl: &[&str],
) -> Result<String> {
    let schema = format!("tmp_{}", uuid::Uuid::new_v4());
    tx.execute(&format!("CREATE SCHEMA \"{schema}\""), &[])
        .await?;

    for ddl in setup_ddl {
        let ddl = ddl.replace("{schema}", &schema);
        tx.execute(&ddl, &[]).await?;
    }

    for table in tables {
        let ddl = table.create_table_sql(&schema);
        tx.execute(&ddl, &[])
            .await
            .with_context(|| format!("falha ao criar tabela {}", table.name))?;
    }

    Ok(schema)
}

pub async fn create_indexes(tx: &Transaction<'_>, schema: &str, tables: &[Table]) -> Result<()> {
    for table in tables {
        for ddl_template in table.extra_ddl {
            let ddl = ddl_template.replace("{schema}", schema);
            tx.execute(&ddl, &[])
                .await
                .with_context(|| format!("falha ao criar índice em {}", table.name))?;
        }
    }
    Ok(())
}

pub async fn swap_schemas(
    tx: &Transaction<'_>,
    temp_schema: &str,
    target_schema: &str,
    tables: &[Table],
    version: &SchemaVersion,
) -> Result<()> {
    tx.execute(
        &format!("CREATE SCHEMA IF NOT EXISTS \"{target_schema}\""),
        &[],
    )
    .await?;

    for table in tables.iter().rev() {
        tx.execute(
            &format!(
                "DROP TABLE IF EXISTS \"{target_schema}\".\"{}\" CASCADE",
                table.name
            ),
            &[],
        )
        .await?;
    }

    // Move custom types do schema temporário para o target
    let types = sql!(
        tx,
        "SELECT typname FROM pg_type t JOIN pg_namespace n ON t.typnamespace = n.oid \
         WHERE n.nspname = $temp_schema AND t.typtype = 'e'"
    )
    .fetch_all()
    .await?;
    for row in &types {
        let typname = row.typname.as_str();
        // Drop o tipo antigo no target se existir
        tx.execute(
            &format!("DROP TYPE IF EXISTS \"{target_schema}\".{typname} CASCADE"),
            &[],
        )
        .await?;
        tx.execute(
            &format!("ALTER TYPE \"{temp_schema}\".{typname} SET SCHEMA \"{target_schema}\""),
            &[],
        )
        .await?;
    }

    for table in tables {
        tx.execute(
            &format!(
                "ALTER TABLE \"{temp_schema}\".\"{}\" SET SCHEMA \"{target_schema}\"",
                table.name
            ),
            &[],
        )
        .await?;
    }

    tx.execute(&format!("DROP SCHEMA \"{temp_schema}\""), &[])
        .await?;

    let comment = serde_json::to_string(version)?;
    tx.execute(
        &format!("COMMENT ON SCHEMA \"{target_schema}\" IS '{comment}'"),
        &[],
    )
    .await?;

    Ok(())
}
