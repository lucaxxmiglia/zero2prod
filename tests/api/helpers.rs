use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::telemetry::*;
use zero2prod::startup::{get_connection_pool, Application};
use wiremock::MockServer;


pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16
}

pub struct ConfirmationLink {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        let client = reqwest::Client::new();
        let response = client
        .post(&format!("{}/subscribe", &self.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Ooops request");

        response
    }

    pub fn get_confirmation_link(&self, email_request: &wiremock::Request) -> ConfirmationLink {
        let body : serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_links = | s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
            assert_eq!(links.len(),1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url:: parse(&raw_link).unwrap();
            assert_eq! (confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link

        };
    
        let html_link= get_links(&body["HtmlContent"].as_str().unwrap());
        let text_link= get_links(&body["TextContent"].as_str().unwrap());
        ConfirmationLink {
            html: html_link,
            plain_text: text_link
        }
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let  configuration = {
        let mut c = get_configuration().expect("Ooops configuration");
        c.database.database_name= format!("test_{}", Uuid::new_v4().to_string());
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone()).await.expect("Ooops application");
    let address = format!("http://127.0.0.1:{}", application.port());
    let app_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());
    

    TestApp {
        address:address,
        db_pool:  get_connection_pool(&configuration.database),
        email_server,
        port: app_port
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.whithout_db())
        .await
        .expect("Oooops database");
    let urk = format!(r#"DROP DATABASE IF EXISTS "{}";"#, config.database_name);
    connection.execute(urk.as_str()).await.expect("oops drop");
    let urk = format!(r#"CREATE DATABASE "{}";"#, config.database_name);
    connection.execute(urk.as_str()).await.expect("oops create");
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Oooops migration");
    connection_pool
}

