pub use diesel::prelude::*;
pub use diesel::r2d2::{self, ConnectionManager, PooledConnection};
pub use diesel::{insert_into, MysqlConnection, RunQueryDsl};
use actix_web::error::BlockingError;
use super::*;

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

pub async fn delete_user(username: String, conn: PooledConnection<ConnectionManager<MysqlConnection>>) {
    block(move || diesel::delete(accounts.filter(dsl::username.eq(username))).execute(&conn))
        .await
        .expect("Could not delete user");
}

pub async fn get_user_from_database(
    username: String,
    pool: Data<DbPool>,
) -> Result<Vec<models::Account>, BlockingError<diesel::result::Error>> {
    block(move || {
        let conn = pool.get().expect("Could not get db connection");
        accounts
            .filter(dsl::username.eq(username))
            .load::<models::Account>(&conn)
    })
    .await
}
