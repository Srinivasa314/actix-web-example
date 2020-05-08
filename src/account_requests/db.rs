mod schema;
pub use schema::accounts::dsl::{self, accounts};
pub mod models;

use actix_web::{error::BlockingError, web::block, web::Data};

pub use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, PooledConnection};

pub type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

pub async fn change_password(
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

pub async fn delete_user(
    username: String,
    conn: PooledConnection<ConnectionManager<MysqlConnection>>,
) {
    block(move || diesel::delete(accounts.filter(dsl::username.eq(username))).execute(&conn))
        .await
        .expect("Could not delete user");
}

pub async fn get_user_from_database(
    username: String,
    conn: PooledConnection<ConnectionManager<MysqlConnection>>,
) -> Result<Vec<models::Account>, BlockingError<diesel::result::Error>> {
    block(move || {
        accounts
            .filter(dsl::username.eq(username))
            .load::<models::Account>(&conn)
    })
    .await
}

pub fn get_connection(pool: Data<DbPool>) -> PooledConnection<ConnectionManager<MysqlConnection>> {
    pool.get().expect("Could not get db connection")
}
