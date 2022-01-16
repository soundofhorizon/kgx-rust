use chrono::NaiveDateTime;
use crate::schema::auction_info as info_table;
use crate::utils::formats;

#[derive(Queryable, Debug)]
pub struct AuctionInfo {
    pub id: i32,
    pub channel_id: i64,
    pub owner_id: i64,
    pub item: String,
    pub end_time: NaiveDateTime,
    pub start_price: i32,
    pub bin_price: Option<i32>,
    pub unit: String,
    pub embed_id: Option<i64>, // embed送信前のみNoneにしてよい
    pub notice: String,
    pub tenders_id: Vec<i64>,
    pub tends_price: Vec<i32>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "info_table"]
pub struct NewAuctionInfo {
    pub channel_id: i64,
    pub owner_id: i64,
    pub item: String,
    pub end_time: NaiveDateTime,
    pub start_price: i32,
    pub bin_price: Option<i32>,
    pub unit: String,
    pub notice: String,
}

use serenity::builder::CreateEmbed;
impl NewAuctionInfo {
    pub fn info_embed(&self, tender: String, end_time: String) -> impl Fn(&mut CreateEmbed) -> &mut CreateEmbed {
        let item = self.item.clone();
        let unit = self.unit.clone();
        let mut start_price = format!("{}{}", unit, formats::int_to_stack(self.start_price));
        if self.start_price >= 64 {
            start_price.push_str(&format!(" ({})", self.start_price));
        }
        let bin_price = if let Some(raw_bin_price) = self.bin_price {
            let mut bin_price = format!("{}{}", self.unit, formats::int_to_stack(raw_bin_price));
            if raw_bin_price >= 64 {
                bin_price.push_str(&format!(" ({})", raw_bin_price));
            }
            bin_price
        } else {
            "なし".into()
        };
        let notice = self.notice.clone();
        move |e| {
            e.field("出品者", &tender, true)
             .field("出品物", &item, true)
             .field("開始価格", &start_price, false)
             .field("即決価格", &bin_price, false)
             .field("終了日時", &end_time, true)
             .field("特記事項", &notice, true)
             .color(0xffaf60)
        }
    }
}

#[derive(Queryable, Debug)]
pub struct ChannelAuction {
    pub channel: i64,
    pub auction: Option<i32>,
}
