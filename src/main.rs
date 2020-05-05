use actix_files::NamedFile;
use actix_web::{get, post, web::block, web::Data, web::Form, App, HttpServer, Responder};

#[macro_use]
extern crate diesel;
use diesel::r2d2::{self, ConnectionManager};
use diesel::{insert_into, MysqlConnection, RunQueryDsl};
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

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

#[derive(Serialize, Deserialize)]
struct AccountCreateParams {
    username: String,
    password: String,
}

mod models;
mod schema;

#[post("/acc_create")]
async fn create_account(pool: Data<DbPool>, params: Form<AccountCreateParams>) -> impl Responder {
    let conn = pool.get().expect("Could not get db connection");
    let mut hasher = Sha256::new();
    hasher.input(&params.password);
    let pass_hash = hasher.result();

    match block(move || {
        insert_into(schema::accounts::dsl::accounts)
            .values(&models::Account {
                username: params.username.clone(),
                password_hash: &pass_hash,
            })
            .execute(&conn)
    })
    .await
    {
        Ok(_) => "Account created! <a href='/'>Login</a>",
        Err(_) => "Username exists",
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
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
