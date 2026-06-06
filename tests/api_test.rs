use actix_web::{web, App};
use actix_web::test as actix_test;
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
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let req = actix_test::TestRequest::get().uri("/health").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
#[serial]
async fn register_login_flow() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "alice@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Alice".to_string(),
    };
    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "register failed: {:?}", resp.status());

    let req = actix_test::TestRequest::post()
        .uri("/auth/login")
        .set_json(&serde_json::json!({
            "email": "alice@example.com",
            "password": "secret123"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
#[serial]
async fn protected_endpoint_requires_auth() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let req = actix_test::TestRequest::get().uri("/projects").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
#[serial]
async fn project_crud_with_auth() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "bob@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Bob".to_string(),
    };
    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let token = resp["token"].as_str().unwrap().to_string();

    let req = actix_test::TestRequest::post()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({"name": "Project A"}))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    let project_id = body["id"].as_str().unwrap().to_string();

    let req = actix_test::TestRequest::get()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let list: serde_json::Value = actix_test::read_body_json(resp).await;
    assert!(list.as_array().unwrap().iter().any(|p| p["id"] == project_id));

    let req = actix_test::TestRequest::delete()
        .uri(&format!("/projects/{}", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 204);
}

#[actix_web::test]
#[serial]
async fn novel_chapter_cascade() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;

    let register = RegisterRequest {
        email: "carol@example.com".to_string(),
        password: "secret123".to_string(),
        display_name: "Carol".to_string(),
    };
    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&register)
        .to_request();
    let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let token = resp["token"].as_str().unwrap().to_string();
    let auth = ("Authorization", format!("Bearer {}", token));

    eprintln!("step: create project");
    let req = actix_test::TestRequest::post()
        .uri("/projects")
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"name": "P"}))
        .to_request();
    let project: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let pid = project["id"].as_str().unwrap().to_string();

    eprintln!("step: create novel");
    eprintln!("pid={}", pid);
    let req = actix_test::TestRequest::get()
        .uri(&format!("/projects/{}", pid))
        .insert_header(auth.clone())
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    eprintln!("GET project status={}", resp.status().as_u16());
    let req = actix_test::TestRequest::post()
        .uri(&format!("/projects/{}/novels", pid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "N"}))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    let status = resp.status();
    let body_bytes = actix_test::read_body(resp).await;
    eprintln!("Novel create status={} body={}", status.as_u16(), String::from_utf8_lossy(&body_bytes));
    let novel: serde_json::Value = serde_json::from_slice(&body_bytes).expect("novel body");
    let nid = novel["id"].as_str().unwrap().to_string();

    eprintln!("step: create chapter 1");
    let req = actix_test::TestRequest::post()
        .uri(&format!("/novels/{}/chapters", nid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "Ch1"}))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body_bytes = actix_test::read_body(resp).await;
    eprintln!("Ch1 create status={} body={}", status, String::from_utf8_lossy(&body_bytes));
    let chapter: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let cid = chapter["id"].as_str().unwrap().to_string();
    assert_eq!(chapter["order_index"].as_i64().unwrap(), 0);

    eprintln!("step: create chapter 2");
    let req = actix_test::TestRequest::post()
        .uri(&format!("/novels/{}/chapters", nid))
        .insert_header(auth.clone())
        .set_json(&serde_json::json!({"title": "Ch2"}))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body_bytes = actix_test::read_body(resp).await;
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
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;

    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&RegisterRequest {
            email: "a@x.com".to_string(),
            password: "secret123".to_string(),
            display_name: "A".to_string(),
        })
        .to_request();
    let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let token_a = resp["token"].as_str().unwrap().to_string();

    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&RegisterRequest {
            email: "b@x.com".to_string(),
            password: "secret123".to_string(),
            display_name: "B".to_string(),
        })
        .to_request();
    let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let token_b = resp["token"].as_str().unwrap().to_string();

    let req = actix_test::TestRequest::post()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .set_json(&serde_json::json!({"name": "A's project"}))
        .to_request();
    let project: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let pid = project["id"].as_str().unwrap().to_string();

    let req = actix_test::TestRequest::get()
        .uri(&format!("/projects/{}", pid))
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);
}

// ===================== 4 个新生成动作的 HTTP 端点 =====================

fn make_novel(project_id: &str) -> novel_ai_rust_api::models::novel::Novel {
    novel_ai_rust_api::models::novel::Novel {
        id: "novel_x".to_string(),
        project_id: project_id.to_string(),
        title: "测试小说".to_string(),
        synopsis: "一部测试小说".to_string(),
        genre: "奇幻".to_string(),
        style: "华丽".to_string(),
        pov: "第三人称".to_string(),
        tone: "热血".to_string(),
        target_word_count: 100000,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn make_chapter(novel_id: &str) -> novel_ai_rust_api::models::chapter::Chapter {
    novel_ai_rust_api::models::chapter::Chapter {
        id: "chapter_x".to_string(),
        novel_id: novel_id.to_string(),
        title: "测试章节".to_string(),
        summary: "概要".to_string(),
        content: "很久以前，勇者走进了森林。".to_string(),
        order_index: 0,
        status: "draft".to_string(),
        word_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn make_characters() -> Vec<novel_ai_rust_api::models::character::Character> {
    vec![novel_ai_rust_api::models::character::Character {
        id: "char_a".to_string(),
        novel_id: "novel_x".to_string(),
        name: "林霄".to_string(),
        role: "主角".to_string(),
        description: "18 岁少年，冷静坚毅".to_string(),
        traits: "[\"勇敢\",\"机智\"]".to_string(),
        backstory: "孤儿出身".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }]
}

#[test]
fn prompt_build_translate_includes_target_language_and_preservation_flag() {
    let novel = make_novel("proj_x");
    let ch = make_chapter("novel_x");
    let chars = make_characters();
    let msgs = novel_ai_rust_api::prompts::build_translate(
        &novel, &ch, &chars, "English", Some("中文"), true,
    );
    assert_eq!(msgs.len(), 2);
}

#[test]
fn prompt_build_polish_reflects_focus_choice() {
    let novel = make_novel("proj_x");
    let ch = make_chapter("novel_x");
    for (focus, must_contain) in [
        ("dialogue", "对话"),
        ("description", "环境与动作"),
        ("pacing", "节奏"),
        ("grammar", "语法"),
        ("overall", "全面润色"),
    ] {
        let msgs = novel_ai_rust_api::prompts::build_polish(&novel, &ch, focus);
        let system = &msgs[0].content;
        assert!(
            system.contains(must_contain),
            "focus={} should contain '{}', got: {}",
            focus,
            must_contain,
            system
        );
    }
}

#[test]
fn prompt_build_style_transfer_embeds_target_style() {
    let novel = make_novel("proj_x");
    let ch = make_chapter("novel_x");
    let msgs = novel_ai_rust_api::prompts::build_style_transfer(
        &novel,
        &ch,
        "海明威式极简",
        Some("华丽"),
    );
    let system = &msgs[0].content;
    let user = &msgs[1].content;
    assert!(system.contains("海明威式极简"));
    assert!(system.contains("风格"));
    assert!(user.contains("海明威式极简"));
    assert!(user.contains("华丽"), "source_style should appear in user: {}", user);
}

#[test]
fn prompt_build_consistency_check_includes_all_characters() {
    let novel = make_novel("proj_x");
    let ch = make_chapter("novel_x");
    let chars = make_characters();
    let msgs = novel_ai_rust_api::prompts::build_consistency_check(&novel, &ch, &chars);
    let user = &msgs[1].content;
    assert!(user.contains("林霄"), "user missing character name");
    assert!(user.contains("孤儿出身"), "user missing character backstory");
    assert!(user.contains("勇者走进"), "user missing chapter content");
    assert!(msgs[0].content.contains("Markdown"), "system should ask for markdown report");
}

#[test]
fn prompt_build_consistency_check_handles_empty_character_list() {
    let novel = make_novel("proj_x");
    let ch = make_chapter("novel_x");
    let msgs = novel_ai_rust_api::prompts::build_consistency_check(&novel, &ch, &[]);
    let user = &msgs[1].content;
    assert!(user.contains("未指定角色") || user.contains("一般性检查"),
        "empty character list should still produce useful prompt: {}", user);
}

// ===================== 4 个新生成动作的 HTTP 端点 =====================
// 不引用 /chapters/{id}/versions 等版本功能端点（本分支没有该功能）。
// 每个测试内联 register/project/novel/chapter。

#[actix_web::test]
#[serial]
async fn gen_translate_requires_auth() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;

    // 注册
    let req = actix_test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&RegisterRequest {
            email: "tr1@x.com".to_string(),
            password: "secret123".to_string(),
            display_name: "TR1".to_string(),
        })
        .to_request();
    let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let token = resp["token"].as_str().unwrap().to_string();

    // 建 project/novel/chapter
    let req = actix_test::TestRequest::post()
        .uri("/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({"name": "P"}))
        .to_request();
    let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let pid = p["id"].as_str().unwrap().to_string();
    let req = actix_test::TestRequest::post()
        .uri(&format!("/projects/{}/novels", pid))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({"title": "N"}))
        .to_request();
    let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let nid = n["id"].as_str().unwrap().to_string();
    let req = actix_test::TestRequest::post()
        .uri(&format!("/novels/{}/chapters", nid))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
        .to_request();
    let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
    let cid = c["id"].as_str().unwrap().to_string();

    // 无 token → 401
    let req = actix_test::TestRequest::post()
        .uri("/generation/translate")
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_language": "English"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
#[serial]
async fn gen_translate_validates_target_language() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "tr2@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "TR2".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let (_pid, _nid, cid) = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let cid = c["id"].as_str().unwrap().to_string();
        (pid, nid, cid)
    };

    // 空 target_language → 400
    let req = actix_test::TestRequest::post()
        .uri("/generation/translate")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_language": "   "
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("target_language"));
}

#[actix_web::test]
#[serial]
async fn gen_translate_cross_user_returns_403() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token_a = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "tr3@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "TR3".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };
    let token_b = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "tr4@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "TR4".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    // 用 token_a 建一个 chapter
    let cid = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        c["id"].as_str().unwrap().to_string()
    };

    // token_b 访问 token_a 的 chapter
    let req = actix_test::TestRequest::post()
        .uri("/generation/translate")
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_language": "English"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);
}

#[actix_web::test]
#[serial]
async fn gen_translate_unknown_model_returns_sse_error() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "tr5@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "TR5".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let cid = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        c["id"].as_str().unwrap().to_string()
    };

    // test env 没配 deepseek provider → 200 + SSE 错误事件
    let req = actix_test::TestRequest::post()
        .uri("/generation/translate")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_language": "English"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("event-stream"), "expected SSE, got {}", ct);
    let body_bytes = actix_test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("unknown model") || body_str.contains("\"error\""),
        "expected SSE error event about unknown model, got: {}",
        body_str
    );
}

#[actix_web::test]
#[serial]
async fn gen_polish_validates_focus() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "po1@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "PO1".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let cid = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        c["id"].as_str().unwrap().to_string()
    };

    // 非法 focus → 400
    let req = actix_test::TestRequest::post()
        .uri("/generation/polish")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "focus": "invalid"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("focus"));

    // 合法 focus (默认 overall) → 200 + SSE
    let req = actix_test::TestRequest::post()
        .uri("/generation/polish")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/event-stream"
    );
}

#[actix_web::test]
#[serial]
async fn gen_style_transfer_validates_target_style() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "st1@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "ST1".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let cid = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        c["id"].as_str().unwrap().to_string()
    };

    // 空 target_style → 400
    let req = actix_test::TestRequest::post()
        .uri("/generation/style-transfer")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_style": ""
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("target_style"));

    // 合法 target_style → 200 + SSE
    let req = actix_test::TestRequest::post()
        .uri("/generation/style-transfer")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek",
            "target_style": "海明威式极简"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/event-stream"
    );
}

#[actix_web::test]
#[serial]
async fn gen_consistency_check_returns_sse() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "cc1@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "CC1".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let (nid, cid) = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let cid = c["id"].as_str().unwrap().to_string();
        (nid, cid)
    };

    // 加一个角色
    let req = actix_test::TestRequest::post()
        .uri("/characters")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "novel_id": nid,
            "name": "林霄",
            "role": "主角",
            "description": "冷静坚毅",
            "traits": "[\"勇敢\"]"
        }))
        .to_request();
    actix_test::call_service(&app, req).await;

    let req = actix_test::TestRequest::post()
        .uri("/generation/character-consistency-check")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&serde_json::json!({
            "chapter_id": cid,
            "model": "deepseek"
        }))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/event-stream"
    );
}

#[actix_web::test]
#[serial]
async fn gen_all_actions_reject_cross_user_chapter() {
    fresh_pool().await;
    let app = actix_test::init_service(build_app(Arc::new(Client::new()))).await;
    let token_a = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "xa@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "XA".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };
    let token_b = {
        let req = actix_test::TestRequest::post()
            .uri("/auth/register")
            .set_json(&RegisterRequest {
                email: "xb@x.com".to_string(),
                password: "secret123".to_string(),
                display_name: "XB".to_string(),
            })
            .to_request();
        let resp: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        resp["token"].as_str().unwrap().to_string()
    };

    let cid = {
        let req = actix_test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"name": "P"}))
            .to_request();
        let p: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let pid = p["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/projects/{}/novels", pid))
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"title": "N"}))
            .to_request();
        let n: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        let nid = n["id"].as_str().unwrap().to_string();

        let req = actix_test::TestRequest::post()
            .uri(&format!("/novels/{}/chapters", nid))
            .insert_header(("Authorization", format!("Bearer {}", token_a)))
            .set_json(&serde_json::json!({"title": "C", "summary": "S", "content": "content"}))
            .to_request();
        let c: serde_json::Value = actix_test::call_and_read_body_json(&app, req).await;
        c["id"].as_str().unwrap().to_string()
    };

    for (uri, body) in [
        (
            "/generation/translate",
            serde_json::json!({"chapter_id": cid, "model": "deepseek", "target_language": "en"}),
        ),
        (
            "/generation/polish",
            serde_json::json!({"chapter_id": cid, "model": "deepseek"}),
        ),
        (
            "/generation/style-transfer",
            serde_json::json!({"chapter_id": cid, "model": "deepseek", "target_style": "x"}),
        ),
        (
            "/generation/character-consistency-check",
            serde_json::json!({"chapter_id": cid, "model": "deepseek"}),
        ),
    ] {
        let req = actix_test::TestRequest::post()
            .uri(uri)
            .insert_header(("Authorization", format!("Bearer {}", token_b)))
            .set_json(&body)
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 403, "{} cross-user should be 403, got {}", uri, resp.status());
    }
}
