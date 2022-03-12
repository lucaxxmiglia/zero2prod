use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock,ResponseTemplate};

//#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscription(body.into()).await;
    
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("select email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Oooops query");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
 
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (body, description) in test_cases {
        // Act
        let response = app.post_subscription(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = test_app.post_subscription(body.into()).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}


#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&test_app.email_server)
    .await;

    let response = test_app.post_subscription(body.into()).await;

    assert_eq!(200, response.status().as_u16());

}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&test_app.email_server)
    .await;

    test_app.post_subscription(body.into()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];

    let body : serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_links = | s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
        .links(s)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
        assert_eq!(links.len(),1);
        links[0].as_str().to_owned()
    };

    let html_link= get_links(&body["HtmlContent"].as_str().unwrap());
    let text_link= get_links(&body["TextContent"].as_str().unwrap());

    assert_eq!(html_link,text_link);    
}