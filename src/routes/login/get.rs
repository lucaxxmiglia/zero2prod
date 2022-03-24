use actix_web::{web,HttpResponse, HttpRequest};
use actix_web::cookie::{Cookie,time::Duration};
use actix_web::http::header::ContentType;
use crate::startup::HmacSecret;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use actix_web_flash_messages::{IncomingFlashMessage, Level};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error>{
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

pub async fn login_form(flash_messages: IncomingFlashMessage) -> HttpResponse {
   let mut error = String::new();
   for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
       writeln!(error,"<p><i>{}</i></p>",m.content().unwrap() );
   }
   /*let error = match request.cookie("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!("<p><i>{}</i></p>",cookie.value())
        }
        
        match query.0.verify(&secret) {
            Ok(error) => format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error)),
            Err(e) => {
                tracing::warn!(error.message = %e, error.cause_chain = ?e, "Oooops HMAC tag");
                "".into()
            }
        }*/
        
        
        
    };
    return HttpResponse::Ok()
    .content_type(ContentType::html())
    //.cookie(Cookie::build("_flash","").max_age(Duration::ZERO).finish())
    .body(format!(
        r#"<!DOCTYPE html>
        <html lang="en">
        
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Login</title>
        </head>
        
        <body>
        {error}
            <form action="/login" method="post">
                <label>Username
                    <input type="text" placeholder="Enter Username" name="username">
                </label>
                <label>Password
                    <input type="password" placeholder="Enter Password" name="password">
                </label>
                <button type="submit">Login</button>
            </form>
        </body>
        
        </html>"#
    ))
}