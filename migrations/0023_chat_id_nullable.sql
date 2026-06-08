-- Torna `chat_id` opcional em `zain.clients`. Clientes criados via MCP
-- (a partir do `cubos-agent.userId`) não necessariamente têm um
-- identificador de canal — origem pode ser qualquer canal que o agente
-- estiver expondo, e o registro chega antes de qualquer mensagem de
-- WhatsApp. A UNIQUE continua: Postgres permite múltiplos NULL em
-- UNIQUE por padrão, então a coluna segue servindo de chave de
-- deduplicação pro fluxo WhatsApp.

ALTER TABLE zain.clients
    ALTER COLUMN chat_id DROP NOT NULL;
