mod account_requests;
mod db;

pub mod models;
mod schema;
use account_requests::*;

use actix_files::{Files, NamedFile};
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::{get, http, web::Data, App, HttpResponse, HttpServer, Responder};

#[macro_use]
extern crate diesel;
use diesel::r2d2::{self, ConnectionManager};
use diesel::MysqlConnection;

use io::prelude::*;
use std::{fs, io};
use tera::Tera;

#[get("/")]
async fn index(tmpl: Data<Tera>, id: Identity) -> impl Responder {
    if id.identity().is_some() {
        let mut ctx = tera::Context::new();
        ctx.insert("name", &id.identity().unwrap());
        HttpResponse::Ok()
            .content_type("text/html")
            .body(tmpl.render("welcome.html", &ctx).expect("Template error"))
    } else {
        HttpResponse::Found()
            .header(http::header::LOCATION, "/static/login.html")
            .finish()
    }
}

#[get("/logout")]
async fn logout(id: Identity) -> impl Responder {
    id.forget();
    NamedFile::open("static/login.html")
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    let conn_url = std::env::var("DATABASE_URL").expect("Failed to get value of DATABASE_URL");

    let private_key = match fs::read("cookie-key") {
        Ok(bytes) => bytes,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                let mut f =
                    fs::File::create("cookie-key").expect("Unable to create cookie key file");
                let key: [u8; 32] = rand::random();

                f.write(&key).expect("Unable to write to file");
                key.to_vec()
            } else {
                panic!(e)
            }
        }
    };

    HttpServer::new(move || {
        let manager = ConnectionManager::<MysqlConnection>::new(&conn_url);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool");
        let tera = Tera::new("templates/*.html").expect("Failed to parse template files");

        App::new()
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&private_key)
                    .name("actix-web-example")
                    .secure(false)
                    .max_age(31556952),
            ))
            .data(pool)
            .data(tera)
            .service(index)
            .service(create_account)
            .service(confirm_delacc)
            .service(login_request)
            .service(chpass_request)
            .service(Files::new("/static", "static/"))
            .service(logout)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
