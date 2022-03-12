
use zero2prod::configuration::get_configuration;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::startup::{Application};


#[tokio::main]
async fn main() -> std::io::Result<()> {
   
   let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
   init_subscriber(subscriber);
   
   let configuration = get_configuration().expect("Ooops configuration")   ;
   let application = Application::build(configuration.clone()).await?;
   application.run_until_stopped().await?;
   Ok(())
   
   /*let timeout = configuration.email_client.timeout();
   
   let connection_pool = PgPoolOptions::new().connect_timeout(std::time::Duration::from_secs(2)).connect_lazy_with(configuration.database.with_db());
   //let connection_pool = PgPool::connect_lazy(configuration.database.with_db()).expect("Ooop connessione");
   let address = format! ("{}:{}", configuration.application.host, configuration.application.port);
   let list = std::net::TcpListener::bind(address)?;
   let sender_email = configuration.email_client.sender().expect("Ooops sender");
   let email_client = EmailClient::new(configuration.email_client.base_url, sender_email, configuration.email_client.authorization_token, timeout);

   
   zero2prod::startup::run(list, connection_pool, email_client)?.await*/
}
