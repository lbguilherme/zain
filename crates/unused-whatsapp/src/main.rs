use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chromium_driver::LaunchOptions;
use chrono::{Local, Timelike};
use cubos_sql::sql;
use deadpool_postgres::{Config, Runtime};
use tokio_postgres::NoTls;
use unused_whatsapp::{WhatsAppClient, WhatsAppOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();
    tracing_subscriber::fmt::init();

    // --- Database setup ---
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/pjtei".into());

    // Connection pool
    let mut pool_cfg = Config::new();
    pool_cfg.url = Some(database_url);
    let pool = pool_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    // --- Load accounts ---
    let accounts = sql!(&pool, "SELECT id, name, phone FROM whatsapp.accounts")
        .fetch_all()
        .await?;

    if accounts.is_empty() {
        tracing::warn!("No accounts in whatsapp.accounts — nothing to do.");
        return Ok(());
    }

    tracing::info!("Starting {} WhatsApp session(s)...", accounts.len());

    let mut handles = Vec::new();

    for account in accounts {
        let pool = pool.clone();
        let account_id: uuid::Uuid = account.id;
        let account_dir = PathBuf::from(format!(".whatsapp/{account_id}"));

        let handle = tokio::spawn(async move {
            if let Err(e) = run_account(pool, account_id, account_dir).await {
                tracing::error!(account_id = %account_id, "session failed: {e:#}");
            }
        });

        handles.push(handle);
    }

    // Wait forever (Ctrl+C to stop)
    tracing::info!("All sessions started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}

async fn run_account(
    pool: deadpool_postgres::Pool,
    account_id: uuid::Uuid,
    account_dir: PathBuf,
) -> anyhow::Result<()> {
    let data_dir = account_dir.join("data");
    let media_dir = account_dir.join("media");
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&media_dir)?;

    let client = WhatsAppClient::launch(WhatsAppOptions {
        launch: LaunchOptions {
            headless: false,
            user_data_dir: Some(data_dir.to_string_lossy().into_owned()),
            ..Default::default()
        },
        ..Default::default()
    })
    .await?;

    let session = client
        .authenticate(|qr_data| {
            eprintln!("\n[{account_id}] Scan QR code:\n");
            qr2term::print_qr(qr_data).unwrap();
            eprintln!();
        })
        .await?;

    tracing::info!(account_id = %account_id, "Authenticated. Starting sync loop.");

    // --- Sync loop ---
    loop {
        // Update profile if stale (>1h since last update).
        if let Err(e) = maybe_sync_profile(&session, &pool, account_id, &media_dir).await {
            tracing::error!(account_id = %account_id, "profile sync failed: {e:#}");
        }

        if let Err(e) = sync_chats(&session, &pool, account_id, &media_dir).await {
            tracing::error!(account_id = %account_id, "sync_chats failed: {e:#}");
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

async fn maybe_sync_profile(
    session: &unused_whatsapp::WhatsAppSession,
    pool: &deadpool_postgres::Pool,
    account_id: uuid::Uuid,
    media_dir: &Path,
) -> anyhow::Result<()> {
    let updated_at = sql!(
        pool,
        "SELECT updated_at FROM whatsapp.accounts WHERE id = $account_id"
    )
    .fetch_value()
    .await?;

    let stale = chrono::Utc::now() - updated_at > chrono::Duration::hours(1);

    if !stale {
        return Ok(());
    }

    tracing::info!(account_id = %account_id, "Profile stale, refreshing...");

    let profile = session.profile().await?;

    tracing::info!(
        account_id = %account_id,
        name = %profile.name,
        phone = %profile.phone,
        "Profile loaded"
    );

    let avatar = match &profile.avatar_url {
        Some(url) => download_media(media_dir, url).await.ok(),
        None => None,
    };

    let name = Some(profile.name.clone());
    let phone = Some(profile.phone.clone());
    sql!(
        pool,
        "UPDATE whatsapp.accounts
            SET name = $name,
                phone = $phone,
                avatar = $avatar,
                updated_at = now()
          WHERE id = $account_id"
    )
    .execute()
    .await?;

    tracing::info!(account_id = %account_id, "Profile updated");
    Ok(())
}

async fn sync_chats(
    session: &unused_whatsapp::WhatsAppSession,
    pool: &deadpool_postgres::Pool,
    account_id: uuid::Uuid,
    media_dir: &Path,
) -> anyhow::Result<()> {
    // Ensure we're on the chat list screen.
    session.navigate_to_chats().await?;

    let since = (Local::now() - chrono::Duration::days(7)).date_naive();
    let cutoff = since.and_hms_opt(0, 0, 0).unwrap();
    let previews = session.get_chats(since).await?;

    tracing::info!(count = previews.len(), "Scanned chat sidebar");

    for preview in &previews {
        let title = preview.title.clone();
        let displayed_last_message = if preview.last_message.is_empty() {
            None
        } else {
            Some(preview.last_message.clone())
        };
        let displayed_timestamp = preview.timestamp.map(|ts| {
            ts.and_local_timezone(Local)
                .single()
                .unwrap_or_else(Local::now)
                .with_timezone(&chrono::Utc)
        });

        // Check if this chat already exists with the same displayed info.
        let select_title = title.clone();
        let existing = sql!(
            pool,
            "SELECT id, chat_jid, displayed_last_message, displayed_timestamp
               FROM whatsapp.chats
              WHERE account_id = $account_id
                AND title = $select_title
              ORDER BY updated_at DESC
              LIMIT 1"
        )
        .fetch_optional()
        .await?;

        let chat_unchanged = existing.as_ref().is_some_and(|c| {
            c.displayed_last_message == displayed_last_message
                && c.displayed_timestamp == displayed_timestamp
        });

        if chat_unchanged {
            tracing::debug!(title = %title, "Chat unchanged, skipping");
            continue;
        }

        // Open the chat.
        tracing::info!(title = %title, "Opening chat...");
        if let Err(e) = session.open_chat(&title).await {
            tracing::warn!(title = %title, "Failed to open chat: {e:#}");
            let _ = session.navigate_to_chats().await;
            continue;
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = session.scroll_to_bottom().await;

        // Read messages, scrolling up until we hit already-saved or old messages.
        // Messages are read in DOM order (oldest at top, newest at bottom).
        // We iterate in REVERSE (newest first) so the cutoff/dedup check works
        // correctly — we collect recent messages and stop when we hit old ones.
        let mut all_new: Vec<unused_whatsapp::RawMessage> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut stop = false;
        let mut no_new_rounds = 0;

        loop {
            let mut msgs = match session.read_messages(media_dir).await {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(title = %title, "Failed to read messages: {e:#}");
                    break;
                }
            };

            tracing::debug!(title = %title, visible = msgs.len(), "Read visible messages");

            // Reverse: process newest first so we stop at old/existing msgs.
            msgs.reverse();

            let mut found_new = false;
            for msg in msgs {
                if seen_ids.contains(&msg.data_id.message_id) {
                    continue;
                }
                seen_ids.insert(msg.data_id.message_id.clone());

                tracing::debug!(
                    msg_id = %msg.data_id.message_id,
                    msg_type = msg.msg_type.as_str(),
                    timestamp = ?msg.timestamp,
                    sender = ?msg.sender_name,
                    text_len = msg.text.as_deref().map(str::len).unwrap_or(0),
                    "Processing message"
                );

                // Check if already in DB.
                let msg_id_check = msg.data_id.message_id.clone();
                let exists = sql!(
                    pool,
                    "SELECT id FROM whatsapp.messages
                      WHERE account_id = $account_id AND message_id = $msg_id_check
                      LIMIT 1"
                )
                .fetch_optional()
                .await?;

                if exists.is_some() {
                    tracing::debug!(msg_id = %msg.data_id.message_id, "Found existing message, stopping");
                    stop = true;
                    break;
                }

                // Check if too old.
                if let Some(ts) = msg.timestamp
                    && ts < cutoff
                {
                    tracing::debug!(timestamp = %ts, cutoff = %cutoff, "Message older than 24h, stopping");
                    stop = true;
                    break;
                }

                found_new = true;
                all_new.push(msg);
            }

            if stop {
                break;
            }
            if !found_new {
                no_new_rounds += 1;
                if no_new_rounds >= 3 {
                    tracing::debug!(title = %title, "No new messages after 3 rounds, stopping");
                    break;
                }
            } else {
                no_new_rounds = 0;
            }

            // Scroll up to load older messages.
            if let Err(e) = session.scroll_up_messages().await {
                tracing::warn!(title = %title, "Failed to scroll up: {e:#}");
                break;
            }
        }

        // Messages in all_new are in reverse DOM order (newest first) since
        // we reversed during collection. Put back in DOM order (oldest first)
        // for timestamp assignment.
        all_new.reverse();

        tracing::info!(title = %title, collected = all_new.len(), "Finished reading messages");

        // Assign precise timestamps using microseconds for ordering within
        // the same minute. System messages without timestamps inherit from
        // the previous (older) message.
        assign_ordered_timestamps(&mut all_new);

        // Extract chat_jid from first message if available.
        let chat_jid: Option<String> = all_new.first().map(|m| m.data_id.chat_jid.clone());
        let chat_type = chat_jid
            .as_deref()
            .map(|jid| {
                if jid.contains("@g.us") {
                    "group"
                } else {
                    "person"
                }
            })
            .unwrap_or("unknown")
            .to_owned();

        // Save chat + messages in a single transaction.
        let mut client = pool.get().await?;
        let tx = client.transaction().await?;

        let chat_db_id: uuid::Uuid = if let Some(c) = &existing {
            let chat_id: uuid::Uuid = c.id;
            let update_chat_jid = chat_jid.clone().or_else(|| c.chat_jid.clone());
            let update_chat_type = chat_type.clone();
            sql!(
                &tx,
                "UPDATE whatsapp.chats
                    SET displayed_last_message = $displayed_last_message,
                        displayed_timestamp = $displayed_timestamp,
                        chat_jid = $update_chat_jid,
                        chat_type = $update_chat_type,
                        updated_at = now()
                  WHERE id = $chat_id"
            )
            .execute()
            .await?;
            chat_id
        } else {
            let insert_title = title.clone();
            let insert_chat_jid = chat_jid.clone();
            let insert_chat_type = chat_type.clone();

            sql!(
                &tx,
                "INSERT INTO whatsapp.chats (account_id, title, displayed_last_message, displayed_timestamp, chat_jid, chat_type)
                 VALUES ($account_id, $insert_title, $displayed_last_message, $displayed_timestamp, $insert_chat_jid, $insert_chat_type)
                 RETURNING id"
            )
            .fetch_value()
            .await?
        };

        for msg in &all_new {
            let chat_id = chat_db_id;
            let raw_id = msg.data_id.raw.clone();
            let message_id = msg.data_id.message_id.clone();
            let msg_type = msg.msg_type.as_str().to_owned();
            let is_from_me = msg.data_id.outgoing;
            let text = msg.text.clone();
            let sender_jid = msg.sender_jid.clone();
            let sender_name = msg.sender_name.clone();
            let sticker_media = msg.sticker_media.clone();
            let image_media = msg.image_media.clone();
            let timestamp = msg.timestamp.map(|ts| {
                ts.and_local_timezone(Local)
                    .single()
                    .unwrap_or_else(Local::now)
                    .with_timezone(&chrono::Utc)
            });
            sql!(
                &tx,
                "INSERT INTO whatsapp.messages (chat_id, account_id, raw_id, message_id, type, is_from_me, text, sender_jid, sender_name, sticker_media, image_media, timestamp)
                 VALUES ($chat_id, $account_id, $raw_id, $message_id, $msg_type, $is_from_me, $text, $sender_jid, $sender_name, $sticker_media, $image_media, $timestamp)
                 ON CONFLICT (account_id, message_id) DO NOTHING"
            )
            .execute()
            .await?;
        }

        tx.commit().await?;
        tracing::info!(title = %title, new_msgs = all_new.len(), "{}", if existing.is_some() { "Chat updated" } else { "Chat created" });
    }

    tracing::info!("Chat sync complete");
    Ok(())
}

/// Assigns ordered timestamps to messages using microseconds for ordering.
///
/// Messages are expected in DOM order (oldest first). Within the same minute,
/// the first message gets `:00.000000`, the next `:00.000001`, etc.
/// System messages without timestamps inherit from the previous message
/// (or epoch if first).
fn assign_ordered_timestamps(msgs: &mut [unused_whatsapp::RawMessage]) {
    let epoch = chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc();
    let mut last_ts = epoch;

    // First pass: fill in missing timestamps (system messages).
    for msg in msgs.iter_mut() {
        if msg.timestamp.is_none() {
            msg.timestamp = Some(last_ts);
        } else {
            last_ts = msg.timestamp.unwrap();
        }
    }

    // Second pass: assign microsecond ordering within each minute.
    // Group messages by their truncated minute (YYYY-MM-DD HH:MM).
    let mut i = 0;
    while i < msgs.len() {
        let base = msgs[i].timestamp.unwrap();
        let base_minute = base
            .with_second(0)
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(base);

        // Find the end of this minute group.
        let mut j = i + 1;
        while j < msgs.len() {
            let ts = msgs[j].timestamp.unwrap();
            let ts_minute = ts
                .with_second(0)
                .and_then(|t| t.with_nanosecond(0))
                .unwrap_or(ts);
            if ts_minute != base_minute {
                break;
            }
            j += 1;
        }

        // Assign microsecond offsets within this group.
        for (offset, msg) in msgs[i..j].iter_mut().enumerate() {
            let micros = offset as u32;
            msg.timestamp = Some(
                base_minute
                    .with_nanosecond(micros * 1000)
                    .unwrap_or(base_minute),
            );
        }

        i = j;
    }
}

/// Extracts the filename from a WhatsApp CDN URL.
fn filename_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let name = parsed.path_segments()?.next_back()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Downloads a file to `media_dir/{filename}` if it doesn't already exist.
async fn download_media(media_dir: &Path, url: &str) -> anyhow::Result<String> {
    let filename = filename_from_url(url)
        .ok_or_else(|| anyhow::anyhow!("could not extract filename from URL"))?;
    let path = media_dir.join(&filename);
    if path.exists() {
        return Ok(filename);
    }

    tracing::info!(filename, "Downloading media...");
    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    std::fs::write(&path, &bytes)?;
    Ok(filename)
}
