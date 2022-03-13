use crate::domain::SubscriberEmail;
use reqwest::Client;
//use secrecy::Secret;

pub struct EmailClient {
    http_client: Client,
    sender: SubscriberEmail,
    base_url: String,
    authorization_token: String,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all="PascalCase")]
pub struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_content: &'a str,
    text_content: &'a str,
}

impl EmailClient {
    pub fn new(base_url: String, sender:SubscriberEmail, authorization_token: String, timeout: std::time::Duration) -> Self{
        Self {
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            sender,
            base_url,
            authorization_token
        }
    }

    pub async fn send_email (
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    ) -> Result<(),reqwest::Error> {
        let url= format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: subject,
            html_content: html_content,
            text_content: text_content,
        };
        
        let _builder = self.http_client.post(&url).header("X-Postmark-Server-Token", self.authorization_token.clone()).json(&request_body).send().await?.error_for_status();
        
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::{header,header_exists,path,method, any};
    use claim::{assert_err,assert_ok};
    use wiremock::{Mock, MockServer, ResponseTemplate} ;
    use wiremock::Request;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value,_> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                //dbg!(&body);
                body.get("From").is_some()
                && body.get("To").is_some()
                && body.get("Subject").is_some()
                && body.get("HtmlContent").is_some()
                && body.get("TextContent").is_some()
            } else {
                false
            }
        }
    }

    fn subject()->String {
        Sentence(1..2).fake()
    }

    fn content()->String {
        Paragraph(1..10).fake()
    }

    fn email()->SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String)->EmailClient {
        EmailClient::new(base_url,email(), Faker.fake(),std::time::Duration::from_secs(1))
    }

    //use secrecy::Secret;
    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        //Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
        .and(header("Content-Type","application/json"))
        .and(path("/email"))
        .and(method("POST"))
        .and(SendEmailBodyMatcher)
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

        let _ = email_client .send_email(email(), &subject(), &content(),&content()).await;

    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        //Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());


        Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

       
        let outcome = email_client .send_email(email(), &subject(), &content(),&content()).await;

        assert_ok!(outcome);

    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

        let outcome = email_client .send_email(email(), &subject(), &content(),&content()).await;

        assert_err!(outcome);

    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
        .respond_with(response)
        .expect(1)
        .mount(&mock_server)
        .await;

        let outcome = email_client .send_email(email(), &subject(), &content(),&content()).await;

        assert_err!(outcome);

    }
}