-- Remove o schema `whatsapp` por completo. As tabelas de WhatsApp (outbox,
-- messages, channels, entidades de webhook, participantes de grupo, etc.)
-- ficaram órfãs quando os crates `whatsapp` e `agent` saíram do workspace —
-- o foco passou a ser config + MCP + dados-abertos. Nenhum crate restante
-- referencia `whatsapp.*`.

DROP SCHEMA IF EXISTS whatsapp CASCADE;
