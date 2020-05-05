use actix_files::NamedFile;
use actix_web::{get, post, web::block, web::Data, web::Form, App, HttpServer, Responder};

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::{insert_into, MysqlConnection, RunQueryDsl};
use schema::accounts::dsl::{self, accounts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[get("/style.css")]
async fn style() -> impl Responder {
    NamedFile::open("static/style.css")
}

#[get("/")]
async fn login() -> impl Responder {
    NamedFile::open("static/login.html")
}

#[get("/newacc")]
async fn newacc() -> impl Responder {
    NamedFile::open("static/newacc.html")
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
async fn login_request(pool: Data<DbPool>, params: Form<FormParams>) -> impl Responder {
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

async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        dotenv::dotenv().ok();
        let conn_url = std::env::var("DATABASE_URL").expect("Failed to get value of DATABASE_URL");
        let manager = ConnectionManager::<MysqlConnection>::new(conn_url);

        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool");

        App::new()
            .data(pool)
            .service(login)
            .service(newacc)
            .service(create_account)
            .service(style)
            .service(login_request)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
