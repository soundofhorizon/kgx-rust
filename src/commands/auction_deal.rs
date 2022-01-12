use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    macros::{group, command},
    Args,
    ArgError,
    CommandResult,
};
use crate::schema::{
    demo_auction_info::dsl::demo_auction_info,
    channel_auction::dsl::{channel_auction, channel as channel_col, auction as auction_col},
};
use crate::utils::*;
use crate::models::*;
use diesel;
use diesel::prelude::*;
use chrono::{Local, Duration};


#[command]
async fn start(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let conn = ctx.get_connection().await;

    let channel_id = msg.channel_id.0 as i64;

    let result = channel_auction.filter(channel_col.eq(channel_id)).get_results::<ChannelAuction>(&conn)?;
    if let [ChannelAuction { auction, .. }] = result[..] {
        if let Some(auction_id) = auction {
            msg.channel_id.say(&ctx.http, format!("既にオークションが開催されています (id:{})", auction_id)).await?;
            return Ok(());
        }
    } else {
        msg.channel_id.say(&ctx.http, "このチャンネルはオークションチャンネルではありません").await?;
        return Ok(());
    }
    
    let item: String = args.single()?; // 出品物
    let start_price: i32 = args.single()?; // 開始価格
    let bin_price: Option<i32> = match args.single() { // 即決価格 デフォルトでNone(即決なし)
        Err(ArgError::Eos) => None,
        other => Some(other?),
    };
    let minutes: i64 = match args.single() { // 終了時刻(何分後か) デフォルトで10
        Err(ArgError::Eos) => 10,
        other => other?,
    };

    if start_price <= 0 {
        msg.channel_id.say(&ctx.http, "開始価格が0以下です").await?;
        return Ok(());
    }
    if let Some(bin_price) = bin_price {
        if bin_price < start_price {
            msg.channel_id.say(&ctx.http, "即決価格が開始価格未満です").await?;
            return Ok(())
        }
    }
    
    let end_time = Local::now().naive_local() + Duration::minutes(minutes);
    
    let new_auction = NewAuctionInfo {
        item, start_price, bin_price, end_time, last_tend: start_price-1
    };
    
    let new_auction: AuctionInfo = diesel::insert_into(demo_auction_info).values(&new_auction).get_result(&conn)?;
    diesel::update(channel_auction.find(channel_id)).set(auction_col.eq(new_auction.id)).execute(&conn)?;
    
    msg.channel_id.say(&ctx.http, format!("オークションを開始します\n{:?}", new_auction)).await?;
    
    Ok(())
}


#[command]
async fn tend(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult{
    let conn = ctx.get_connection().await;

    let manager = AuctionManager::from_channel(&conn, msg.channel_id)?;

    let mut manager = match manager {
        Ok(manager) => manager,
        Err(error) => {
            match error {
                GetAuctionError::NotAuctionChannel => {
                    msg.channel_id.say(&ctx.http, "このチャンネルはオークションチャンネルではありません").await?;
                },
                GetAuctionError::NotHeld => {
                    msg.channel_id.say(&ctx.http, format!("オークションが開催されていません")).await?;
                },
                _ => unreachable!(),
            }
            return Ok(());
        }
    };

    let price: i32 = args.single()?;

    let tend_result = manager.tend(&conn, msg.author.id.0, price);
    match tend_result {
        Ok(finished) => {
            if finished {
                msg.channel_id.send_message(
                    &ctx, |m| {
                        m.embed(|e| {
                            e.description(format!("即決価格以上の入札がされました\n落札者: {}\n落札額: {}", msg.author.name, price))
                        })
                    }
                ).await?;
            } else {
                msg.channel_id.say(&ctx.http, "入札しました").await?;
            }
        },
        Err(error) => {
            let content = match error {
                TendError::LessThanStartPrice => format!("入札価格が開始価格({})より低いです", manager.start_price),
                TendError::LastTendOrLess => format!("入札価格が現在の入札価格({})以下です", manager.tend.last().unwrap().price),
                TendError::SameTender => "同一人物による入札は出来ません。".into(),
            };
            msg.channel_id.say(&ctx.http, content).await?;
        }
    }
    
    Ok(())
}


#[group]
#[commands(start, tend)]
struct AuctionDeal;