use crate::helpers::{spawn_app,TestApp, ConfirmationLink};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
    .respond_with(ResponseTemplate::new(200))
    .expect(0)
    .mount(&app.email_server)
    .await;

    let news_body = serde_json::json!({
        "title" : "Newsletter title",
        "content": {
            "text": "news plain text",
            "html":"<p>news html text</p>"
        }
    });

    let response = app.post_newsletter(news_body).await;

    assert_eq!(response.status().as_u16(),200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(any())
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&app.email_server)
    .await;

    let news_body = serde_json::json!({
        "title" : "Newsletter title",
        "content": {
            "text": "news plain text",
            "html":"<p>news html text</p>"
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletter",&app.address))
        .json(&news_body)
        .send()
        .await
        .expect("Oooops exec req");

    assert_eq!(response.status().as_u16(),200);
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;

    let test_cases=vec![
        (serde_json::json!({
            "content": {
                "text": "news plain text",
                "html":"<p>news html text</p>"
            }
        }), "missing title"),
        (serde_json::json!({
            "title": "News"
        }), "missing content"),
    ];

    for (body, message) in test_cases {
        let response = app.post_newsletter(body).await;

        assert_eq!(response.status().as_u16(),400, "API non è fallita con 400 quando il payload era {}", message);
    }
    
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLink{
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .named("Create unconfirmed sub")
    .expect(1)
    .mount_as_scoped(&app.email_server)
    .await;
    app.post_subscription(body.into()).await.error_for_status().unwrap();

    let email_request = &app.email_server.received_requests().await.unwrap().pop().unwrap();
    app.get_confirmation_link(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let conf_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(conf_link.html).await.unwrap().error_for_status().unwrap();
}