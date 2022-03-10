use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use tracing_actix_web::TracingLogger;
use std::net::TcpListener;
use sqlx::PgPool;
use crate::email_client::EmailClient;

pub mod configuration;
pub mod routes;
pub mod domain;
pub mod telemetry;
pub mod email_client;

pub fn run(listener: TcpListener, db_poop: PgPool, email_client:EmailClient) -> Result<Server,std::io::Error> {
   let db_poop = web::Data::new(db_poop);
   let email_client = web::Data::new(email_client);
   let server=  HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(routes::health_check))
            .route("/subscribe", web::post().to(routes::subscribe))
            .app_data(db_poop.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
    
}
