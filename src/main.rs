use actix_web::{App, HttpServer};
use dotenv::dotenv;

pub mod api;
mod models;
mod config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    HttpServer::new(|| {
        App::new()
            .service(api::routes::predict)
            .service(api::routes::health_check)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
