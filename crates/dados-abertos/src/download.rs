use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;

use crate::source::Download;

const MAX_CONCURRENT: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn download_all(downloads: &[Download], data_dir: &Path) -> Result<()> {
    tokio::fs::create_dir_all(data_dir).await?;

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let mut to_download: Vec<(String, String, PathBuf)> = Vec::new();

    println!("Verificando arquivos existentes...");
    for dl in downloads {
        let path = data_dir.join(&dl.filename);

        if path.exists() {
            let local_size = tokio::fs::metadata(&path).await?.len();
            match get_remote_size(&client, &dl.url).await {
                Ok(Some(remote_size)) => {
                    if local_size == remote_size {
                        println!("  {} OK ({local_size} bytes)", dl.filename);
                        continue;
                    }
                    println!(
                        "  {} tamanho incorreto (local={local_size}, remoto={remote_size}), re-baixando.",
                        dl.filename
                    );
                    tokio::fs::remove_file(&path).await?;
                }
                Ok(None) => {
                    println!(
                        "  {} já existe (servidor sem Content-Length, assumindo OK)",
                        dl.filename
                    );
                    continue;
                }
                Err(e) => {
                    println!("  {} verificação falhou ({e:#}), assumindo OK", dl.filename);
                    continue;
                }
            }
        }

        let part_path = path.with_extension("part");
        if part_path.exists() {
            let part_size = tokio::fs::metadata(&part_path).await?.len();
            println!(
                "  {} parcial encontrado ({part_size} bytes), vai resumir.",
                dl.filename
            );
        }

        to_download.push((dl.url.clone(), dl.filename.clone(), path));
    }

    if to_download.is_empty() {
        println!("Todos os arquivos já existem e estão completos.");
        return Ok(());
    }

    println!(
        "Baixando {} arquivo(s) (max {MAX_CONCURRENT} simultâneos)...",
        to_download.len()
    );

    let multi = MultiProgress::new();
    let style = ProgressStyle::with_template(
        "{prefix:.bold} [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})",
    )
    .unwrap()
    .progress_chars("█▉▊▋▌▍▎▏ ");

    let semaphore = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::new();

    for (url, filename, path) in to_download {
        let pb = multi.add(ProgressBar::new(0));
        pb.set_style(style.clone());
        pb.set_prefix(filename.clone());
        let client = client.clone();
        let sem = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = download_with_retries(&client, &url, &path, &pb).await;
            if let Err(e) = &result {
                pb.abandon_with_message(format!("ERRO: {e}"));
            } else {
                pb.finish_with_message("OK");
            }
            result.with_context(|| format!("falha ao baixar {filename}"))
        }));
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// Faz um GET e lê o Content-Length dos headers sem consumir o body.
/// O servidor da Receita não suporta HEAD corretamente (retorna Content-Length: 0).
async fn get_remote_size(client: &reqwest::Client, url: &str) -> Result<Option<u64>> {
    let resp = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("HTTP error para {url}"))?;

    let size = resp.content_length().filter(|&s| s > 0);
    drop(resp);
    Ok(size)
}

async fn download_with_retries(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match download_file(client, url, path, pb).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                pb.set_message(format!("tentativa {attempt} falhou: {e:#}, retentando..."));
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    let tmp_path = path.with_extension("part");

    let existing_size = match tokio::fs::metadata(&tmp_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };

    let mut req = client.get(url);
    if existing_size > 0 {
        req = req.header("Range", format!("bytes={existing_size}-"));
    }

    let resp = req
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("HTTP error para {url}"))?;

    let status = resp.status();
    let (mut downloaded, total_size) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let total = resp
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit('/').next())
            .and_then(|v| v.parse::<u64>().ok());
        (existing_size, total)
    } else {
        if existing_size > 0 {
            let _ = tokio::fs::remove_file(&tmp_path).await;
        }
        (0u64, resp.content_length())
    };

    if let Some(len) = total_size {
        pb.set_length(len);
    }
    pb.set_position(downloaded);

    let mut file = if downloaded > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&tmp_path)
            .await?
    } else {
        tokio::fs::File::create(&tmp_path).await?
    };

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("erro ao ler dados do download")?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
    }

    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);

    if let Some(expected) = total_size {
        let actual = tokio::fs::metadata(&tmp_path).await?.len();
        if actual != expected {
            anyhow::bail!(
                "download incompleto: esperado {expected} bytes, recebido {actual} bytes"
            );
        }
    }

    tokio::fs::rename(&tmp_path, path).await?;
    Ok(())
}
