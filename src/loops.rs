use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::id::{ChannelId, UserId};
use tokio;
use std::time::Duration;
use chrono::{Local, Timelike};
use crate::utils::*;
use crate::schema::channel_auction::dsl::channel_auction;
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
                let manager = AuctionManager::from_id(&conn, auction_id).unwrap().unwrap();
                
                if !(manager.end_time <= now) {
                    continue;
                }

                if let [.., last_tend] = &manager.tend[..] {
                    let owner_name = formats::display_name(&ctx, &UserId(manager.owner_id).to_user(&ctx.http).await.unwrap(), None).await;
                    let last_tender_name = formats::display_name(&ctx, &UserId(last_tend.tender_id).to_user(&ctx.http).await.unwrap(), None).await;
                    let _ = ChannelId(channel as u64).send_message(
                        &ctx, |m| {
                            m.embed(|e| {
                                e.description(format!("{}が出品した{}を{}が{}{}で落札しました！",
                                    owner_name, manager.item, last_tender_name, manager.unit, formats::stack_with_raw(last_tend.price)))
                            })
                        }
                    ).await;
                } else {
                    let _ = ChannelId(channel as u64).send_message(
                        &ctx, |m| {
                            m.embed(|e| {
                                e.description("入札者はいませんでした")
                            })
                        }
                    ).await;
                };
                ChannelId(channel as u64).say(&ctx, "--------ｷﾘﾄﾘ線--------").await.unwrap();
                manager.finish(&ctx).await;
            }

            // 00秒まで待機
            let now = Local::now().naive_local();
            tokio::time::sleep(Duration::from_secs(60-now.second() as u64)).await;
        }
    });
}