use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use crate::domain::{NewSubscriber, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(name="Adding new subscriber", skip(form, pool), fields (subscriber_email=%form.email, subscriber_name=%form.name))]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {

let name = match SubscriberName::parse(form.0.name) {
    Ok(name) => name,
    Err(_) => return HttpResponse::BadRequest().finish()
};

let new_subscriber = NewSubscriber {
    email: form.0.email,
    name: name,
};

    match insert_subscriber(&new_subscriber, &pool)
        .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}


#[tracing::instrument(name = "Salvo subscriber", skip(new_subscriber, pool))]
pub async fn insert_subscriber(new_subscriber: &NewSubscriber, pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1,$2,$3,$4)",
        Uuid::new_v4(),
        new_subscriber.email,
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