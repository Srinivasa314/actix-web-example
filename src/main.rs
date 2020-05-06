use actix_files::NamedFile;
use actix_web::{
    error, get, http, post, web::block, web::Data, web::Form, App, HttpResponse, HttpServer,
    Responder, Result,
};

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::{insert_into, MysqlConnection, RunQueryDsl};

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use io::prelude::*;
use schema::accounts::dsl::{self, accounts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io};
use tera::Tera;

#[get("/style.css")]
async fn style() -> impl Responder {
    NamedFile::open("static/style.css")
}

#[get("/")]
async fn index(tmpl: Data<Tera>, id: Identity) -> Result<HttpResponse> {
    if id.identity().is_some() {
        let mut ctx = tera::Context::new();
        ctx.insert("name", &id.identity().unwrap());
        Ok(HttpResponse::Ok().content_type("text/html").body(
            tmpl.render("welcome.html", &ctx)
                .map_err(|_| error::ErrorInternalServerError("Template error"))?,
        ))
    } else {
        Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/login")
            .finish())
    }
}

#[get("/login")]
async fn login() -> impl Responder {
    NamedFile::open("static/login.html")
}

#[get("/newacc")]
async fn newacc() -> impl Responder {
    NamedFile::open("static/newacc.html")
}

#[get("/logout")]
async fn logout(id: Identity) -> impl Responder {
    id.forget();
    NamedFile::open("static/login.html")
}

fn sha256(s: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(s);
    hasher.result().to_vec()
}

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

#[derive(Serialize, Deserialize)]
struct FormParams {
    username: String,
    password: String,
}

mod models;
mod schema;

#[post("/acc_create")]
async fn create_account(pool: Data<DbPool>, params: Form<FormParams>) -> impl Responder {
    let conn = pool.get().expect("Could not get db connection");
    let pass_hash = sha256(&params.password);

    match block(move || {
        insert_into(accounts)
            .values(&models::Account {
                username: params.username.clone(),
                password_hash: pass_hash,
            })
            .execute(&conn)
    })
    .await
    {
        Ok(_) => "Account created! <a href='/'>Login</a>",
        Err(_) => "Username exists",
    }
}

#[post("/login_request")]
async fn login_request(
    id: Identity,
    pool: Data<DbPool>,
    params: Form<FormParams>,
) -> impl Responder {
    let conn = pool.get().expect("Could not get db connection");
    let username = params.username.clone();

    match block(move || {
        accounts
            .filter(dsl::username.eq(username))
            .load::<models::Account>(&conn)
    })
    .await
    {
        Ok(result) => match result.len() {
            0 => Ok("No such user"),
            _ => {
                if result[0].password_hash == sha256(&params.password) {
                    id.remember(params.username.clone());
                    Ok("LOGIN_SUCCESS")
                } else {
                    Ok("Wrong Password")
                }
            }
        },
        Err(e) => Err(e),
    }
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
            .service(newacc)
            .service(login)
            .service(create_account)
            .service(style)
            .service(logout)
            .service(login_request)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
