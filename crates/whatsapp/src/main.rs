use cubos_sql::sql;
use deadpool_postgres::{Config, Runtime};
use tokio_postgres::NoTls;

use whatsapp::client::WhapiClient;
use whatsapp::types::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/pjtei".into());
    let whapi_token = std::env::var("WHAPI_TOKEN").expect("WHAPI_TOKEN não definido");
    let whapi_base_url =
        std::env::var("WHAPI_BASE_URL").unwrap_or_else(|_| "https://gate.whapi.cloud".into());

    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    let api = WhapiClient::new(&whapi_base_url, &whapi_token);

    tracing::info!("Iniciando sync loop...");

    loop {
        if let Err(e) = sync_all(&pool, &api).await {
            tracing::error!("Erro no sync: {e:#}");
        }

        tracing::info!("Aguardando 60s para próximo ciclo...");
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn sync_all(pool: &deadpool_postgres::Pool, api: &WhapiClient) -> anyhow::Result<()> {
    let page_size = 500i32;
    let mut offset = 0i32;
    let mut chats_processed = 0;
    let mut chats_synced = 0;

    'pagination: loop {
        let page = api.get_chats(offset, page_size).await?;
        tracing::info!(
            offset,
            count = page.chats.len(),
            total = page.total,
            "Chats recebidos"
        );

        if page.chats.is_empty() {
            break;
        }

        for chat in &page.chats {
            if chat.id == "0@s.whatsapp.net" {
                continue;
            }

            let chat_id = &chat.id;
            let chat_timestamp = chat.timestamp;

            // Buscar timestamp salvo no banco
            let existing = sql!(
                pool,
                "SELECT timestamp FROM whatsapp.chats WHERE id = $chat_id"
            )
            .fetch_optional()
            .await?;

            let saved_ts: Option<i64> = existing.as_ref().and_then(|r| r.timestamp);

            let is_new = existing.is_none();
            let is_updated = !is_new
                && match (chat_timestamp, saved_ts) {
                    (Some(api_ts), Some(db_ts)) => api_ts > db_ts,
                    (Some(_), None) => true,
                    _ => false,
                };

            // Chats vêm ordenados por timestamp desc. Se o chat já existe
            // e não foi atualizado, todos os seguintes também não foram.
            if !is_new && !is_updated {
                tracing::info!(chat_id, "Chat não atualizado, parando paginação de chats");
                break 'pagination;
            }

            // Upsert do chat
            let name = chat.name.clone();
            let chat_type = chat.chat_type.clone().unwrap_or_else(|| "unknown".into());
            let chat_pic = chat.chat_pic.clone();
            let pin = chat.pin.unwrap_or(false);
            let mute = chat.mute.unwrap_or(false);
            let archive = chat.archive.unwrap_or(false);
            let unread = chat.unread.unwrap_or(0);
            let read_only = chat.read_only.unwrap_or(false);
            let last_message_id = chat.last_message.as_ref().map(|m| m.id.clone());

            sql!(
                pool,
                "INSERT INTO whatsapp.chats (id, name, type, timestamp, chat_pic, pin, mute, archive, unread, read_only, last_message_id, updated_at)
                 VALUES ($chat_id, $name, $chat_type, $chat_timestamp, $chat_pic, $pin, $mute, $archive, $unread, $read_only, $last_message_id, now())
                 ON CONFLICT (id) DO UPDATE SET
                    name = $name,
                    type = $chat_type,
                    timestamp = $chat_timestamp,
                    chat_pic = $chat_pic,
                    pin = $pin,
                    mute = $mute,
                    archive = $archive,
                    unread = $unread,
                    read_only = $read_only,
                    last_message_id = $last_message_id,
                    updated_at = now()"
            )
            .execute()
            .await?;

            chats_processed += 1;

            // Sincronizar mensagens
            let chat_name = chat.name.as_deref().unwrap_or(chat_id);
            tracing::info!(
                chat_id,
                chat_name,
                is_new,
                "Sincronizando mensagens do chat"
            );

            if let Err(e) = sync_messages(pool, api, chat_id, saved_ts).await {
                tracing::error!(chat_id, "Erro sincronizando mensagens: {e:#}");
            } else {
                chats_synced += 1;
            }
        }

        offset += page_size;
        if offset >= page.total {
            break;
        }
    }

    tracing::info!(chats_processed, chats_synced, "Sync completo");
    Ok(())
}

async fn sync_messages(
    pool: &deadpool_postgres::Pool,
    api: &WhapiClient,
    chat_id: &str,
    last_known_ts: Option<i64>,
) -> anyhow::Result<()> {
    let page_size = 500i32;

    // Se já temos mensagens, buscar apenas as mais novas (a partir do último timestamp conhecido).
    // Se é um chat novo, buscar tudo (sem filtro de tempo).
    match last_known_ts {
        Some(ts) => {
            // Buscar mensagens a partir do timestamp conhecido, em ordem ascendente.
            // Usamos ts (não ts+1) pois podem haver mensagens no mesmo segundo.
            let mut offset = 0i32;
            loop {
                let page = api
                    .get_messages_since(chat_id, ts, offset, page_size)
                    .await?;
                tracing::debug!(
                    chat_id,
                    offset,
                    count = page.messages.len(),
                    total = page.total,
                    "Mensagens recebidas (incremental)"
                );

                for msg in &page.messages {
                    save_message(pool, msg).await?;
                }

                offset += page_size;
                if offset >= page.total {
                    break;
                }
            }
        }
        None => {
            // Chat novo: buscar todas as mensagens (desc por padrão)
            let mut offset = 0i32;
            loop {
                let page = api.get_messages(chat_id, offset, page_size).await?;
                tracing::debug!(
                    chat_id,
                    offset,
                    count = page.messages.len(),
                    total = page.total,
                    "Mensagens recebidas (full)"
                );

                for msg in &page.messages {
                    save_message(pool, msg).await?;
                }

                offset += page_size;
                if offset >= page.total {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn save_message(pool: &deadpool_postgres::Pool, msg: &Message) -> anyhow::Result<()> {
    let id = &msg.id;
    let chat_id = &msg.chat_id;
    let msg_type = &msg.msg_type;
    let subtype = msg.subtype.as_deref();
    let from_number = msg.from.as_deref();
    let from_me = msg.from_me;
    let from_name = msg.from_name.as_deref();
    let timestamp = msg.timestamp.map(|t| t as i64);
    let source = msg.source.as_deref();
    let status = msg.status.as_deref();
    let text_body = msg.text_body();
    let has_media = msg.has_media();
    let media_mime = msg.media_mime();
    let media_url = msg.media_url();
    let context_quoted_id = msg.context.as_ref().and_then(|c| c.quoted_id.as_deref());
    let context_forwarded = msg.context.as_ref().and_then(|c| c.forwarded);
    let raw = serde_json::to_value(msg)?;

    sql!(
        pool,
        "INSERT INTO whatsapp.messages (id, chat_id, type, subtype, from_number, from_me, from_name, timestamp, source, status, text_body, has_media, media_mime, media_url, context_quoted_id, context_forwarded, raw)
         VALUES ($id, $chat_id, $msg_type, $subtype, $from_number, $from_me, $from_name, $timestamp, $source, $status, $text_body, $has_media, $media_mime, $media_url, $context_quoted_id, $context_forwarded, $raw)
         ON CONFLICT (id) DO UPDATE SET
            status = $status,
            raw = $raw"
    )
    .execute()
    .await?;

    Ok(())
}
