use std::env;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2;
use r2d2::{Pool, PooledConnection, ConnectionManager};
use serenity::prelude::*;
use serenity::async_trait;


type PgManager = ConnectionManager<PgConnection>;
type PgConnectionPool = Pool<PgManager>;
type PooledPgConnection = PooledConnection<PgManager>;

struct PoolKey;
impl TypeMapKey for PoolKey {
    type Value = PgConnectionPool;
}

pub fn establish_connection() -> PgConnection {
    let url = env::var("DATABASE_URL").unwrap();
    PgConnection::establish(&url).unwrap()
}


#[async_trait]
pub trait GetConnection {
    async fn get_connection(&self) -> PooledPgConnection {
        self.pool().await.get().unwrap()
    }
    async fn pool(&self) -> PgConnectionPool;
}

#[async_trait]
impl GetConnection for Context {
    async fn pool(&self) -> PgConnectionPool {
        let data = self.data.read().await;
        data.get::<PoolKey>().unwrap().clone()
    }
}

pub async fn insert_pool(client: &Client) {
    let url = env::var("DATABASE_URL").unwrap();
    let manager = PgManager::new(url);
    let pool = Pool::builder().max_size(4).build(manager).unwrap();
    let mut data = client.data.write().await;
    data.insert::<PoolKey>(pool);
}