-- ── Domains para conteudo de mensagem ───────────────────────────────────

CREATE DOMAIN whatsapp.image_content AS JSONB;
CREATE DOMAIN whatsapp.video_content AS JSONB;
CREATE DOMAIN whatsapp.audio_content AS JSONB;
CREATE DOMAIN whatsapp.voice_content AS JSONB;
CREATE DOMAIN whatsapp.document_content AS JSONB;
CREATE DOMAIN whatsapp.sticker_content AS JSONB;
CREATE DOMAIN whatsapp.location_content AS JSONB;
CREATE DOMAIN whatsapp.live_location_content AS JSONB;
CREATE DOMAIN whatsapp.contact_msg_content AS JSONB;
CREATE DOMAIN whatsapp.contact_list_content AS JSONB;
CREATE DOMAIN whatsapp.link_preview_content AS JSONB;
CREATE DOMAIN whatsapp.interactive_content AS JSONB;
CREATE DOMAIN whatsapp.poll_content AS JSONB;
CREATE DOMAIN whatsapp.hsm_content AS JSONB;
CREATE DOMAIN whatsapp.system_content AS JSONB;
CREATE DOMAIN whatsapp.order_content AS JSONB;
CREATE DOMAIN whatsapp.event_content AS JSONB;
CREATE DOMAIN whatsapp.product_content AS JSONB;
CREATE DOMAIN whatsapp.product_items_content AS JSONB;
CREATE DOMAIN whatsapp.admin_invite_content AS JSONB;
CREATE DOMAIN whatsapp.list_content AS JSONB;
CREATE DOMAIN whatsapp.buttons_content AS JSONB;
CREATE DOMAIN whatsapp.message_context AS JSONB;
CREATE DOMAIN whatsapp.message_action AS JSONB;

-- ── Domains para canal ─────────────────────────────────────────────────

CREATE DOMAIN whatsapp.health_data AS JSONB;
CREATE DOMAIN whatsapp.qr_data AS JSONB;

-- ── Tabela de mensagens (espelha Message do Whapi) ─────────────────────

CREATE TABLE whatsapp.messages (
    channel_id          TEXT NOT NULL,
    id                  TEXT NOT NULL,
    msg_type            TEXT NOT NULL,
    subtype             TEXT,
    chat_id             TEXT NOT NULL,
    chat_name           TEXT,
    from_id             TEXT,
    from_me             BOOLEAN NOT NULL,
    from_name           TEXT,
    source              TEXT,
    "timestamp"         BIGINT NOT NULL,
    device_id           BIGINT,
    status              TEXT,
    text_body           TEXT,
    image               whatsapp.image_content,
    video               whatsapp.video_content,
    short               whatsapp.video_content,
    gif                 whatsapp.video_content,
    audio               whatsapp.audio_content,
    voice               whatsapp.voice_content,
    document            whatsapp.document_content,
    sticker             whatsapp.sticker_content,
    location            whatsapp.location_content,
    live_location       whatsapp.live_location_content,
    contact             whatsapp.contact_msg_content,
    contact_list        whatsapp.contact_list_content,
    link_preview        whatsapp.link_preview_content,
    group_invite        whatsapp.link_preview_content,
    newsletter_invite   whatsapp.link_preview_content,
    catalog             whatsapp.link_preview_content,
    interactive         whatsapp.interactive_content,
    poll                whatsapp.poll_content,
    hsm                 whatsapp.hsm_content,
    system_msg          whatsapp.system_content,
    "order"             whatsapp.order_content,
    admin_invite        whatsapp.admin_invite_content,
    product             whatsapp.product_content,
    product_items       whatsapp.product_items_content,
    msg_event           whatsapp.event_content,
    list                whatsapp.list_content,
    buttons             whatsapp.buttons_content,
    "action"            whatsapp.message_action,
    context             whatsapp.message_context,
    reactions           JSONB,
    msg_labels          JSONB,
    PRIMARY KEY (channel_id, id)
);

-- ── Tabela de chats (espelha Chat do Whapi) ────────────────────────────

CREATE TABLE whatsapp.chats (
    channel_id      TEXT NOT NULL,
    id              TEXT NOT NULL,
    name            TEXT,
    chat_type       TEXT NOT NULL,
    "timestamp"     BIGINT,
    chat_pic        TEXT,
    chat_pic_full   TEXT,
    pin             BOOLEAN,
    mute            BOOLEAN,
    mute_until      BIGINT,
    archive         BOOLEAN,
    unread          INTEGER,
    unread_mention  BOOLEAN,
    read_only       BOOLEAN,
    not_spam        BOOLEAN,
    last_message    JSONB,
    labels          JSONB,
    PRIMARY KEY (channel_id, id)
);

-- ── Tabela de contatos (espelha Contact do Whapi) ──────────────────────
-- Usada tanto para contacts.post quanto users.post/delete

CREATE TABLE whatsapp.contacts (
    channel_id          TEXT NOT NULL,
    id                  TEXT NOT NULL,
    name                TEXT NOT NULL,
    pushname            TEXT,
    is_business         BOOLEAN,
    profile_pic         TEXT,
    profile_pic_full    TEXT,
    status              TEXT,
    saved               BOOLEAN,
    PRIMARY KEY (channel_id, id)
);

-- ── Tabela de labels ───────────────────────────────────────────────────

CREATE TABLE whatsapp.labels (
    channel_id  TEXT NOT NULL,
    id          TEXT NOT NULL,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    "count"     INTEGER,
    PRIMARY KEY (channel_id, id)
);

-- ── Tabela de grupos (espelha Group do Whapi = Chat + campos extras) ──

CREATE TABLE whatsapp.groups (
    channel_id  TEXT NOT NULL,
    id          TEXT NOT NULL,
    name        TEXT,
    description TEXT,
    data        JSONB NOT NULL,
    PRIMARY KEY (channel_id, id)
);

-- ── Tabela de statuses de mensagem ─────────────────────────────────────

CREATE TABLE whatsapp.statuses (
    channel_id      TEXT NOT NULL,
    message_id      TEXT NOT NULL,
    status          TEXT NOT NULL,
    status_code     INTEGER,
    recipient_id    TEXT,
    viewer_id       TEXT,
    "timestamp"     TEXT,
    PRIMARY KEY (channel_id, message_id)
);

-- ── Tabela de canais (saude + QR) ──────────────────────────────────────

CREATE TABLE whatsapp.channels (
    channel_id  TEXT PRIMARY KEY,
    health      whatsapp.health_data,
    qr          whatsapp.qr_data,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
