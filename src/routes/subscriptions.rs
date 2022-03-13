use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

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
    std::iter::repeat_with(|| rng.sample(Alphanumeric)).map(char::from).take(25).collect()
}

#[tracing::instrument(name="Adding new subscriber", skip(form, pool,email_client, base_url), fields (subscriber_email=%form.email, subscriber_name=%form.name))]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>, base_url: web::Data<ApplicationBaseUrl>) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(sub) => sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    
    let subscriber_id = match insert_subscriber(&new_subscriber, &pool).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    let sub_token = generate_subscription_token();

    if store_token(&pool, subscriber_id, &sub_token).await.is_err() {
        return HttpResponse::InternalServerError().finish()
    }
    
    if send_confirmation_email(&email_client,new_subscriber, &base_url.0,&sub_token).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name="Manda mail a new subscriber", skip(email_client, new_subscriber, base_url, subscription_token))]
pub async fn send_confirmation_email(email_client: &EmailClient, new_subscriber: NewSubscriber, base_url: &str, subscription_token: &str) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?subscription_token={}", base_url, subscription_token);
    let html_body = &format!("Welcome! Click <a href=\"{}\" here </a>", confirmation_link);
    let text_body = &format!("Welcome! Click {} here", confirmation_link);
    email_client.send_email(new_subscriber.email, "Welcome!", html_body,text_body).await
}

#[tracing::instrument(name = "Salvo subscriber", skip(new_subscriber, pool))]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    pool: &PgPool,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at,status) VALUES ($1,$2,$3,$4,'pending_confirmation')",
        subscriber_id,
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
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Salvo token", skip(sub_token, pool))]
pub async fn store_token(
    pool: &PgPool,
    sub_id: Uuid,
    sub_token: &str
    
) -> Result<(), sqlx::Error> {
    sqlx::query!("INSERT INTO subscription_tokens (subscription_token, subscriber_id) values ($1,$2)", sub_token, sub_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Fallita esecuzione query {:?}", e);
        e
    })?;
    Ok(())
}