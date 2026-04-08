-- Converter timestamps BIGINT para TIMESTAMPTZ

ALTER TABLE whatsapp.messages
  ALTER COLUMN "timestamp" TYPE TIMESTAMPTZ USING to_timestamp("timestamp");

ALTER TABLE whatsapp.chats
  ALTER COLUMN "timestamp" TYPE TIMESTAMPTZ USING CASE WHEN "timestamp" IS NOT NULL THEN to_timestamp("timestamp") END,
  ALTER COLUMN mute_until TYPE TIMESTAMPTZ USING CASE WHEN mute_until IS NOT NULL THEN to_timestamp(mute_until) END;

ALTER TABLE whatsapp.statuses
  ALTER COLUMN "timestamp" TYPE TIMESTAMPTZ USING CASE WHEN "timestamp" IS NOT NULL THEN to_timestamp("timestamp"::float8) END;

-- Coluna para rastrear última mensagem processada pelo agent

ALTER TABLE zain.clients
  ADD COLUMN last_whatsapp_message_processed_at TIMESTAMPTZ;
