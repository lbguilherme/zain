//! Download e unpack on-demand de extensões da Chrome Web Store.
//!
//! Dado o ID de uma extensão (ex: NopeCHA = `dknlfmjaanfblgfdfebhijalfmhmjjjo`),
//! garante que existe um diretório unpacked em
//! `$TMPDIR/zain-chrome-extensions/{id}/` pronto para passar ao Chromium via
//! `--load-extension=<dir>`. Se já existe e parece válido, é reusado. Se não,
//! baixa o `.crx` do endpoint público da Google e descomprime.
//!
//! ## Sobre o formato CRX
//!
//! Um arquivo `.crx` da Chrome Web Store é um ZIP comum com um header binário
//! prefixado. Existem dois formatos vivos hoje:
//!
//! ```text
//! CRX2: "Cr24" (4) | version=2 (4) | pubkey_len (4) | sig_len (4) | pubkey | sig | ZIP
//! CRX3: "Cr24" (4) | version=3 (4) | header_len  (4) | header (protobuf)        | ZIP
//! ```
//!
//! Não validamos assinatura — só extraímos a parte ZIP. O Chromium vai
//! revalidar tudo no `--load-extension` (ou avisar se algo estiver torto,
//! mas não bloquear, já que estamos em modo dev/load-unpacked).

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};

const CRX_MAGIC: &[u8; 4] = b"Cr24";
const UPDATE_URL_PREFIX: &str = "https://clients2.google.com/service/update2/crx";

/// Garante que a extensão de id `extension_id` existe unpacked em disco e
/// retorna o caminho. Cacheia em `$TMPDIR/zain-chrome-extensions/{id}`.
pub async fn ensure_unpacked(extension_id: &str) -> anyhow::Result<PathBuf> {
    validate_id(extension_id)?;

    let dir = cache_dir().join(extension_id);
    if is_valid_unpacked(&dir) {
        tracing::debug!(extension = extension_id, dir = %dir.display(), "extensão já em cache");
        return Ok(dir);
    }

    tracing::info!(
        extension = extension_id,
        "baixando extensão da Chrome Web Store"
    );
    let crx_bytes = download_crx(extension_id).await?;
    let zip_bytes = strip_crx_header(&crx_bytes)
        .with_context(|| format!("parsing CRX header da extensão {extension_id}"))?;

    // Extrai para um diretório temporário primeiro, depois move atomicamente —
    // evita deixar um diretório meio escrito se algo falhar no meio.
    let parent = cache_dir();
    fs::create_dir_all(&parent).context("criando diretório de cache de extensões")?;
    let staging = parent.join(format!("{extension_id}.staging-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging).ok();
    }
    extract_zip(zip_bytes, &staging)
        .with_context(|| format!("extraindo zip da extensão {extension_id}"))?;

    if dir.exists() {
        fs::remove_dir_all(&dir).ok();
    }
    fs::rename(&staging, &dir).context("movendo staging → cache final")?;

    if !is_valid_unpacked(&dir) {
        anyhow::bail!(
            "extensão {extension_id} extraída mas não tem manifest.json em {}",
            dir.display()
        );
    }

    tracing::info!(
        extension = extension_id,
        dir = %dir.display(),
        "extensão pronta para uso"
    );
    Ok(dir)
}

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("zain-chrome-extensions")
}

fn is_valid_unpacked(dir: &Path) -> bool {
    dir.join("manifest.json").is_file()
}

/// IDs de extensão Chrome são exatamente 32 caracteres ASCII no alfabeto
/// `a..p` (base16 deslocado). Validamos pra evitar path traversal e
/// requests malformados.
fn validate_id(id: &str) -> anyhow::Result<()> {
    if id.len() != 32 || !id.bytes().all(|b| (b'a'..=b'p').contains(&b)) {
        anyhow::bail!("ID de extensão inválido: {id:?}");
    }
    Ok(())
}

async fn download_crx(extension_id: &str) -> anyhow::Result<Vec<u8>> {
    // Parâmetros mínimos esperados pela API. `prodversion` precisa parecer
    // um Chrome moderno; o resto é boilerplate.
    let url = format!(
        "{UPDATE_URL_PREFIX}\
         ?response=redirect\
         &acceptformat=crx2,crx3\
         &prodversion=120.0.0.0\
         &x=id%3D{extension_id}%26installsource%3Dondemand%26uc"
    );

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                    (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .build()
        .context("criando reqwest client")?;

    let resp = client
        .get(&url)
        .send()
        .await
        .context("baixando .crx da Google")?
        .error_for_status()
        .context("Google respondeu com status de erro")?;

    let bytes = resp.bytes().await.context("lendo corpo do .crx")?.to_vec();

    if bytes.len() < 16 || &bytes[..4] != CRX_MAGIC {
        anyhow::bail!(
            "resposta não parece um CRX (magic ausente, {} bytes)",
            bytes.len()
        );
    }
    Ok(bytes)
}

/// Pula o header CRX2/CRX3 e devolve a fatia que contém o ZIP.
fn strip_crx_header(crx: &[u8]) -> anyhow::Result<&[u8]> {
    if crx.len() < 16 || &crx[..4] != CRX_MAGIC {
        anyhow::bail!("magic CRX ausente");
    }
    let version = u32::from_le_bytes(crx[4..8].try_into().unwrap());
    let zip_offset = match version {
        2 => {
            // CRX2: pubkey_len (4) + sig_len (4) + pubkey + sig
            let pubkey_len = u32::from_le_bytes(crx[8..12].try_into().unwrap()) as usize;
            let sig_len = u32::from_le_bytes(crx[12..16].try_into().unwrap()) as usize;
            16usize
                .checked_add(pubkey_len)
                .and_then(|n| n.checked_add(sig_len))
                .ok_or_else(|| anyhow!("CRX2 com tamanhos overflow"))?
        }
        3 => {
            // CRX3: header_len (4) + header bytes (protobuf)
            let header_len = u32::from_le_bytes(crx[8..12].try_into().unwrap()) as usize;
            12usize
                .checked_add(header_len)
                .ok_or_else(|| anyhow!("CRX3 com tamanho overflow"))?
        }
        v => anyhow::bail!("versão CRX desconhecida: {v}"),
    };
    if zip_offset > crx.len() {
        anyhow::bail!(
            "CRX truncado: zip_offset={zip_offset} > total={}",
            crx.len()
        );
    }
    Ok(&crx[zip_offset..])
}

fn extract_zip(zip_bytes: &[u8], dest: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dest).context("criando diretório de destino")?;

    let cursor = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("abrindo zip do CRX")?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("lendo entrada {i} do zip"))?;

        // Sanitiza nome — rejeita absolutos e ".." pra evitar zip-slip.
        let Some(rel) = entry.enclosed_name() else {
            anyhow::bail!("entrada com path inseguro: {}", entry.name());
        };
        let out = dest.join(rel);

        if entry.is_dir() {
            fs::create_dir_all(&out).with_context(|| format!("criando dir {}", out.display()))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("criando parent {}", parent.display()))?;
        }
        let mut writer =
            fs::File::create(&out).with_context(|| format!("criando arquivo {}", out.display()))?;
        std::io::copy(&mut entry, &mut writer)
            .with_context(|| format!("escrevendo arquivo {}", out.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_id_length_and_alphabet() {
        assert!(validate_id("dknlfmjaanfblgfdfebhijalfmhmjjjo").is_ok());
        // tamanho errado
        assert!(validate_id("abc").is_err());
        // caractere fora do alfabeto a..p
        assert!(validate_id("dknlfmjaanfblgfdfebhijalfmhmjjjz").is_err());
        // path traversal
        assert!(validate_id("../etc/passwd").is_err());
    }

    #[test]
    fn strip_crx3_header() {
        // Mock CRX3: magic + version=3 + header_len=4 + 4 bytes header + zip data
        let mut crx = Vec::new();
        crx.extend_from_slice(CRX_MAGIC);
        crx.extend_from_slice(&3u32.to_le_bytes());
        crx.extend_from_slice(&4u32.to_le_bytes());
        crx.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // header
        crx.extend_from_slice(b"PKfake-zip-content"); // pretend zip
        let zip = strip_crx_header(&crx).unwrap();
        assert_eq!(zip, b"PKfake-zip-content");
    }

    #[test]
    fn strip_crx2_header() {
        // CRX2: magic + version=2 + pubkey_len=3 + sig_len=2 + pubkey + sig + zip
        let mut crx = Vec::new();
        crx.extend_from_slice(CRX_MAGIC);
        crx.extend_from_slice(&2u32.to_le_bytes());
        crx.extend_from_slice(&3u32.to_le_bytes());
        crx.extend_from_slice(&2u32.to_le_bytes());
        crx.extend_from_slice(&[1, 2, 3]); // pubkey
        crx.extend_from_slice(&[4, 5]); // sig
        crx.extend_from_slice(b"PKzip");
        let zip = strip_crx_header(&crx).unwrap();
        assert_eq!(zip, b"PKzip");
    }

    #[test]
    fn rejects_truncated_crx() {
        let crx = b"Cr24XX"; // sem nem version completo
        assert!(strip_crx_header(crx).is_err());
    }

    #[test]
    fn rejects_unknown_version() {
        let mut crx = Vec::new();
        crx.extend_from_slice(CRX_MAGIC);
        crx.extend_from_slice(&99u32.to_le_bytes());
        crx.extend_from_slice(&[0; 8]);
        assert!(strip_crx_header(&crx).is_err());
    }
}
