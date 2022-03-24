use crate::helpers::*;

#[tokio::test]
async fn an_error_message_is_set_on_failure() {
    let app = spawn_app().await;
    
    let login_body = serde_json::json!({
        "username": "gino",
        "password":"pilotino"
    });

    let response = app.post_login(&login_body).await;

    assert_is_redirected_to(&response, "/login");
    let flash_cookie = response.cookies().find(|c| c.name()=="_flash").unwrap();
    assert_eq!(flash_cookie.value(),"Auth failed");
    
    let html_page = app.get_login_html().await;
    
    assert!(html_page.contains("Auth failed"));
}

