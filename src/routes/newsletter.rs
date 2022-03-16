use actix_web::{ HttpResponse, web, ResponseError};
use actix_web::http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use crate::routes::error_chain_fmt;
use crate::email_client::EmailClient;
use anyhow::Context;
use crate::domain::SubscriberEmail;

struct ConfirmedSubscriber {
    pub email: SubscriberEmail,
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter::<'_>) -> std::fmt::Result {
        error_chain_fmt(self,f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn publish_newsletter(body: web::Json<BodyData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>) -> Result<HttpResponse, PublishError> {
    let subs = get_confirmed_subs(&pool).await?;
    for sub in subs {
        match sub {
            Ok(sub) => {
                email_client.send_email(&sub.email, &body.title, &body.content.html, &body.content.text).await.with_context(|| format!("Failed to send {}",sub.email))?;
            },
            Err(error)  => {
                tracing::warn!(error.cause_chain = ?error, "Skip conf sub, email invalid")
            }
         }
        
    }
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name="Get confirmed subs", skip(pool))]
async fn get_confirmed_subs(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {

    let rows = sqlx::query!( "SELECT email from subscriptions where status = 'confirmed'")
    .fetch_all(pool)
    .await?;

    let subs = rows.into_iter().map(|r| 
        match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber {email}),
            Err(e) => {
                
                Err(anyhow::anyhow!(e))
        }
    }).collect();

    Ok(subs)
}