use actix_files::NamedFile;
use actix_web::{get, post, web::block, web::Data, web::Form, App, HttpServer, Responder};

#[macro_use]
extern crate diesel;
use diesel::r2d2::{self, ConnectionManager};
use diesel::{insert_into, MysqlConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    passconfirm: String,
}

mod models;
mod schema;

#[post("/acc_create")]
async fn create_account(pool: Data<DbPool>, params: Form<AccountCreateParams>) -> impl Responder {
    if params.password != params.passconfirm {
        "Passwords do not match"
    } else if params.username.len() > 255 {
        "Username is too long"
    } else {
        let conn = pool.get().expect("Could not get db connection");
        match block(move || {
            let mut hasher = Sha256::new();
            hasher.input(&params.password);
            insert_into(schema::accounts::dsl::accounts)
                .values(&models::Account {
                    username: params.username.clone(),
                    password_hash: &hasher.result(),
                })
                .execute(&conn)
        })
        .await
        {
            Ok(_) => "Account created!",
            Err(_) => "Username exists",
        }
    }
}

#[actix_rt::main]

async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let conn_url = std::env::var("DATABASE_URL").expect("Failed to get value of DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(conn_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool");

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .service(login)
            .service(newacc)
            .service(create_account)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
