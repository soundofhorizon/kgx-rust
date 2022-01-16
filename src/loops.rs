use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::id::ChannelId;
use tokio;
use std::time::Duration;
use chrono::{Local, Timelike};
use crate::utils::*;
use crate::schema::channel_auction::dsl::{channel_auction, auction as auction_col};
use crate::models::*;
use diesel::prelude::*;

pub async fn start_check_minutely(ctx: Arc<Context>) {
    tokio::spawn(async move {
        loop {
            let now = Local::now().naive_local();
            
            let conn = ctx.get_connection().await;

            let result = channel_auction.get_results::<ChannelAuction>(&conn).unwrap();
            for ChannelAuction { channel, auction: auction_id } in result.into_iter() {
                let auction_id = match auction_id {
                    Some(auction_id) => auction_id,
                    None => continue,
                };
                let auction_info = AuctionManager::from_id(&conn, auction_id).unwrap().unwrap();
                
                if !(auction_info.end_time <= now) {
                    continue;
                }

                let content = if let [.., last_tend] = &auction_info.tend[..] {
                    format!("価格{}で落札されました", last_tend.price)
                } else {
                    "入札者はいませんでした".to_string()
                };
                let _ = ChannelId(channel as u64).send_message(
                    &ctx, |m| {
                        m.embed(|e| {
                            e.description(content)
                        })
                    }
                ).await;
                diesel::update(channel_auction.find(channel)).set(auction_col.eq(None::<i32>)).execute(&conn).unwrap();
            }

            // 00秒まで待機
            let now = Local::now().naive_local();
            tokio::time::sleep(Duration::from_secs(60-now.second() as u64)).await;
        }
    });
}