CREATE OR REPLACE FUNCTION whatsapp.outbox_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('whatsapp_outbox', '');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER outbox_notify_trigger
AFTER INSERT ON whatsapp.outbox
FOR EACH ROW EXECUTE FUNCTION whatsapp.outbox_notify();
