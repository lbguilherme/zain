-- Última atividade do cliente: carimbada toda vez que o `get_client_state`
-- roda (= todo turno em que o cliente interage). Os workers de background
-- usam isso pra ESPAÇAR a cadência de clientes inativos — não faz sentido
-- abrir gov.br / portal repetidamente pra quem não conversa há semanas.
--
-- NULL = nunca interagiu via get_client_state (lead muito frio / antigo) —
-- tratado como "bem inativo" pela fórmula de cadência.
ALTER TABLE zain.clients
    ADD COLUMN last_activity_at TIMESTAMPTZ;
