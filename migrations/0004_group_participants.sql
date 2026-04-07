CREATE TABLE whatsapp.group_participants (
    channel_id      TEXT NOT NULL,
    group_id        TEXT NOT NULL,
    participant_id  TEXT NOT NULL,
    PRIMARY KEY (channel_id, group_id, participant_id)
);
