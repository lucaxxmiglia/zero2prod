
use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use tracing_actix_web::TracingLogger;
use std::net::TcpListener;
use sqlx::PgPool;
use crate::email_client::EmailClient;
use crate::configuration::Settings;
use sqlx::postgres::PgPoolOptions;
use crate::configuration::DatabaseSettings;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error>{
        let timeout = configuration.email_client.timeout();
        let conn_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration.email_client.sender().expect("Ooops sender");
        let address = format! ("{}:{}", configuration.application.host, configuration.application.port);
        let list = std::net::TcpListener::bind(address)?;
        
        let email_client = EmailClient::new(configuration.email_client.base_url.clone(), sender_email, configuration.email_client.authorization_token.clone(), timeout);
        let port = list.local_addr().unwrap().port();
        let server = run(list, conn_pool, email_client, configuration.application.base_url)?;
        Ok(Self{port, server})
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
    
}

pub struct ApplicationBaseUrl(pub String);

pub fn get_connection_pool(configuration: &DatabaseSettings) ->PgPool{
    PgPoolOptions::new().connect_timeout(std::time::Duration::from_secs(2)).connect_lazy_with(configuration.with_db())
}

pub fn run(listener: TcpListener, db_poop: PgPool, email_client:EmailClient, base_url: String) -> Result<Server,std::io::Error> {
   let db_poop = web::Data::new(db_poop);
   let email_client = web::Data::new(email_client);
   let base_url = web::Data::new(ApplicationBaseUrl(base_url));
   let server=  HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(crate::routes::health_check))
            .route("/subscribe", web::post().to(crate::routes::subscribe))
            .route("/newsletter", web::post().to(crate::routes::publish_newsletter))
            .route("/subscriptions/confirm", web::get().to(crate::routes::confirm))
            .app_data(db_poop.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
    
}
