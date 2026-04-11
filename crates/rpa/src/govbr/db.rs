//! Acesso à tabela `zain.govbr`.
//!
//! A PK é `client_id` (FK para `zain.clients`). `cpf` é só uma coluna
//! regular — múltiplos clientes podem compartilhar o mesmo cpf.
//!
//! As colunas `session` (DOMAIN sobre JSONB) e `nivel` (ENUM) são mapeadas
//! automaticamente para [`SavedSession`] e [`Nivel`] via configuração do
//! `cubos_sql` em [`Cargo.toml`](../../../Cargo.toml).

use cubos_sql::sql;
use deadpool_postgres::Pool;
use uuid::Uuid;

use super::Nivel;
use super::session::SavedSession;

#[derive(Debug, Clone)]
pub struct GovbrRow {
    pub client_id: Uuid,
    pub cpf: String,
    pub password: String,
    pub otp: Option<String>,
    pub session: Option<SavedSession>,
    pub nome: Option<String>,
    pub email: Option<String>,
    pub telefone: Option<String>,
    pub nivel: Option<Nivel>,
}

pub async fn load(pool: &Pool, client_id: Uuid) -> anyhow::Result<Option<GovbrRow>> {
    let row = sql!(
        pool,
        "SELECT client_id, cpf, password, otp, session, nome, email, telefone, nivel
         FROM zain.govbr WHERE client_id = $client_id"
    )
    .fetch_optional()
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    Ok(Some(GovbrRow {
        client_id: r.client_id,
        cpf: r.cpf,
        password: r.password,
        otp: r.otp,
        session: r.session,
        nome: r.nome,
        email: r.email,
        telefone: r.telefone,
        nivel: r.nivel,
    }))
}

pub async fn clear_session(pool: &Pool, client_id: Uuid) -> anyhow::Result<()> {
    sql!(
        pool,
        "UPDATE zain.govbr SET session = NULL, updated_at = now()
         WHERE client_id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

pub async fn save_session(
    pool: &Pool,
    client_id: Uuid,
    session: &SavedSession,
) -> anyhow::Result<()> {
    // O `sql!` precisa do valor por ownership; clone é barato comparado
    // a ir/voltar do banco.
    let session = session.clone();
    sql!(
        pool,
        "UPDATE zain.govbr SET session = $session!, updated_at = now()
         WHERE client_id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}

pub async fn save_profile(
    pool: &Pool,
    client_id: Uuid,
    nome: &str,
    email: Option<&str>,
    telefone: Option<&str>,
    nivel: Option<Nivel>,
) -> anyhow::Result<()> {
    // `nome` é NULLABLE na tabela — mantém Option no binding para o macro.
    let nome: Option<&str> = Some(nome);
    sql!(
        pool,
        "UPDATE zain.govbr
         SET nome = $nome, email = $email, telefone = $telefone, nivel = $nivel,
             updated_at = now()
         WHERE client_id = $client_id"
    )
    .execute()
    .await?;
    Ok(())
}
