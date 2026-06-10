-- Backoff exponencial por cliente no worker `jobs::mei_refresh`.
--
-- Antes: quando a atualização da situação MEI de um cliente falhava por
-- instabilidade do gov.br/Receita (ex: captcha não resolvido), o worker NÃO
-- carimbava `mei_consultado_em` "pra retentar no próximo ciclo" — e acabava
-- re-tentando o MESMO cliente de hora em hora, pra sempre. Isso é bot-like,
-- desperdiça browser/cota de captcha e foi o que queimou o IP no NopeCHA.
--
-- Agora: cada falha transitória espaça a próxima tentativa do worker com
-- backoff exponencial (1h, 2h, 4h, … saturando em 72h). Um desfecho
-- conclusivo (tem MEI / elegibilidade checada) zera o contador.
--
-- - mei_refresh_falhas: nº de falhas transitórias consecutivas (dirige a
--   curva do backoff).
-- - mei_proxima_tentativa_em: o worker não re-seleciona o cliente antes
--   disso. NULL = já elegível. Só afeta a seleção do background; o fluxo
--   interativo (`auth_govbr`) nunca consulta essa coluna.
ALTER TABLE zain.clients
    ADD COLUMN mei_refresh_falhas       INTEGER     NOT NULL DEFAULT 0,
    ADD COLUMN mei_proxima_tentativa_em TIMESTAMPTZ;
