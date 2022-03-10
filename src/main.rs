use zero2prod::run; 
use zero2prod::configuration::get_configuration;
use sqlx::{PgPool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};


#[tokio::main]
async fn main() -> std::io::Result<()> {
   
   let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
   init_subscriber(subscriber);
   
   let configuration = get_configuration().expect("Ooops configuration")   ;
   let connection_pool = PgPool::connect_lazy(&configuration.database.connection_string()).expect("Ooop connessione");
   let address = format! ("{}:{}", configuration.application.host, configuration.application.port);
   let list = std::net::TcpListener::bind(address)?;
   run(list, connection_pool)?.await
}
