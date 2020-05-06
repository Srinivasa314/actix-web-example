use actix_files::{Files, NamedFile};
use actix_web::{
    error::BlockingError, get, http, post, web::block, web::Data, web::Form, App, HttpResponse,
    HttpServer, Responder,
};

#[macro_use]
extern crate diesel;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, PooledConnection};
use diesel::{insert_into, MysqlConnection, RunQueryDsl};

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use io::prelude::*;
use schema::accounts::dsl::{self, accounts};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
            .header(http::header::LOCATION, "/login.html")
            .finish()
    }
}

#[get("/logout")]
async fn logout(id: Identity) -> impl Responder {
    id.forget();
    NamedFile::open("static/login.html")
}

type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;
mod models;
mod schema;

fn sha256(s: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(s);
    hasher.result().to_vec()
}

async fn get_user_from_database(
    username: String,
    pool: &DbPool,
) -> Result<Vec<models::Account>, BlockingError<diesel::result::Error>> {
    block(move || {
        let conn = pool.get().expect("Could not get db connection");
        accounts
            .filter(dsl::username.eq(username))
            .load::<models::Account>(&conn)
    })
    .await
}

#[derive(Serialize, Deserialize)]
struct FormParams {
    username: String,
    password: String,
}

#[post("/acc_create")]
async fn create_account(pool: Data<DbPool>, params: Form<FormParams>) -> impl Responder {
    let pass_hash = sha256(&params.password);

    match block(move || {
        let conn = pool.get().expect("Could not get db connection");
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
    let username = params.username.clone();

    match get_user_from_database(username, &pool).await {
        Ok(result) => match result.len() {
            0 => "No such user",
            _ => {
                if result[0].password_hash == sha256(&params.password) {
                    id.remember(params.username.clone());
                    "LOGIN_SUCCESS"
                } else {
                    "Wrong Password"
                }
            }
        },
        Err(e) => panic!(e),
    }
}

#[derive(Serialize, Deserialize)]
struct ChangePassParams {
    oldpass: String,
    newpass: String,
}

#[post("/chpass_request")]
async fn chpass_request(
    id: Identity,
    pool: Data<DbPool>,
    params: Form<ChangePassParams>,
) -> impl Responder {
    let pool1 = pool.clone();
    match id.identity() {
        None => "Invalid session",
        Some(username) => {
            let name = username.clone();
            match get_user_from_database(username, &pool).await {
                Ok(result) => {
                    if result[0].password_hash == sha256(&params.oldpass) {
                        change_password(
                            name,
                            sha256(&params.newpass),
                            pool1.get().expect("Could not get db connection"),
                        )
                        .await;
                        "Password changed <a href='/'>Go Back</a>"
                    } else {
                        "Wrong Password"
                    }
                }
                Err(e) => panic!(e),
            }
        }
    }
}

async fn change_password(
    username: String,
    newpasshash: Vec<u8>,
    conn: PooledConnection<ConnectionManager<MysqlConnection>>,
) {
    block(move || {
        diesel::update(accounts.filter(dsl::username.eq(username)))
            .set(dsl::password_hash.eq(newpasshash))
            .execute(&conn)
    })
    .await
    .expect("Unable to change password");
}

#[post("/confirm_delacc")]
async fn confirm_delacc(id: Identity, pool: Data<DbPool>, body: bytes::Bytes) -> impl Responder {
    let password = String::from_utf8_lossy(&body);

    match id.identity() {
        None => "Invalid session",
        Some(username) => {
            let pool1 = pool.clone();
            let name = username.clone();
            match block(move || {
                let conn = pool.get().expect("Could not get db connection");
                accounts
                    .filter(dsl::username.eq(name))
                    .load::<models::Account>(&conn)
            })
            .await
            {
                Ok(result) => {
                    if result[0].password_hash == sha256(&password) {
                        delete_user(username, pool1.get().expect("Could not get db connection"))
                            .await;
                        id.forget();
                        "Account deleted"
                    } else {
                        "Wrong Password"
                    }
                }
                Err(e) => panic!(e),
            }
        }
    }
}

async fn delete_user(username: String, conn: PooledConnection<ConnectionManager<MysqlConnection>>) {
    block(move || diesel::delete(accounts.filter(dsl::username.eq(username))).execute(&conn))
        .await
        .expect("Could not delete user");
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
            .service(Files::new("/", "static/"))
            .service(logout)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
