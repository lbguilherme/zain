CREATE OR REPLACE FUNCTION zain.clients_notify() RETURNS trigger AS $$
BEGIN
    IF NEW.needs_processing = true
       AND (TG_OP = 'INSERT' OR OLD.needs_processing IS DISTINCT FROM NEW.needs_processing)
    THEN
        PERFORM pg_notify('zain_clients_needs_processing', '');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER clients_notify_trigger
AFTER INSERT OR UPDATE OF needs_processing ON zain.clients
FOR EACH ROW EXECUTE FUNCTION zain.clients_notify();
