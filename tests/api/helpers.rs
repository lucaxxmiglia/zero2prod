use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::telemetry::*;


pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
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
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("ooops listener");
    let port = listener.local_addr().unwrap().port();
    let mut configuration = get_configuration().expect("Ooops configuration");
    //conf.database.database_name = format!("{}_test",conf.database.database_name);
    configuration.database.database_name = format!("test_{}", Uuid::new_v4().to_string());
    let connection_pool = configure_database(&configuration.database).await;
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(configuration.email_client.base_url, sender_email, configuration.email_client.authorization_token, timeout);
    //let connection_pool = PgPool:: connect(&configuration.database.connection_string()).await.expect("Failed to connect to Postgres.");

    let server =
        zero2prod::run(listener, connection_pool.clone(), email_client).expect("ooops server");
    let _ = tokio::spawn(server);
    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: connection_pool,
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