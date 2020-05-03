use actix_web::{get, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn login() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/login.html"))
}

#[get("/newacc")]
async fn newacc() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/newacc.html"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(login).service(newacc))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}
