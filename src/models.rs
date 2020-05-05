use crate::schema::accounts;

#[derive(Queryable, Insertable)]
pub struct Account {
    pub username: String,
    pub password_hash: Vec<u8>,
}