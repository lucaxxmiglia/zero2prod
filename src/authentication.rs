use anyhow::Context;
use secrecy::{Secret, ExposeSecret};
use sqlx::PgPool;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentialsError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Debug,Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>
}

pub async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<uuid::Uuid, AuthError>{
    let user_id = sqlx::query!("select user_id from users where username=$1 and password=$2", credentials.username, credentials.password.expose_secret())
    .fetch_optional(pool).await.context("Oooops query").map_err(AuthError::UnexpectedError)?;

    user_id.map(|row| row.user_id).ok_or_else(|| anyhow::anyhow!("Invalid username or password"))
    .map_err(AuthError::InvalidCredentialsError)
}