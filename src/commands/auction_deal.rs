use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    macros::{group, command},
    Args,
    ArgError,
    CommandResult,
};
use crate::schema::{
    demo_auction_info::dsl::{demo_auction_info, id as auction_id_col, last_tend as last_tend_col},
    channel_auction::dsl::{channel_auction, channel as channel_col, auction as auction_col},
};
use crate::utils::*;
use crate::models::*;
use diesel;
use diesel::prelude::*;
use chrono::{Local, Duration};


#[command]
async fn start(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let d = ctx.data.read().await;
    let conn = d.get::<ConnectionMapKey>().unwrap().lock().await;

    let channel_id = msg.channel_id.0 as i64;

    let result = channel_auction.filter(channel_col.eq(channel_id)).get_results::<ChannelAuction>(&*conn)?;
    if let [ChannelAuction { auction, .. }] = result[..] {
        if let Some(auction_id) = auction {
            msg.reply(&ctx.http, format!("既にオークションが開催されています (id:{})", auction_id)).await?;
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
    
    let new_auction: AuctionInfo = diesel::insert_into(demo_auction_info).values(&new_auction).get_result(&*conn)?;
    diesel::update(channel_auction.find(channel_id)).set(auction_col.eq(new_auction.id)).execute(&*conn)?;
    
    msg.channel_id.say(&ctx.http, format!("オークションを開始します\n{:?}", new_auction)).await?;
    
    Ok(())
}


#[command]
async fn tend(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult{
    let d = ctx.data.read().await;
    let conn = d.get::<ConnectionMapKey>().unwrap().lock().await;

    let channel_id = msg.channel_id.0 as i64;

    let result = channel_auction.filter(channel_col.eq(channel_id)).get_results::<ChannelAuction>(&*conn)?;
    let auction_id = if let [ChannelAuction { auction, .. }] = result[..] {
        match auction {
            Some(auction_id) => auction_id,
            None => {
                msg.reply(&ctx.http, format!("オークションが開催されていません")).await?;
                return Ok(());
            },
        }
    } else {
        msg.channel_id.say(&ctx.http, "このチャンネルはオークションチャンネルではありません").await?;
        return Ok(());
    };

    let price: i32 = args.single()?;
    let auction_info: AuctionInfo = demo_auction_info.filter(auction_id_col.eq(auction_id)).get_result(&*conn)?;

    if price <= auction_info.last_tend {
        let content = if auction_info.last_tend == auction_info.start_price-1 {
            // 入札なしのとき
            format!("入札価格が開始価格({})より低いです", auction_info.start_price)
        } else {
            format!("入札価格が現在の入札価格({})以下です", auction_info.last_tend)
        };
        msg.channel_id.say(&ctx.http, content).await?;
        return Ok(());
    }

    let author_name = &msg.author.name;
    if let Some(bin_price) = auction_info.bin_price {
        if price >= bin_price {
            msg.channel_id.send_message(
                &ctx, |m| {
                    m.embed(|e| {
                        e.description(format!("即決価格以上の入札がされました\n落札者: {}\n落札額: {}", author_name, price))
                    })
                }
            ).await?;
            diesel::update(channel_auction.find(channel_id)).set(auction_col.eq(None::<i32>)).execute(&*conn)?;
            return Ok(());
        }
    }

    msg.channel_id.say(&ctx.http, "入札しました").await?;
    diesel::update(demo_auction_info.find(auction_id)).set(last_tend_col.eq(price)).execute(&*conn)?;

    Ok(())
}


#[group]
#[commands(start, tend)]
struct AuctionDeal;