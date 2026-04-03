//! WhatsApp domain types: message IDs, chat IDs, parsed data-id.

/// A parsed `data-id` attribute from a WhatsApp message element.
///
/// Format for 1:1 chats: `{direction}_{chatJid}_{messageId}`
/// Format for groups:     `{direction}_{groupJid}_{messageId}_{senderLid}`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataId {
    /// Raw `data-id` string from the DOM.
    pub raw: String,
    /// `true` if the message was sent by us, `false` if received.
    pub outgoing: bool,
    /// JID of the chat (contact or group).
    pub chat_jid: String,
    /// Unique message identifier (hex hash).
    pub message_id: String,
    /// Sender's LID, present in group messages.
    pub sender_lid: Option<String>,
}

impl DataId {
    /// Parses a `data-id` string.
    ///
    /// Returns `None` if the format is unrecognized.
    pub fn parse(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.splitn(4, '_').collect();
        if parts.len() < 3 {
            return None;
        }

        let outgoing = match parts[0] {
            "true" => true,
            "false" => false,
            _ => return None,
        };
        let chat_jid = parts[1].to_owned();
        if !chat_jid.contains('@') {
            return None;
        }

        // parts[2] might be "messageId" or "messageId_senderLid"
        // If the chat is a group (@g.us) and there are 4 parts, the 4th is sender LID
        if parts.len() == 4 {
            // Group with sender LID: false_group@g.us_msgId_sender@lid
            // But splitn(4) means parts[2] = msgId, parts[3] = rest (could contain more _)
            let message_id = parts[2].to_owned();
            let sender_lid = Some(parts[3].to_owned());
            Some(Self {
                raw: raw.to_owned(),
                outgoing,
                chat_jid,
                message_id,
                sender_lid,
            })
        } else {
            // 1:1 chat: false_contact@c.us_msgId
            let message_id = parts[2].to_owned();
            Some(Self {
                raw: raw.to_owned(),
                outgoing,
                chat_jid,
                message_id,
                sender_lid: None,
            })
        }
    }

    /// Returns `true` if this is a group chat message.
    pub fn is_group(&self) -> bool {
        self.chat_jid.contains("@g.us")
    }
}

/// Summary of a chat in the sidebar list.
#[derive(Debug, Clone)]
pub struct ChatPreview {
    /// Chat title (contact name or group name).
    pub title: String,
    /// Last message preview text.
    pub last_message: String,
    /// Number of unread messages (0 if none).
    pub unread_count: u32,
    /// Parsed timestamp of the last message activity.
    pub timestamp: Option<chrono::NaiveDateTime>,
}

/// Type of a WhatsApp message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Image,
    Sticker,
    Voice,
    Video,
    System,
    Unknown,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Sticker => "sticker",
            Self::Voice => "voice",
            Self::Video => "video",
            Self::System => "system",
            Self::Unknown => "unknown",
        }
    }
}

/// A message parsed from the WhatsApp Web DOM with all extracted data.
#[derive(Debug, Clone)]
pub struct RawMessage {
    /// Parsed data-id with chat/message/sender identifiers.
    /// Access `data_id.raw` for the full raw string, `data_id.outgoing` for direction.
    pub data_id: DataId,
    /// Message type.
    pub msg_type: MessageType,
    /// Text content (body for text, caption for images, label for stickers).
    pub text: Option<String>,
    /// Sender JID (from data-id: chat_jid for 1:1 or sender_lid for groups).
    pub sender_jid: Option<String>,
    /// Sender display name (from data-pre-plain-text).
    pub sender_name: Option<String>,
    /// Parsed timestamp from data-pre-plain-text (minute precision).
    pub timestamp: Option<chrono::NaiveDateTime>,
    /// Sticker media filename (saved in media dir as `sticker_{sha256}`).
    pub sticker_media: Option<String>,
    /// Image media filename (saved in media dir as `image_{sha256}`).
    pub image_media: Option<String>,
}

/// User profile information from the "Edit profile" screen.
#[derive(Debug, Clone)]
pub struct UserProfile {
    /// Display name.
    pub name: String,
    /// Phone number (e.g. "+55 71 9934-3413").
    pub phone: String,
    /// Avatar image URL from the WhatsApp CDN, or `None` if no photo is set.
    pub avatar_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_1to1() {
        let id = DataId::parse("false_557193738552@c.us_3AB2DE63CABCBEA800E7").unwrap();
        assert!(!id.outgoing);
        assert_eq!(id.chat_jid, "557193738552@c.us");
        assert_eq!(id.message_id, "3AB2DE63CABCBEA800E7");
        assert!(id.sender_lid.is_none());
        assert!(!id.is_group());
    }

    #[test]
    fn parse_group_with_lid() {
        let id = DataId::parse(
            "false_5511972699834-1553516308@g.us_AC9EFCCCE5BBA2342F8BC46AB241EF82_45982020554835@lid",
        )
        .unwrap();
        assert!(!id.outgoing);
        assert_eq!(id.chat_jid, "5511972699834-1553516308@g.us");
        assert_eq!(id.message_id, "AC9EFCCCE5BBA2342F8BC46AB241EF82");
        assert_eq!(id.sender_lid.as_deref(), Some("45982020554835@lid"));
        assert!(id.is_group());
    }

    #[test]
    fn parse_sent() {
        let id = DataId::parse("true_557193738552@c.us_AABBCCDD").unwrap();
        assert!(id.outgoing);
    }

    #[test]
    fn parse_invalid() {
        assert!(DataId::parse("garbage").is_none());
        assert!(DataId::parse("false_only_two").is_none());
    }
}
