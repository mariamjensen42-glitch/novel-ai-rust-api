use actix_web::{test, web, App};
use reqwest::Client;
use serial_test::serial;
use std::sync::Arc;

use novel_ai_rust_api::auth::AuthMiddleware;
use novel_ai_rust_api::config::get_config;
use novel_ai_rust_api::db;
use novel_ai_rust_api::handlers;
use novel_ai_rust_api::models::user::RegisterRequest;

async fn fresh_pool() {
    let pool = db::pool::init_pool().await.expect("init pool");
    sqlx::query("DELETE FROM chapter_characters")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM outline_nodes")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM chapters")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM characters")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM novels")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM projects")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .ok();
    db::pool::set_pool(pool);
}

fn build_app(http: Arc<Client>) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .wrap(AuthMiddleware)
        .app_data(web::Data::new(http))
        .configure(handlers::health::configure)
        .configure(handlers::auth::configure)
        .configure(handlers::projects::configure)
        .configure(handlers::novels::configure)
        .configure(handlers::chapters::configure)
        .configure(handlers::characters::configure)
        .configure(handlers::outlines::configure)
        .configure(handlers::generation::configure)
}

#[actix_web::test]
#[serial]
async fn health_check_returns_200() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
#[serial]
async fn register_login_flow() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "alice@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Alice".to_string(),
    };
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "register failed: {:?}", resp.status());

    let req = test::TestRequest::post()
        .uri("/auth/login")
        .set_json(&serde_json::json!({
            "email": "alice@example.com",
            "password": "secret123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
#[serial]
async fn protected_endpoint_requires_auth() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;
    let req = test::TestRequest::get().uri("/projects").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
#[serial]
async fn project_crud_with_auth() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "bob@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Bob".to_string(),
    };
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let token = resp["token"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({"name": "Project A"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let project_id = body["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::get()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let list: serde_json::Value = test::read_body_json(resp).await;
    assert!(list.as_array().unwrap().iter().any(|p| p["id"] == project_id));

    let req = test::TestRequest::delete()
        .uri(&format!("/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 204);
}

#[actix_web::test]
#[serial]
async fn novel_chapter_cascade() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "carol@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Carol".to_string(),
    };
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let token = resp["token"].as_str().unwrap().to_string();
    let auth = ("Authorization", format!("Bearer {}", token));

    eprintln!("step: create project");
    let req = test::TestRequest::post()
        .uri("/projects")
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"name": "P"}))
        .to_request();
    let project: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let pid = project["id"].as_str().unwrap().to_string();

    eprintln!("step: create novel");
    eprintln!("pid={}", pid);
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}", pid))
        .insert_header(auth.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    eprintln!("GET project status={}", resp.status().as_u16());
    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/novels", pid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "N"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body_bytes = test::read_body(resp).await;
    eprintln!("Novel create status={} body={}", status.as_u16(), String::from_utf8_lossy(&body_bytes));
    let novel: serde_json::Value = serde_json::from_slice(&body_bytes).expect("novel body");
    let nid = novel["id"].as_str().unwrap().to_string();

    eprintln!("step: create chapter 1");
    let req = test::TestRequest::post()
        .uri(&format!("/novels/{}/chapters", nid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "Ch1"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body_bytes = test::read_body(resp).await;
    eprintln!("Ch1 create status={} body={}", status, String::from_utf8_lossy(&body_bytes));
    let chapter: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let cid = chapter["id"].as_str().unwrap().to_string();
    assert_eq!(chapter["order_index"].as_i64().unwrap(), 0);

    eprintln!("step: create chapter 2");
    let req = test::TestRequest::post()
        .uri(&format!("/novels/{}/chapters", nid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "Ch2"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body_bytes = test::read_body(resp).await;
    eprintln!("Ch2 create status={} body={}", status, String::from_utf8_lossy(&body_bytes));
    let chapter2: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(chapter2["order_index"].as_i64().unwrap(), 1);

    let _ = cid;
    let _ = get_config();
}

#[actix_web::test]
#[serial]
async fn unauthorized_user_cannot_access_others_project() {
    fresh_pool().await;
    let app = test::init_service(build_app(Arc::new(Client::new()))).await;

    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&RegisterRequest {
            email: "a@x.com".to_string(),
            password: "secret123".to_string(),
            display_name: "A".to_string(),
        })
        .to_request();
    let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let token_a = resp["token"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&RegisterRequest {
            email: "b@x.com".to_string(),
            password: "secret123".to_string(),
            display_name: "B".to_string(),
        })
        .to_request();
    let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let token_b = resp["token"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .set_json(&serde_json::json!({"name": "A's project"}))
        .to_request();
    let project: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let pid = project["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}", pid))
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);
}
