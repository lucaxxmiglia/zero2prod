use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use anyhow::Context;

pub struct StoreTokenError(sqlx::Error);
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Failed to acquire a Postgres connection from the pool")]
    PoolError(#[source] sqlx::Error),
    #[error("Error inserting subscriber")]
    InsertSubscriberError(#[source] sqlx::Error),
    #[error("Error committing transaction")]
    TransactionCommitError(#[source] sqlx::Error),
    #[error("Error storing token")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Error sending mail")]
    SendEmailError(#[from] reqwest::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A database error occurred while storing a sub token")
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by: \n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::PoolError(_) | SubscribeError::InsertSubscriberError(_) | SubscribeError::TransactionCommitError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber { name, email })
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name="Adding new subscriber", skip(form, pool,email_client, base_url), fields (subscriber_email=%form.email, subscriber_name=%form.name))]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let mut transaction = pool.begin().await.context("Ooops pool")?;

    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction).await.context("Ooops insert subscriber")?;

    let sub_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &sub_token).await.context("Ooops store token")?;

    transaction.commit().await.map_err(SubscribeError::TransactionCommitError).context("Ooops transaction commit")?;

    send_confirmation_email(&email_client, new_subscriber, &base_url.0, &sub_token).await.context("Ooops send email")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Manda mail a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = &format!("Welcome! Click <a href=\"{}\" here </a>", confirmation_link);
    let text_body = &format!("Welcome! Click {} here", confirmation_link);
    email_client
        .send_email(new_subscriber.email, "Welcome!", html_body, text_body)
        .await
}

#[tracing::instrument(name = "Salvo subscriber", skip(new_subscriber, transaction))]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at,status) VALUES ($1,$2,$3,$4,'pending_confirmation')",
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        //tracing::error!("Fallita esecuzione query {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Salvo token", skip(sub_token, transaction))]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    sub_id: Uuid,
    sub_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        "INSERT INTO subscription_tokens (subscription_token, subscriber_id) values ($1,$2)",
        sub_token,
        sub_id
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        //tracing::error!("Fallita esecuzione query {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}
