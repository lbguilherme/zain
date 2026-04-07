use std::time::Duration;

use anyhow::{Context as _, bail};
use cubos_sql::sql;
use deadpool_postgres::Pool;
use serde::Deserialize;
use tokio_postgres::Transaction;

use crate::domains;

// ── Event header (sem deny_unknown_fields, só para peek) ───────────────

#[derive(Deserialize)]
struct EventHeader {
    event: EventMeta,
    #[allow(dead_code)]
    channel_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventMeta {
    #[serde(rename = "type")]
    event_type: String,
    event: String,
}

// ── Event payloads (um struct por shape de payload) ────────────────────

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MessagesEvent {
    event: EventMeta,
    channel_id: String,
    messages: Vec<MessageData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MessagesDeleteEvent {
    event: EventMeta,
    channel_id: String,
    #[serde(default)]
    messages_removed: Option<Vec<String>>,
    #[serde(default)]
    messages_removed_all: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MessagesUpdateEvent {
    event: EventMeta,
    channel_id: String,
    messages_updates: Vec<MessageUpdateData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusesEvent {
    event: EventMeta,
    channel_id: String,
    statuses: Vec<StatusData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatsEvent {
    event: EventMeta,
    channel_id: String,
    chats: Vec<ChatData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatsDeleteEvent {
    event: EventMeta,
    channel_id: String,
    chats_removed: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatsUpdateEvent {
    event: EventMeta,
    channel_id: String,
    chats_updates: Vec<ChatUpdateData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactsEvent {
    event: EventMeta,
    channel_id: String,
    contacts: Vec<ContactData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactsUpdateEvent {
    event: EventMeta,
    channel_id: String,
    contacts_updates: Vec<ContactUpdateData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupsEvent {
    event: EventMeta,
    channel_id: String,
    groups: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupsParticipantsEvent {
    event: EventMeta,
    channel_id: String,
    groups_participants: Vec<ParticipantEventData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupsUpdateEvent {
    event: EventMeta,
    channel_id: String,
    groups_updates: Vec<GroupUpdateData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresencesEvent {
    event: EventMeta,
    channel_id: String,
    presences: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelHealthEvent {
    event: EventMeta,
    channel_id: String,
    health: domains::HealthData,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelPatchEvent {
    event: EventMeta,
    channel_id: String,
    qr: domains::QrData,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsersEvent {
    event: EventMeta,
    channel_id: String,
    user: ContactData,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelsEvent {
    event: EventMeta,
    channel_id: String,
    labels: Vec<LabelData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelsDeleteEvent {
    event: EventMeta,
    channel_id: String,
    labels_removed: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CallsEvent {
    event: EventMeta,
    channel_id: String,
    calls: Vec<serde_json::Value>,
}

// ── Data types ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageData {
    id: String,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    subtype: Option<String>,
    chat_id: String,
    #[serde(default)]
    chat_name: Option<String>,
    #[serde(default)]
    from: Option<String>,
    from_me: bool,
    #[serde(default)]
    from_name: Option<String>,
    #[serde(default)]
    source: Option<String>,
    timestamp: i64,
    #[serde(default)]
    device_id: Option<i64>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    text: Option<TextContent>,
    #[serde(default)]
    image: Option<domains::ImageContent>,
    #[serde(default)]
    video: Option<domains::VideoContent>,
    #[serde(default)]
    short: Option<domains::VideoContent>,
    #[serde(default)]
    gif: Option<domains::VideoContent>,
    #[serde(default)]
    audio: Option<domains::AudioContent>,
    #[serde(default)]
    voice: Option<domains::VoiceContent>,
    #[serde(default)]
    document: Option<domains::DocumentContent>,
    #[serde(default)]
    sticker: Option<domains::StickerContent>,
    #[serde(default)]
    location: Option<domains::LocationContent>,
    #[serde(default)]
    live_location: Option<domains::LiveLocationContent>,
    #[serde(default)]
    contact: Option<domains::ContactMsgContent>,
    #[serde(default)]
    contact_list: Option<domains::ContactListContent>,
    #[serde(default)]
    link_preview: Option<domains::LinkPreviewContent>,
    #[serde(default)]
    group_invite: Option<domains::LinkPreviewContent>,
    #[serde(default)]
    newsletter_invite: Option<domains::LinkPreviewContent>,
    #[serde(default)]
    catalog: Option<domains::LinkPreviewContent>,
    #[serde(default)]
    interactive: Option<serde_json::Value>,
    #[serde(default)]
    poll: Option<serde_json::Value>,
    #[serde(default)]
    hsm: Option<serde_json::Value>,
    #[serde(default)]
    system: Option<serde_json::Value>,
    #[serde(default)]
    order: Option<serde_json::Value>,
    #[serde(default)]
    admin_invite: Option<serde_json::Value>,
    #[serde(default)]
    product: Option<serde_json::Value>,
    #[serde(default)]
    product_items: Option<serde_json::Value>,
    #[serde(default)]
    event: Option<serde_json::Value>,
    #[serde(default)]
    list: Option<serde_json::Value>,
    #[serde(default)]
    buttons: Option<serde_json::Value>,
    #[serde(default)]
    action: Option<domains::MessageAction>,
    #[serde(default)]
    context: Option<domains::MessageContext>,
    #[serde(default)]
    reactions: Option<serde_json::Value>,
    #[serde(default)]
    labels: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextContent {
    pub body: String,
    #[serde(default)]
    pub buttons: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub sections: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub view_once: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageUpdateData {
    id: String,
    #[serde(default)]
    trigger: Option<serde_json::Value>,
    before_update: serde_json::Value,
    after_update: serde_json::Value,
    changes: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusData {
    id: String,
    #[serde(default)]
    code: Option<i32>,
    status: String,
    #[serde(default)]
    recipient_id: Option<String>,
    #[serde(default)]
    viewer_id: Option<String>,
    timestamp: serde_json::Value,
    #[serde(default)]
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatData {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    chat_type: String,
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default)]
    chat_pic: Option<String>,
    #[serde(default)]
    chat_pic_full: Option<String>,
    #[serde(default)]
    pin: Option<bool>,
    #[serde(default)]
    mute: Option<bool>,
    #[serde(default)]
    mute_until: Option<i64>,
    #[serde(default)]
    archive: Option<bool>,
    #[serde(default)]
    unread: Option<i32>,
    #[serde(default)]
    unread_mention: Option<bool>,
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    not_spam: Option<bool>,
    #[serde(default)]
    last_message: Option<serde_json::Value>,
    #[serde(default)]
    labels: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatUpdateData {
    before_update: serde_json::Value,
    after_update: serde_json::Value,
    changes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactData {
    id: String,
    name: String,
    #[serde(default)]
    pushname: Option<String>,
    #[serde(default)]
    is_business: Option<bool>,
    #[serde(default)]
    profile_pic: Option<String>,
    #[serde(default)]
    profile_pic_full: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    saved: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactUpdateData {
    before_update: serde_json::Value,
    after_update: serde_json::Value,
    changes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelData {
    id: String,
    name: String,
    color: String,
    #[serde(default)]
    count: Option<i32>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParticipantEventData {
    group_id: String,
    participants: Vec<String>,
    #[serde(default)]
    action: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupUpdateData {
    before_update: serde_json::Value,
    after_update: serde_json::Value,
    changes: Vec<String>,
}

// ── Loop principal ─────────────────────────────────────────────────────

pub async fn event_processor_loop(pool: &Pool) -> anyhow::Result<()> {
    loop {
        match process_batch(pool).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(n, "Eventos webhook processados"),
            Err(e) => tracing::error!("Erro no event processor: {e:#}"),
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn process_batch(pool: &Pool) -> anyhow::Result<usize> {
    let mut client = pool.get().await?;

    let has_events = sql!(
        &client,
        "SELECT EXISTS(SELECT 1 FROM whatsapp.webhook_events WHERE processed = false)"
    )
    .fetch_value()
    .await?;

    if !has_events {
        return Ok(0);
    }

    let tx = client.transaction().await?;

    let rows = sql!(
        &tx,
        "SELECT id, body FROM whatsapp.webhook_events
         WHERE processed = false
         ORDER BY created_at ASC
         LIMIT 100
         FOR UPDATE"
    )
    .fetch_all()
    .await?;

    let mut processed = 0usize;

    for row in &rows {
        let event_id = row.id;
        let body = &row.body;

        match process_event(&tx, body).await {
            Ok(()) => {
                sql!(
                    &tx,
                    "UPDATE whatsapp.webhook_events SET processed = true WHERE id = $event_id"
                )
                .execute()
                .await?;
                processed += 1;
            }
            Err(e) => {
                tracing::error!(%event_id, "Erro processando evento: {e:#}");
                break;
            }
        }
    }

    tx.commit().await?;
    Ok(processed)
}

// ── Dispatch por tipo de evento ────────────────────────────────────────

async fn process_event(tx: &Transaction<'_>, body: &serde_json::Value) -> anyhow::Result<()> {
    let header: EventHeader =
        serde_json::from_value(body.clone()).context("Falha ao ler cabeçalho do evento")?;

    let et = header.event.event_type.as_str();
    let ea = header.event.event.as_str();

    match (et, ea) {
        // ── Messages ───────────────────────────────────────────────
        ("messages", "post") | ("messages", "put") => {
            let evt: MessagesEvent =
                serde_json::from_value(body.clone()).context("messages.post/put")?;
            for msg in &evt.messages {
                upsert_message(tx, &evt.channel_id, msg).await?;
            }
            Ok(())
        }
        ("messages", "delete") => {
            let evt: MessagesDeleteEvent =
                serde_json::from_value(body.clone()).context("messages.delete")?;
            let channel_id = &evt.channel_id;
            if let Some(ids) = &evt.messages_removed {
                for id in ids {
                    sql!(
                        tx,
                        "DELETE FROM whatsapp.messages WHERE channel_id = $channel_id AND id = $id"
                    )
                    .execute()
                    .await?;
                }
            }
            if let Some(chat_id) = &evt.messages_removed_all {
                sql!(
                    tx,
                    "DELETE FROM whatsapp.messages
                     WHERE channel_id = $channel_id AND chat_id = $chat_id"
                )
                .execute()
                .await?;
            }
            Ok(())
        }
        ("messages", "patch") => {
            let evt: MessagesUpdateEvent =
                serde_json::from_value(body.clone()).context("messages.patch")?;
            for upd in &evt.messages_updates {
                let msg: MessageData = serde_json::from_value(upd.after_update.clone()).context(
                    format!("messages.patch: after_update da mensagem {}", upd.id),
                )?;
                upsert_message(tx, &evt.channel_id, &msg).await?;
            }
            Ok(())
        }

        // ── Statuses ───────────────────────────────────────────────
        ("statuses", "post") | ("statuses", "put") => {
            let evt: StatusesEvent = serde_json::from_value(body.clone()).context("statuses")?;
            let channel_id = &evt.channel_id;
            for s in &evt.statuses {
                let message_id = &s.id;
                let status = &s.status;
                let status_code = s.code;
                let recipient_id = s.recipient_id.as_deref();
                let viewer_id = s.viewer_id.as_deref();
                let timestamp = match &s.timestamp {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                };
                sql!(
                    tx,
                    "INSERT INTO whatsapp.statuses
                         (channel_id, message_id, status, status_code,
                          recipient_id, viewer_id, \"timestamp\")
                     VALUES ($channel_id, $message_id, $status, $status_code,
                             $recipient_id, $viewer_id, $timestamp)
                     ON CONFLICT (channel_id, message_id) DO UPDATE SET
                         status = EXCLUDED.status,
                         status_code = EXCLUDED.status_code,
                         recipient_id = EXCLUDED.recipient_id,
                         viewer_id = EXCLUDED.viewer_id,
                         \"timestamp\" = EXCLUDED.\"timestamp\""
                )
                .execute()
                .await?;
            }
            Ok(())
        }

        // ── Chats ──────────────────────────────────────────────────
        ("chats", "post") | ("chats", "put") => {
            let evt: ChatsEvent = serde_json::from_value(body.clone()).context("chats.post/put")?;
            for chat in &evt.chats {
                upsert_chat(tx, &evt.channel_id, chat).await?;
            }
            Ok(())
        }
        ("chats", "delete") => {
            let evt: ChatsDeleteEvent =
                serde_json::from_value(body.clone()).context("chats.delete")?;
            let channel_id = &evt.channel_id;
            for id in &evt.chats_removed {
                sql!(
                    tx,
                    "DELETE FROM whatsapp.chats WHERE channel_id = $channel_id AND id = $id"
                )
                .execute()
                .await?;
            }
            Ok(())
        }
        ("chats", "patch") => {
            let evt: ChatsUpdateEvent =
                serde_json::from_value(body.clone()).context("chats.patch")?;
            let channel_id = &evt.channel_id;
            for upd in &evt.chats_updates {
                let after = upd.after_update.clone();
                let id = after
                    .get("id")
                    .and_then(|v| v.as_str())
                    .context("chats.patch: missing id")?
                    .to_owned();
                sql!(
                    tx,
                    "UPDATE whatsapp.chats SET
                        name = CASE WHEN $after ? 'name' THEN $after->>'name' ELSE name END,
                        chat_type = CASE WHEN $after ? 'type' THEN $after->>'type' ELSE chat_type END,
                        \"timestamp\" = CASE WHEN $after ? 'timestamp' THEN ($after->>'timestamp')::bigint ELSE \"timestamp\" END,
                        chat_pic = CASE WHEN $after ? 'chat_pic' THEN $after->>'chat_pic' ELSE chat_pic END,
                        chat_pic_full = CASE WHEN $after ? 'chat_pic_full' THEN $after->>'chat_pic_full' ELSE chat_pic_full END,
                        pin = CASE WHEN $after ? 'pin' THEN ($after->>'pin')::boolean ELSE pin END,
                        mute = CASE WHEN $after ? 'mute' THEN ($after->>'mute')::boolean ELSE mute END,
                        mute_until = CASE WHEN $after ? 'mute_until' THEN ($after->>'mute_until')::bigint ELSE mute_until END,
                        archive = CASE WHEN $after ? 'archive' THEN ($after->>'archive')::boolean ELSE archive END,
                        unread = CASE WHEN $after ? 'unread' THEN ($after->>'unread')::integer ELSE unread END,
                        unread_mention = CASE WHEN $after ? 'unread_mention' THEN ($after->>'unread_mention')::boolean ELSE unread_mention END,
                        read_only = CASE WHEN $after ? 'read_only' THEN ($after->>'read_only')::boolean ELSE read_only END,
                        not_spam = CASE WHEN $after ? 'not_spam' THEN ($after->>'not_spam')::boolean ELSE not_spam END,
                        last_message = CASE WHEN $after ? 'last_message' THEN ($after->'last_message') ELSE last_message END,
                        labels = CASE WHEN $after ? 'labels' THEN ($after->'labels') ELSE labels END
                    WHERE channel_id = $channel_id AND id = $id"
                )
                .execute()
                .await?;
            }
            Ok(())
        }

        // ── Contacts ───────────────────────────────────────────────
        ("contacts", "post") => {
            let evt: ContactsEvent =
                serde_json::from_value(body.clone()).context("contacts.post")?;
            for c in &evt.contacts {
                upsert_contact(tx, &evt.channel_id, c).await?;
            }
            Ok(())
        }
        ("contacts", "patch") => {
            let evt: ContactsUpdateEvent =
                serde_json::from_value(body.clone()).context("contacts.patch")?;
            let channel_id = &evt.channel_id;
            for upd in &evt.contacts_updates {
                let after = upd.after_update.clone();
                let id = after
                    .get("id")
                    .and_then(|v| v.as_str())
                    .context("contacts.patch: missing id")?
                    .to_owned();
                sql!(
                    tx,
                    "UPDATE whatsapp.contacts SET
                        name = CASE WHEN $after ? 'name' THEN $after->>'name' ELSE name END,
                        pushname = CASE WHEN $after ? 'pushname' THEN $after->>'pushname' ELSE pushname END,
                        is_business = CASE WHEN $after ? 'is_business' THEN ($after->>'is_business')::boolean ELSE is_business END,
                        profile_pic = CASE WHEN $after ? 'profile_pic' THEN $after->>'profile_pic' ELSE profile_pic END,
                        profile_pic_full = CASE WHEN $after ? 'profile_pic_full' THEN $after->>'profile_pic_full' ELSE profile_pic_full END,
                        status = CASE WHEN $after ? 'status' THEN $after->>'status' ELSE status END,
                        saved = CASE WHEN $after ? 'saved' THEN ($after->>'saved')::boolean ELSE saved END
                    WHERE channel_id = $channel_id AND id = $id"
                )
                .execute()
                .await?;
            }
            Ok(())
        }

        // ── Groups ─────────────────────────────────────────────────
        ("groups", "post") => {
            let evt: GroupsEvent = serde_json::from_value(body.clone()).context("groups.post")?;
            let channel_id = &evt.channel_id;
            for g in &evt.groups {
                let id = g.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let name = g.get("name").and_then(|v| v.as_str());
                let description = g.get("description").and_then(|v| v.as_str());
                let data = g.clone();
                sql!(
                    tx,
                    "INSERT INTO whatsapp.groups (channel_id, id, name, description, data)
                     VALUES ($channel_id, $id, $name, $description, $data)
                     ON CONFLICT (channel_id, id) DO UPDATE SET
                         name = EXCLUDED.name,
                         description = EXCLUDED.description,
                         data = EXCLUDED.data"
                )
                .execute()
                .await?;
            }
            Ok(())
        }
        ("groups", "put") => {
            let evt: GroupsParticipantsEvent =
                serde_json::from_value(body.clone()).context("groups.put")?;
            let channel_id = &evt.channel_id;
            for pe in &evt.groups_participants {
                let group_id = &pe.group_id;
                let action = pe.action.as_deref();
                for participant_id in &pe.participants {
                    match action {
                        Some("remove") => {
                            sql!(
                                tx,
                                "DELETE FROM whatsapp.group_participants
                                 WHERE channel_id = $channel_id
                                   AND group_id = $group_id
                                   AND participant_id = $participant_id"
                            )
                            .execute()
                            .await?;
                        }
                        _ => {
                            sql!(
                                tx,
                                "INSERT INTO whatsapp.group_participants
                                     (channel_id, group_id, participant_id)
                                 VALUES ($channel_id, $group_id, $participant_id)
                                 ON CONFLICT (channel_id, group_id, participant_id) DO NOTHING"
                            )
                            .execute()
                            .await?;
                        }
                    }
                }
            }
            Ok(())
        }
        ("groups", "patch") => {
            let evt: GroupsUpdateEvent =
                serde_json::from_value(body.clone()).context("groups.patch")?;
            let channel_id = &evt.channel_id;
            for upd in &evt.groups_updates {
                let id = upd
                    .after_update
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let name = upd.after_update.get("name").and_then(|v| v.as_str());
                let description = upd.after_update.get("description").and_then(|v| v.as_str());
                let data = upd.after_update.clone();
                sql!(
                    tx,
                    "INSERT INTO whatsapp.groups (channel_id, id, name, description, data)
                     VALUES ($channel_id, $id, $name, $description, $data)
                     ON CONFLICT (channel_id, id) DO UPDATE SET
                         name = EXCLUDED.name,
                         description = EXCLUDED.description,
                         data = EXCLUDED.data"
                )
                .execute()
                .await?;
            }
            Ok(())
        }

        // ── Presences ──────────────────────────────────────────────
        ("presences", "post") => {
            let _: PresencesEvent =
                serde_json::from_value(body.clone()).context("presences.post")?;
            Ok(())
        }

        // ── Channel ────────────────────────────────────────────────
        ("channel", "post") => {
            let evt: ChannelHealthEvent =
                serde_json::from_value(body.clone()).context("channel.post")?;
            let channel_id = &evt.channel_id;
            let health = Some(evt.health);
            sql!(
                tx,
                "INSERT INTO whatsapp.channels (channel_id, health, updated_at)
                 VALUES ($channel_id, $health, now())
                 ON CONFLICT (channel_id) DO UPDATE SET
                     health = EXCLUDED.health, updated_at = now()"
            )
            .execute()
            .await?;
            Ok(())
        }
        ("channel", "patch") => {
            let evt: ChannelPatchEvent =
                serde_json::from_value(body.clone()).context("channel.patch")?;
            let channel_id = &evt.channel_id;
            let qr = Some(evt.qr);
            sql!(
                tx,
                "INSERT INTO whatsapp.channels (channel_id, qr, updated_at)
                 VALUES ($channel_id, $qr, now())
                 ON CONFLICT (channel_id) DO UPDATE SET
                     qr = EXCLUDED.qr, updated_at = now()"
            )
            .execute()
            .await?;
            Ok(())
        }

        // ── Users ──────────────────────────────────────────────────
        ("users", "post") => {
            let evt: UsersEvent = serde_json::from_value(body.clone()).context("users.post")?;
            upsert_contact(tx, &evt.channel_id, &evt.user).await
        }
        ("users", "delete") => {
            let evt: UsersEvent = serde_json::from_value(body.clone()).context("users.delete")?;
            let channel_id = &evt.channel_id;
            let id = &evt.user.id;
            sql!(
                tx,
                "DELETE FROM whatsapp.contacts WHERE channel_id = $channel_id AND id = $id"
            )
            .execute()
            .await?;
            Ok(())
        }

        // ── Labels ─────────────────────────────────────────────────
        ("labels", "post") => {
            let evt: LabelsEvent = serde_json::from_value(body.clone()).context("labels.post")?;
            let channel_id = &evt.channel_id;
            for label in &evt.labels {
                let id = &label.id;
                let name = &label.name;
                let color = &label.color;
                let count = label.count;
                sql!(
                    tx,
                    "INSERT INTO whatsapp.labels (channel_id, id, name, color, \"count\")
                     VALUES ($channel_id, $id, $name, $color, $count)
                     ON CONFLICT (channel_id, id) DO UPDATE SET
                         name = EXCLUDED.name,
                         color = EXCLUDED.color,
                         \"count\" = EXCLUDED.\"count\""
                )
                .execute()
                .await?;
            }
            Ok(())
        }
        ("labels", "delete") => {
            let evt: LabelsDeleteEvent =
                serde_json::from_value(body.clone()).context("labels.delete")?;
            let channel_id = &evt.channel_id;
            for id in &evt.labels_removed {
                sql!(
                    tx,
                    "DELETE FROM whatsapp.labels WHERE channel_id = $channel_id AND id = $id"
                )
                .execute()
                .await?;
            }
            Ok(())
        }

        // ── Calls ──────────────────────────────────────────────────
        ("calls", "post") => {
            let _: CallsEvent = serde_json::from_value(body.clone()).context("calls.post")?;
            Ok(())
        }

        // ── Desconhecido ───────────────────────────────────────────
        _ => bail!("Tipo de evento desconhecido: {et}.{ea}"),
    }
}

// ── Message insert/upsert ──────────────────────────────────────────────

async fn upsert_message(
    tx: &Transaction<'_>,
    channel_id: &str,
    msg: &MessageData,
) -> anyhow::Result<()> {
    let id = &msg.id;
    let msg_type = &msg.msg_type;
    let subtype = msg.subtype.as_deref();
    let chat_id = &msg.chat_id;
    let chat_name = msg.chat_name.as_deref();
    let from_id = msg.from.as_deref();
    let from_me = msg.from_me;
    let from_name = msg.from_name.as_deref();
    let source = msg.source.as_deref();
    let timestamp = msg.timestamp;
    let device_id = msg.device_id;
    let status = msg.status.as_deref();
    let text_body = msg.text.as_ref().map(|t| t.body.as_str());
    let image = msg.image.clone();
    let video = msg.video.clone();
    let short = msg.short.clone();
    let gif = msg.gif.clone();
    let audio = msg.audio.clone();
    let voice = msg.voice.clone();
    let document = msg.document.clone();
    let sticker = msg.sticker.clone();
    let location = msg.location.clone();
    let live_location = msg.live_location.clone();
    let contact = msg.contact.clone();
    let contact_list = msg.contact_list.clone();
    let link_preview = msg.link_preview.clone();
    let group_invite = msg.group_invite.clone();
    let newsletter_invite = msg.newsletter_invite.clone();
    let catalog = msg.catalog.clone();
    let interactive = msg.interactive.clone();
    let poll = msg.poll.clone();
    let hsm = msg.hsm.clone();
    let system_msg = msg.system.clone();
    let order = msg.order.clone();
    let admin_invite = msg.admin_invite.clone();
    let product = msg.product.clone();
    let product_items = msg.product_items.clone();
    let msg_event = msg.event.clone();
    let list = msg.list.clone();
    let buttons = msg.buttons.clone();
    let action = msg.action.clone();
    let context = msg.context.clone();
    let reactions = msg.reactions.clone();
    let msg_labels = msg.labels.clone();

    sql!(
        tx,
        "INSERT INTO whatsapp.messages (
            channel_id, id, msg_type, subtype, chat_id, chat_name,
            from_id, from_me, from_name, source, \"timestamp\", device_id, status,
            text_body,
            image, video, short, gif, audio, voice, document, sticker,
            location, live_location, contact, contact_list,
            link_preview, group_invite, newsletter_invite, catalog,
            interactive, poll, hsm, system_msg, \"order\",
            admin_invite, product, product_items, msg_event, list, buttons,
            \"action\", context, reactions, msg_labels
        ) VALUES (
            $channel_id, $id, $msg_type, $subtype, $chat_id, $chat_name,
            $from_id, $from_me, $from_name, $source, $timestamp, $device_id, $status,
            $text_body,
            $image, $video, $short, $gif, $audio, $voice, $document, $sticker,
            $location, $live_location, $contact, $contact_list,
            $link_preview, $group_invite, $newsletter_invite, $catalog,
            $interactive, $poll, $hsm, $system_msg, $order,
            $admin_invite, $product, $product_items, $msg_event, $list, $buttons,
            $action, $context, $reactions, $msg_labels
        ) ON CONFLICT (channel_id, id) DO UPDATE SET
            msg_type = EXCLUDED.msg_type, subtype = EXCLUDED.subtype,
            chat_id = EXCLUDED.chat_id, chat_name = EXCLUDED.chat_name,
            from_id = EXCLUDED.from_id, from_me = EXCLUDED.from_me,
            from_name = EXCLUDED.from_name, source = EXCLUDED.source,
            \"timestamp\" = EXCLUDED.\"timestamp\", device_id = EXCLUDED.device_id,
            status = EXCLUDED.status, text_body = EXCLUDED.text_body,
            image = EXCLUDED.image, video = EXCLUDED.video,
            short = EXCLUDED.short, gif = EXCLUDED.gif,
            audio = EXCLUDED.audio, voice = EXCLUDED.voice,
            document = EXCLUDED.document, sticker = EXCLUDED.sticker,
            location = EXCLUDED.location, live_location = EXCLUDED.live_location,
            contact = EXCLUDED.contact, contact_list = EXCLUDED.contact_list,
            link_preview = EXCLUDED.link_preview, group_invite = EXCLUDED.group_invite,
            newsletter_invite = EXCLUDED.newsletter_invite, catalog = EXCLUDED.catalog,
            interactive = EXCLUDED.interactive, poll = EXCLUDED.poll,
            hsm = EXCLUDED.hsm, system_msg = EXCLUDED.system_msg,
            \"order\" = EXCLUDED.\"order\", admin_invite = EXCLUDED.admin_invite,
            product = EXCLUDED.product, product_items = EXCLUDED.product_items,
            msg_event = EXCLUDED.msg_event, list = EXCLUDED.list,
            buttons = EXCLUDED.buttons, \"action\" = EXCLUDED.\"action\",
            context = EXCLUDED.context, reactions = EXCLUDED.reactions,
            msg_labels = EXCLUDED.msg_labels"
    )
    .execute()
    .await
    .context(format!("Upsert mensagem {id}"))?;

    Ok(())
}

// ── Chat upsert ────────────────────────────────────────────────────────

async fn upsert_chat(
    tx: &Transaction<'_>,
    channel_id: &str,
    chat: &ChatData,
) -> anyhow::Result<()> {
    let id = &chat.id;
    let name = chat.name.as_deref();
    let chat_type = &chat.chat_type;
    let timestamp = chat.timestamp;
    let chat_pic = chat.chat_pic.as_deref();
    let chat_pic_full = chat.chat_pic_full.as_deref();
    let pin = chat.pin;
    let mute = chat.mute;
    let mute_until = chat.mute_until;
    let archive = chat.archive;
    let unread = chat.unread;
    let unread_mention = chat.unread_mention;
    let read_only = chat.read_only;
    let not_spam = chat.not_spam;
    let last_message = chat.last_message.clone();
    let labels = chat.labels.clone();

    sql!(
        tx,
        "INSERT INTO whatsapp.chats (
            channel_id, id, name, chat_type, \"timestamp\",
            chat_pic, chat_pic_full, pin, mute, mute_until,
            archive, unread, unread_mention, read_only, not_spam,
            last_message, labels
        ) VALUES (
            $channel_id, $id, $name, $chat_type, $timestamp,
            $chat_pic, $chat_pic_full, $pin, $mute, $mute_until,
            $archive, $unread, $unread_mention, $read_only, $not_spam,
            $last_message, $labels
        ) ON CONFLICT (channel_id, id) DO UPDATE SET
            name = EXCLUDED.name, chat_type = EXCLUDED.chat_type,
            \"timestamp\" = EXCLUDED.\"timestamp\",
            chat_pic = EXCLUDED.chat_pic, chat_pic_full = EXCLUDED.chat_pic_full,
            pin = EXCLUDED.pin, mute = EXCLUDED.mute, mute_until = EXCLUDED.mute_until,
            archive = EXCLUDED.archive, unread = EXCLUDED.unread,
            unread_mention = EXCLUDED.unread_mention, read_only = EXCLUDED.read_only,
            not_spam = EXCLUDED.not_spam,
            last_message = EXCLUDED.last_message, labels = EXCLUDED.labels"
    )
    .execute()
    .await
    .context(format!("Upsert chat {id}"))?;

    Ok(())
}

// ── Contact upsert ─────────────────────────────────────────────────────

async fn upsert_contact(
    tx: &Transaction<'_>,
    channel_id: &str,
    c: &ContactData,
) -> anyhow::Result<()> {
    let id = &c.id;
    let name = &c.name;
    let pushname = c.pushname.as_deref();
    let is_business = c.is_business;
    let profile_pic = c.profile_pic.as_deref();
    let profile_pic_full = c.profile_pic_full.as_deref();
    let status = c.status.as_deref();
    let saved = c.saved;

    sql!(
        tx,
        "INSERT INTO whatsapp.contacts (
            channel_id, id, name, pushname, is_business,
            profile_pic, profile_pic_full, status, saved
        ) VALUES (
            $channel_id, $id, $name, $pushname, $is_business,
            $profile_pic, $profile_pic_full, $status, $saved
        ) ON CONFLICT (channel_id, id) DO UPDATE SET
            name = EXCLUDED.name, pushname = EXCLUDED.pushname,
            is_business = EXCLUDED.is_business,
            profile_pic = EXCLUDED.profile_pic,
            profile_pic_full = EXCLUDED.profile_pic_full,
            status = EXCLUDED.status, saved = EXCLUDED.saved"
    )
    .execute()
    .await
    .context("Upsert contact")?;

    Ok(())
}
