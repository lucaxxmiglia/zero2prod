use actix_web::{ HttpResponse, web, ResponseError, HttpRequest};
use actix_web::http::{StatusCode, header};
use actix_web::http::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use sqlx::PgPool;
use crate::routes::error_chain_fmt;
use crate::email_client::EmailClient;
use anyhow::Context;
use crate::domain::SubscriberEmail;
use secrecy::Secret;
use crate::authentication::*;

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
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]   
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter::<'_>) -> std::fmt::Result {
        error_chain_fmt(self,f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            PublishError::AuthError(_) =>{
                let mut response = HttpResponse::new (StatusCode::UNAUTHORIZED);
                let head_val = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response.headers_mut().insert(header::WWW_AUTHENTICATE, head_val);
                response
                
            } 
        }
    }
}




fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error>{
    //dbg!("{:?}",headers);
    let head = headers.get("authorization").context("Auth is missing")?.to_str().context("UTF8 not a valid string")?;

    let b64 = head.strip_prefix("Basic ").context("Auth is not Basic")?;

    let decoded_bytes = base64::decode_config(b64, base64::STANDARD).context("Failed to base64")?;

    let decoded = String::from_utf8(decoded_bytes).context("No UTF8")?;

    let mut creds = decoded.splitn(2,":");

    let username = creds.next().ok_or_else(|| anyhow::anyhow!("Username missing"))?.to_string();
    let password = creds.next().ok_or_else(|| anyhow::anyhow!("Password missing"))?.to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password)
    })

}



#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
    )]
pub async fn publish_newsletter(body: web::Json<BodyData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>, request: HttpRequest) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(&request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username",&tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool).await
    .map_err(|e| match e {
        AuthError::InvalidCredentialsError(_) => PublishError::AuthError(e.into()),
        AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into())
    })?;
    tracing::Span::current().record("user_id",&tracing::field::display(&user_id));
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