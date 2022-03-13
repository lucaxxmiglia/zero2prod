use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name="Conferma pending", skip(parameters, pool))]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some (subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(name="Mark subscriber confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) ->  Result<(), sqlx::Error> {
    sqlx::query!("UPDATE subscriptions SET status = 'confirmed' where id = $1", subscriber_id).execute(pool).await
    .map_err(|e| {tracing::error!("Failed to execute query {:?}",e); e})?;
    Ok(())
}

#[tracing::instrument(name="Individua id dal token", skip(pool, token))]
pub async fn get_subscriber_id_from_token(pool: &PgPool, token: &str) ->  Result<Option<Uuid>, sqlx::Error> {
    let res = sqlx::query!("select subscriber_id from subscription_tokens where subscription_token = $1", token).fetch_optional(pool).await
    .map_err(|e| {tracing::error!("Failed to execute query {:?}",e); e})?;
    Ok(res.map(|r| r.subscriber_id))
}
