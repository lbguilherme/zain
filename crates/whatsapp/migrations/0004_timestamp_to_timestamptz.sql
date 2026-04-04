ALTER TABLE whatsapp.chats
    ALTER COLUMN timestamp TYPE TIMESTAMPTZ
    USING to_timestamp(timestamp);

ALTER TABLE whatsapp.messages
    ALTER COLUMN timestamp TYPE TIMESTAMPTZ
    USING to_timestamp(timestamp);
