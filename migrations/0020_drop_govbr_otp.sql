-- Remove a coluna `govbr_otp` de `zain.clients`. O OTP é efêmero e não
-- precisa ser persistido: a tool `auth_govbr_otp` recebe o código direto
-- do usuário no momento do login.

ALTER TABLE zain.clients
    DROP COLUMN govbr_otp;
