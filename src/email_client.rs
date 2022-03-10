use crate::domain::SubscriberEmail;
use reqwest::Client;

pub struct EmailClient {
    http_client: Client,
    sender: SubscriberEmail,
    base_url: String,
}

impl EmailClient {
    pub fn new(base_url: String, sender:SubscriberEmail) -> Self{
        Self {
            http_client: Client::new(),
            sender,
            base_url
        }
    }
    pub async fn send_email (
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    ) -> Result<(),String> {
        todo!()
    }
}