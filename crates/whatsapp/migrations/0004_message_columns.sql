ALTER TABLE whatsapp.messages ADD COLUMN raw_id TEXT NOT NULL DEFAULT '';
ALTER TABLE whatsapp.messages ADD COLUMN sender_jid TEXT;
ALTER TABLE whatsapp.messages ADD COLUMN is_from_me BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE whatsapp.messages ADD COLUMN text TEXT;
ALTER TABLE whatsapp.messages ADD COLUMN sender_name TEXT;
ALTER TABLE whatsapp.messages DROP COLUMN data;
