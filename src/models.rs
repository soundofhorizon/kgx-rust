use chrono::NaiveDateTime;
use crate::schema::demo_auction_info as info_table;

#[derive(Queryable, Debug)]
pub struct AuctionInfo {
    pub id: i32,
    pub last_tend: i32,
    pub item: String,
    pub end_time: NaiveDateTime,
    pub start_price: i32,
    pub bin_price: Option<i32>,
}

#[derive(Insertable, Debug)]
#[table_name = "info_table"]
pub struct NewAuctionInfo {
    pub last_tend: i32,
    pub item: String,
    pub end_time: NaiveDateTime,
    pub start_price: i32,
    pub bin_price: Option<i32>,
}

#[derive(Queryable, Debug)]
pub struct ChannelAuction {
    pub channel: i64,
    pub auction: Option<i32>,
}
