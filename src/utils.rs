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
    use serenity::prelude::*;
    use serenity::model::id::ChannelId;
    use diesel::prelude::*;
    use super::GetConnection;
    use crate::models::*;
    use crate::schema::{
        channel_auction::dsl::{channel_auction, channel as channel_col, auction as auction_col},
        auction_info::dsl::{auction_info as info_table, id as auction_id_col, tenders_id as tenders_id_col, tends_price as tends_price_col},
    };
    use crate::utils::PooledPgConnection;

    pub struct TendInfo {
        pub tender_id: u64,
        pub price: i32,
    }

    #[derive(Debug)]
    pub enum GetAuctionError {
        NotAuctionChannel,
        NotHeld,
        InvalidId,
    }

    #[derive(Debug)]
    pub enum TendError {
        LessThanStartPrice,
        LastTendOrLess,
        SameTender,
        ByOwner,
    }
    
    pub struct AuctionManager {
        pub channel_id: u64,
        pub id: i32,
        pub owner_id: u64,
        pub item: String,
        pub unit: String,
        pub tend: Vec<TendInfo>,
        pub end_time: NaiveDateTime,
        pub start_price: i32,
        pub bin_price: Option<i32>,
        pub notice: String,
        pub embed_id: u64,
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
            let auction_info = info_table.filter(auction_id_col.eq(auction_id)).get_result::<AuctionInfo>(conn).optional()?;
            if let Some(info) = auction_info {
                let mut tend = vec![];
                for (tender_id, price) in info.tenders_id.into_iter().zip(info.tends_price) {
                    tend.push(TendInfo { tender_id: tender_id as u64, price })
                }
                Ok(Ok(Self {
                    channel_id: info.channel_id as u64,
                    id: info.id,
                    owner_id: info.owner_id as u64,
                    item: info.item,
                    tend,
                    end_time: info.end_time,
                    start_price: info.start_price,
                    bin_price: info.bin_price,
                    embed_id: info.embed_id.unwrap() as u64,
                    unit: info.unit,
                    notice: info.notice,
                }))
            } else {
                Ok(Err(GetAuctionError::InvalidId))
            }
        }

        pub fn tend(&mut self, conn: &PooledPgConnection, tender_id: u64, tend_price: i32) -> Result<bool, TendError> {

            if tender_id == self.owner_id {
                return Err(TendError::ByOwner);
            }

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
            let mut tenders_id = vec![];
            let mut tends_price = vec![];
            for TendInfo { tender_id, price } in self.tend.iter() {
                tenders_id.push(*tender_id as i64);
                tends_price.push(*price);
            }
            diesel::update(info_table.find(self.id)).set((tenders_id_col.eq(tenders_id), tends_price_col.eq(tends_price)))
                .execute(conn).unwrap();
            
            Ok(finish)
        }
        
        pub async fn finish(&self, ctx: &Context) {
            let conn = ctx.get_connection().await;
            diesel::update(channel_auction).filter(auction_col.eq(Some(self.id))).set(auction_col.eq(None::<i32>)).execute(&conn).unwrap();
            ChannelId(self.channel_id).unpin(ctx, self.embed_id).await.unwrap();
        }
    }
}
pub use auction_manager::{AuctionManager, GetAuctionError, TendError};


pub mod formats {
    use chrono::Duration;
    use regex::Regex;
    use std::collections::HashMap;
    use serenity::prelude::*;
    use serenity::model::{guild::{Member, Guild}, user::User};

    const DATETIME_PATTERN: &str = r"^(?P<year>\d{4})[-/](?P<month>\d{1,2})[-/](?P<day>\d{1,2})[-\stT](?P<hour>\d{1,2}):(?P<minute>\d{1,2})$";
    const DURATION_PATTERN: &str = 
        r"^(?ix)
        ((?P<month>\d{1,4})(?P<m_unit>M))?
        ((?P<week>\d{1,5})w)?
        ((?P<day>\d{1,5})d)?
        ((?P<hour>\d{1,5})h)?
        ((?P<minute>\d{1,5})m)?
        $";
    const STACK_PATTERN: &str = r"^(?P<value>\d{1,8})(?P<unit>(st|lc)?)$";

    pub fn datetime(text: &str) -> Option<(i32, u32, u32, u32, u32)> {
        let pattern = Regex::new(DATETIME_PATTERN).unwrap();
        let cap = match pattern.captures(text) {
            Some(cap) => cap,
            None => return None,
        };
        let year = cap.name("year").unwrap().as_str().parse().unwrap();
        let month = cap.name("month").unwrap().as_str().parse().unwrap();
        let day = cap.name("day").unwrap().as_str().parse().unwrap();
        let hour = cap.name("hour").unwrap().as_str().parse().unwrap();
        let minute = cap.name("minute").unwrap().as_str().parse().unwrap();
        Some((year, month, day, hour, minute))
    }

    pub fn duration(text: &str) -> Option<(i32, Duration)> {
        if text.is_empty() {
            return None;
        }
        let pattern = Regex::new(DURATION_PATTERN).unwrap();
        let cap = match pattern.captures(text) {
            Some(cap) => cap,
            None => return None,
        };
        if [cap.name("week"), cap.name("day"), cap.name("hour"), cap.name("minute")].iter()
        .all(Option::is_none) && cap.name("month").is_some() && cap.name("m_unit").unwrap().as_str() == "m" {
            return Some((0, Duration::minutes(cap.name("month").unwrap().as_str().parse().unwrap())));
        }
        let month = cap.name("month").map(|s| s.as_str().parse().unwrap()).unwrap_or(0);
        let mut duration = Duration::zero();
        if let Some(week) = cap.name("week") {
            duration = duration + Duration::weeks(week.as_str().parse().unwrap());
        }
        if let Some(day) = cap.name("day") {
            duration = duration + Duration::days(day.as_str().parse().unwrap());
        }
        if let Some(hour) = cap.name("hour") {
            duration = duration + Duration::hours(hour.as_str().parse().unwrap());
        }
        if let Some(minute) = cap.name("minute") {
            duration = duration + Duration::minutes(minute.as_str().parse().unwrap());
        }

        Some((month, duration))
    }

    pub fn stack_to_int(text: &str) -> Option<i32> {
        let units = HashMap::from([
            ("", 1),
            ("st", 64),
            ("lc", 3456),
        ]);
        let pattern = Regex::new(STACK_PATTERN).unwrap();
        let mut res = 0;

        for term in text.to_lowercase().split("+").map(str::trim) {
            if let Some(cap) = pattern.captures(term) {
                let value: i32 = cap.name("value").unwrap().as_str().parse().unwrap();
                let unit = units.get(cap.name("unit").unwrap().as_str()).unwrap();
                res += unit * value;
            } else {
                return None
            }
        }
        Some(res)
    }

    pub fn int_to_stack(mut value: i32) -> String {
        let lc = value / 3456;
        value %= 3456;
        let st = value / 64;
        value %= 64;
        
        let mut res = vec![];
        if lc > 0 {
            res.push(format!("{}LC", lc))
        }
        if st > 0 {
            res.push(format!("{}st", st))
        }
        if value > 0 {
            res.push(format!("{}個", value))
        }

        res.join("+")
    }

    pub fn stack_with_raw(value: i32) -> String {
        let mut res = int_to_stack(value);
        if value >= 64 {
            res.push_str(&format!(" ({})", value));
        }
        res
    }

    pub fn last_day(year: i32, month: u32) -> u32 {
        if month == 2 {
            if year%400==0 || year%100!=0 && year%4==0 {
                29
            } else {
                28
            }
        } else if [4, 6, 9, 11].contains(&month) {
            30
        } else {
            31
        }
    }

    pub fn get_nick(member: &Member) -> &str {
        member.nick.as_ref().unwrap_or(&member.user.name)
    }

    pub async fn display_name(ctx: &Context, user: &User, guild: Option<Guild>) -> String {
        let user_name = user.name.clone();
        if let Some(guild_id) = guild {
            match guild_id.member(ctx, user).await {
                Ok(member) => member.nick.unwrap_or(user_name),
                Err(_) => "退出済みのユーザー".into(),
            }
        } else {
            user_name
        }
    }
}


pub mod discord_helper {
    use std::time::Duration;
    use serenity::prelude::*;
    use serenity::Result as SrnResult;
    use serenity::model::{channel::Message, id::{MessageId, ChannelId}};

    
    pub async fn await_right_reply<F, T>(ctx: &Context, msg: &Message, filter: F) -> Option<T> where
        F: Fn(&str) -> Result<T, String>,
    {
        while let Some(reply) = msg.channel_id.await_reply(ctx).author_id(msg.author.id)
            .timeout(Duration::from_secs(60*10)).await {
            
            if reply.content == "cancel" {
                msg.channel_id.say(ctx, "キャンセルしました\n--------ｷﾘﾄﾘ線--------").await.unwrap();
                return None;
            }

            match filter(&reply.content) {
                Ok(result) => return Some(result),
                Err(error_message) => {
                    msg.channel_id.send_message(ctx, |m| {
                        m.embed(|e| {
                            e.description(error_message.to_string() + "\n入力しなおしてください。終了したい場合は**cancel**と入力してください。").color(0xffaf60)
                        })
                    }).await.unwrap();
                }
            }
        }
        msg.channel_id.say(ctx, "10分間操作がなかったためキャンセルしました\n--------ｷﾘﾄﾘ線--------").await.unwrap();
        None
    }

    pub async fn purge(ctx: &Context, channel_id: ChannelId, after: MessageId) -> SrnResult<()> {
        let messages = channel_id.messages(ctx, |g| {
            g.after(after)
        }).await?;
        channel_id.delete_messages(&ctx, messages).await?;
        Ok(())
    }
}
pub use discord_helper::{await_right_reply};