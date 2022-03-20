use actix_web::{web,HttpResponse};
use actix_web::http::header::ContentType;

mod post;
pub use post::*;
#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>
}

pub async fn login_form(param: web::Query<QueryParams>) -> HttpResponse {
    let error = match param.0.error {
        None => "".into(),
        Some(error_message) => format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error_message))
    };
    return HttpResponse::Ok()
    .content_type(ContentType::html())
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