use std::env;
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


pub mod auction_manager {
    use chrono::NaiveDateTime;
    use serenity::model::id::ChannelId;
    use diesel::prelude::*;
    use crate::models::*;
    use crate::schema::{
        channel_auction::dsl::{channel_auction, channel as channel_col, auction as auction_col},
        demo_auction_info::dsl::{demo_auction_info, id as auction_id_col, last_tend as last_tend_col},
    };
    use crate::utils::PooledPgConnection;

    pub struct TendInfo {
        pub tender_id: u64,
        pub price: i32,
    }

    pub enum GetAuctionError {
        NotAuctionChannel,
        NotHeld,
        InvalidId,
    }

    pub enum TendError {
        LessThanStartPrice,
        LastTendOrLess,
        SameTender,
    }
    
    pub struct AuctionManager {
        pub channel_id: u64,
        pub id: i32,
        pub item: String,
        pub tend: Vec<TendInfo>,
        pub end_time: NaiveDateTime,
        pub start_price: i32,
        pub bin_price: Option<i32>,
    }

    impl AuctionManager {
        pub fn from_channel(conn: &PooledPgConnection, ChannelId(id): ChannelId) -> QueryResult<Result<Self, GetAuctionError>> {
            let result = channel_auction.filter(channel_col.eq(id as i64)).get_result::<ChannelAuction>(conn).optional()?;
            if let Some(ChannelAuction { auction: auction_id, .. }) = result {
                if let Some(auction_id) = auction_id {
                    Ok(Self::from_id(conn, auction_id)?)
                } else {
                    Ok(Err(GetAuctionError::NotHeld))
                }
            } else {
                Ok(Err(GetAuctionError::NotHeld))
            }
        }

        pub fn from_id(conn: &PooledPgConnection, auction_id: i32) -> QueryResult<Result<Self, GetAuctionError>> {
            let auction_info = demo_auction_info.filter(auction_id_col.eq(auction_id)).get_result::<AuctionInfo>(conn).optional()?;
            if let Some(info) = auction_info {
                let mut tend = vec![];
                if info.last_tend >= info.start_price {
                    tend.push(TendInfo { tender_id: 0, price: info.last_tend});
                }
                Ok(Ok(Self {
                    channel_id: 0,
                    id: info.id,
                    item: info.item,
                    tend,
                    end_time: info.end_time,
                    start_price: info.start_price,
                    bin_price: info.bin_price,
                }))
            } else {
                Ok(Err(GetAuctionError::InvalidId))
            }
        }

        pub fn tend(&mut self, conn: &PooledPgConnection, tender_id: u64, tend_price: i32) -> Result<bool, TendError> {

            let mut finish = false;
            if let Some(bin_price) = self.bin_price {
                if tend_price >= bin_price {
                    finish = true;
                }
            }
            
            if let [.., TendInfo { tender_id: last_tender_id, price: last_tend_price }] = self.tend[..] {
                if tender_id == last_tender_id && !finish {
                    return Err(TendError::SameTender);
                } else if tend_price <= last_tend_price {
                    return Err(TendError::LastTendOrLess);
                }
            } else {
                if tend_price < self.start_price {
                    return Err(TendError::LessThanStartPrice);
                }
            }
            let new_tend = TendInfo { tender_id, price: tend_price };
            self.tend.push(new_tend);
            diesel::update(demo_auction_info.find(self.id)).set(last_tend_col.eq(tend_price)).execute(conn).unwrap();
            
            if finish {
                self.finish(conn);
            }
            
            Ok(finish)
        }
        
        pub fn finish(&self, conn: &PooledPgConnection) {
            diesel::update(channel_auction).filter(auction_col.eq(Some(self.id))).set(auction_col.eq(None::<i32>)).execute(conn).unwrap();
        }
    }
}
pub use auction_manager::{AuctionManager, GetAuctionError, TendError};