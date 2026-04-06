use std::io::Read;
use std::path::Path;
use std::pin::pin;

use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use futures_util::SinkExt;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::mpsc;
use tokio_postgres::Transaction;

use crate::schema::{NormalizeFn, Table};

pub async fn import_all(
    tx: &Transaction<'_>,
    temp_schema: &str,
    data_dir: &Path,
    tables: &[Table],
) -> Result<()> {
    for table in tables {
        import_table(tx, temp_schema, data_dir, table).await?;
    }
    Ok(())
}

async fn import_table(
    tx: &Transaction<'_>,
    schema: &str,
    data_dir: &Path,
    table: &Table,
) -> Result<()> {
    let filenames = table.zip_filenames();
    let mut total_rows: u64 = 0;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {prefix} {human_pos} registros ({per_sec})").unwrap(),
    );
    pb.set_prefix(table.name.to_string());

    for filename in &filenames {
        let zip_path = data_dir.join(filename);
        if !zip_path.exists() {
            pb.finish_and_clear();
            anyhow::bail!("arquivo não encontrado: {}", zip_path.display());
        }

        let normalizers: Vec<Option<NormalizeFn>> =
            table.columns.iter().map(|c| c.normalize).collect();
        let table_name = table.name;
        let has_headers = table.has_headers;
        let delimiter = table.delimiter;
        let csv_filename = table.csv_filename.map(|s| s.to_string());
        let (sender, mut rx) = mpsc::channel::<String>(10_000);

        let zip_path_owned = zip_path.clone();
        let filename_owned = filename.clone();
        let reader_handle = tokio::task::spawn_blocking(move || {
            read_csv_from_zip(
                &zip_path_owned,
                &normalizers,
                has_headers,
                delimiter,
                sender,
                &filename_owned,
                csv_filename.as_deref(),
            )
        });

        let copy_sql = table.copy_in_sql(schema);
        let sink = tx
            .copy_in::<_, bytes::Bytes>(&copy_sql)
            .await
            .with_context(|| format!("falha ao iniciar COPY para {table_name}"))?;
        let mut sink = pin!(sink);

        let mut row_count: u64 = 0;
        let mut buf = BytesMut::with_capacity(1024 * 1024);

        while let Some(line) = rx.recv().await {
            buf.put_slice(line.as_bytes());
            buf.put_u8(b'\n');
            row_count += 1;

            if buf.len() >= 1024 * 1024 {
                pb.set_position(total_rows + row_count);
                sink.as_mut()
                    .send(buf.split().freeze())
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("falha ao enviar dados COPY para {table_name}: {e:?}")
                    })?;
            }
        }

        if !buf.is_empty() {
            sink.as_mut().send(buf.freeze()).await.map_err(|e| {
                anyhow::anyhow!("falha ao enviar dados COPY para {table_name}: {e:?}")
            })?;
        }

        sink.as_mut()
            .finish()
            .await
            .map_err(|e| anyhow::anyhow!("falha ao finalizar COPY para {table_name}: {e:?}"))?;

        reader_handle
            .await?
            .with_context(|| format!("falha ao ler {filename}"))?;

        total_rows += row_count;
        pb.set_position(total_rows);
    }

    pb.finish_with_message(format!("{total_rows} registros"));
    Ok(())
}

fn read_csv_from_zip(
    zip_path: &Path,
    normalizers: &[Option<NormalizeFn>],
    has_headers: bool,
    delimiter: u8,
    sender: mpsc::Sender<String>,
    filename: &str,
    csv_filename: Option<&str>,
) -> Result<()> {
    let file = std::fs::File::open(zip_path)
        .with_context(|| format!("falha ao abrir {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("falha ao ler zip {}", zip_path.display()))?;

    if archive.is_empty() {
        anyhow::bail!("zip vazio: {filename}");
    }

    let col_count = normalizers.len();

    for i in 0..archive.len() {
        let mut csv_file = archive.by_index(i)?;
        let csv_name = csv_file.name().to_string();

        if let Some(target) = csv_filename {
            let basename = csv_name.rsplit('/').next().unwrap_or(&csv_name);
            if !basename.eq_ignore_ascii_case(target) {
                continue;
            }
        }

        let mut raw_bytes = Vec::new();
        csv_file
            .read_to_end(&mut raw_bytes)
            .with_context(|| format!("falha ao extrair {csv_name} de {filename}"))?;

        let (utf8, _, had_errors) = encoding_rs::WINDOWS_1252.decode(&raw_bytes);
        if had_errors {
            eprintln!("  aviso: erros de encoding em {csv_name}");
        }

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(has_headers)
            .flexible(true)
            .from_reader(utf8.as_bytes());

        for (line_num, result) in reader.records().enumerate() {
            let record =
                result.with_context(|| format!("erro na linha {line_num} de {csv_name}"))?;

            let mut fields = Vec::with_capacity(col_count);
            #[allow(clippy::needless_range_loop)]
            for j in 0..col_count {
                let val = record.get(j).unwrap_or("").trim();
                let normalized = if val.is_empty() {
                    "\\N".to_string()
                } else if let Some(f) = normalizers[j] {
                    f(val).map_err(|msg| {
                        let row: Vec<_> = (0..col_count)
                            .map(|k| record.get(k).unwrap_or(""))
                            .collect();
                        anyhow::anyhow!(
                            "{msg}: {val:?} (coluna {j}, linha {line_num} de {csv_name})\n  row: {row:?}"
                        )
                    })?
                } else {
                    escape_copy(val)
                };
                fields.push(normalized);
            }

            let line = fields.join("\t");
            if sender.blocking_send(line).is_err() {
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Escapa caracteres especiais para COPY text format.
/// Remove null bytes (0x00) que o PostgreSQL rejeita em campos TEXT.
fn escape_copy(val: &str) -> String {
    val.replace('\0', "")
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
