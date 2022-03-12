use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use crate::email_client::EmailClient;

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

#[tracing::instrument(name="Adding new subscriber", skip(form, pool,email_client), fields (subscriber_email=%form.email, subscriber_name=%form.name))]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(sub) => sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if insert_subscriber(&new_subscriber, &pool).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
  
    if send_confirmation_email(&email_client,new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name="Manda mail a new subscriber", skip(email_client, new_subscriber))]
pub async fn send_confirmation_email(email_client: &EmailClient, new_subscriber: NewSubscriber) -> Result<(), reqwest::Error> {
    let confirmation_link = "https://mytest.com/confirm";
    let html_body = &format!("Welcome! Click <a href=\"{}\" here </a>", confirmation_link);
    let text_body = &format!("Welcome! Click {} here", confirmation_link);
    email_client.send_email(new_subscriber.email, "Welcome!", html_body,text_body).await
}

#[tracing::instrument(name = "Salvo subscriber", skip(new_subscriber, pool))]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at,status) VALUES ($1,$2,$3,$4,'confirmed')",
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Fallita esecuzione query {:?}", e);
        e
    })?;
    Ok(())
}
