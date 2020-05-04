use crate::schema::accounts;

#[derive(Queryable, Insertable)]
pub struct Account<'a> {
    pub username: String,
    pub password_hash: &'a [u8],
}