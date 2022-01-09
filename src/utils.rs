use std::env;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serenity::prelude::*;
use tokio::sync::Mutex;

pub struct ConnectionMapKey;

impl TypeMapKey for ConnectionMapKey {
    type Value = Mutex<PgConnection>;
}

pub fn establish_connection() -> PgConnection {
    let url = env::var("DATABASE_URL").unwrap();
    PgConnection::establish(&url).unwrap()
}