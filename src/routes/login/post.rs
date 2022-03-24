use actix_web::HttpResponse;
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::error::InternalError;
use actix_web::cookie::Cookie;
use secrecy::Secret;
use actix_web::{web, ResponseError};
use sqlx::PgPool;
use crate::authentication::*;
use crate::routes::error_chain_fmt;
use hmac::{Hmac,Mac};
use secrecy::ExposeSecret;
use crate::startup::HmacSecret;
use actix_web_flash_messages::FlashMessage;


#[derive(serde::Deserialize)]
pub struct LoginFormData {
    pub username: String,
    pub password: Secret<String>
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Auth failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    
    fn status_code(&self) -> StatusCode {
       /* match self {
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }*/
        StatusCode::SEE_OTHER
    }

}

#[tracing::instrument(skip(form, pool), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(form: web::Form<LoginFormData>, pool: web::Data<PgPool>, secret: web::Data<HmacSecret>) -> Result<HttpResponse,InternalError<LoginError>> {
    let creds = Credentials {
        username: form.0.username,
        password: form.0.password
    };
    
    tracing::Span::current().record("username", &tracing::field::display(&creds.username));
    match validate_credentials(creds, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION,"/"))
            .finish())
        },
        Err(e) => {
         let e = match e {
                AuthError::InvalidCredentialsError(_)=> LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into())
        };
     /*   let query_string = format!("error={}",urlencoding::Encoded::new(e.to_string()));
        let hmac_tac = {
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };*/
        FlashMessage::error(e.to_string()).send();
        let response = HttpResponse::SeeOther()
      //  .insert_header((LOCATION,format!("/login?{}&tag={:x}", query_string,hmac_tac)))
        .insert_header((LOCATION,"/login"))
        //.cookie(Cookie::new("_flash",e.to_string()))
        .finish();
        Err(InternalError::from_response(e, response))
        }
    }
        
    
}