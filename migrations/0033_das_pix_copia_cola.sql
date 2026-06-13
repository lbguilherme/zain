-- Cacheia o "PIX copia e cola" (payload EMV/BR Code) da guia DAS junto do
-- PDF, pra não ter que redecodificar o QR a cada cache hit do mesmo dia.
-- NULL quando não deu pra decodificar o QR (best-effort).
ALTER TABLE zain.das_guia_cache
    ADD COLUMN pix_copia_cola TEXT;
