use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    macros::{group, command},
    Args,
    CommandResult,
};
use crate::schema::{
    auction_info::dsl::{auction_info, id as auction_id_col, embed_id},
    channel_auction::dsl::{channel_auction, channel as channel_col, auction as auction_col},
};
use crate::utils::*;
use crate::models::*;
use diesel;
use diesel::prelude::*;
use chrono::{Local, Duration, NaiveDate, Datelike, Timelike};

macro_rules! unwrap_or_return {
    ($result:expr) => {
        if let Some(content) = $result {
            content
        } else {
            return Ok(());
        }
    }
}

#[command]
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
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
    
    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description("何によるオークションですか？単位を入力してください。(ex.GTギフト券, がちゃりんご, エメラルド etc)").color(0xffaf60)
        })
    }).await?;
    let unit = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        if content.contains("\n") {
            Err("単位に改行を含めてはいけません".into())
        } else {
            Ok(content.to_string())
        }
    }).await);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description("出品するものを入力してください。").color(0xffaf60)
        })
    }).await?;
    let item = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        if content.contains("\n") {
            Err("出品物に改行を含めてはいけません".into())
        } else {
            Ok(content.to_string())
        }
    }).await);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description(
                "開始価格を入力してください。
                **※次のように入力してください。【〇LC+△ST+□】 or　【〇ST+△】 or 【△】 ex.1lc+1st+1 or 1st+1 or 32**"
            ).color(0xffaf60)
        })
    }).await?;
    let start_price = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        if let Some(price) = formats::stack_to_int(content) {
            if price == 0 {
                Err("開始価格を0にすることはできません".into())
            } else {
                Ok(price)
            }
        } else {
            Err("価格の形式が正しくありません\n**※次のように入力してください。【〇LC+△ST+□】 or 【〇ST+△】 or 【△】 ex.1lc+1st+1 or 1st+1 or 32**".into())
        }
    }).await);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description(
                "即決価格を入力してください。\n
                **※次のように入力してください。【〇LC+△ST+□】 or　【〇ST+△】 or 【△】 ex.1lc+1st+1 or 1st+1 or 32**\n
                ない場合は`なし`とお書きください。").color(0xffaf60)
        })
    }).await?;
    let bin_price = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        if content == "なし" {
            Ok(None)
        } else if let Some(price) = formats::stack_to_int(content) {
            if price == start_price {
                Err("即決価格が開始価格と等しいです。(価格が決まっているのであれば取引チャンネルをお使いください。)".into())
            } else if price < start_price {
                Err("即決価格が開始価格より低いです".into())
            } else {
                Ok(Some(price))
            }
        } else {
            Err("価格の形式が正しくありません\n**※次のように入力してください。【〇LC+△ST+□】 or　【〇ST+△】 or 【△】 ex.1lc+1st+1 or 1st+1 or 32**".into())
        }
    }).await);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description(format!("オークション終了日時を入力してください。\n**注意！**時間の書式に注意してください！\n\n
            例 {0}年5月14日の午後8時に終了したい場合：\n**{0}/05/14-20:00**と入力してください。\n\n
            例 1カ月2週間3日4時間5分後に終了したい場合:\n**1M2w3d4h5m**と入力してください。\n\n
            終了したい場合は**cancel**と入力してください", Local::now().naive_local().year())).color(0xffaf60)
        })
    }).await?;
    let (end_time, end_time_txt) = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        let now = Local::now().naive_local();

        let time = if let Some((year, month, day, hour, minute)) = formats::datetime(content) {
            if !(2000 <= year && year <= 3000) {
                return Err("年は2000~3000の範囲で指定してください".into());
            }
            if !(1 <= month && month <= 12) {
                return Err("存在しない月です".into());
            }
            if !(1 <= day && day <= formats::last_day(year, month)) {
                return Err("存在しない日です".into());
            }
            let date = NaiveDate::from_ymd(year, month, day);
            let datetime;
            if (hour, minute) == (24, 00) {
                datetime = date.and_hms(0, 0, 0) + Duration::days(1);
            } else if !(hour < 24 && minute < 60) {
                return Err("範囲外の時刻です".into());
            } else {
                datetime = date.and_hms(hour, minute, 0);
            }
            (datetime, year, month, day, hour, minute)

        } else if let Some(duration) = formats::duration(content) {
            let time = now.clone();
            let mut month = time.year()*12 + time.month() as i32 - 1;
            month += duration.0;
            let year = month / 12;
            let month = (month%12+1) as u32;
            let mut time = NaiveDate::from_ymd(year, month, time.day().min(formats::last_day(year, month))).and_time(time.time());
            time += duration.1;
            (time, time.year(), time.month(), time.day(), time.hour(), time.minute())

        } else {
            let year = now.year();
            return Err(format!("時間の書式が正しくありません\n\n
            例 {0}年5月14日の午後8時に終了したい場合：\n**{0}/05/14-20:00**と入力してください。\n\n
            例 1カ月2週間3日4時間5分後に終了したい場合:\n**1M2w3d4h5m**と入力してください。\n\n", year));
        };

        let duration = time.0 - now;
        if duration <= Duration::zero() {
            Err("終了時刻を現在時刻以前にすることはできません。".into())
        } else if duration <= Duration::hours(12) {
            Err("開催期間を12時間以下にすることはできません。".into())
        } else if duration >= Duration::weeks(8) {
            Err("2ヶ月以上にわたるオークションはできません。".into())
        } else {
            Ok((time.0, format!("{:0>4}/{:0>2}/{:0>2} {:0>2}:{:0>2}", time.1, time.2, time.3, time.4, time.5)))
        }
    }).await);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| {
            e.description("その他、即決特典などありましたらお書きください。\n長い場合、改行などをして**１回の送信**で書いてください。\n
            何も無ければ「なし」で構いません。").color(0xffaf60)
        })
    }).await?;
    let notice = unwrap_or_return!(discord_helper::await_right_reply(ctx, msg, |content| {
        Ok(content.into())
    }).await);

    
    let channel_id = msg.channel_id.0 as i64;
    let new_auction = NewAuctionInfo {
        channel_id, owner_id: msg.author.id.0 as i64, item, unit, start_price, bin_price, end_time, notice,
    };
    let embed_editter = new_auction.info_embed(formats::display_name(&ctx, &msg.author, msg.guild(&ctx).await).await, end_time_txt.clone());

    discord_helper::purge(&ctx, msg.channel_id, msg.id).await?;
    msg.channel_id.send_message(&ctx, |m| {
        m.embed(|e| {
            e.title("これで始めます。よろしいですか？YES/NOで答えてください。(小文字でもOK。NOの場合初めからやり直してください。)");
            embed_editter(e)
        })
    }).await?;
    if !unwrap_or_return!(discord_helper::await_right_reply(&ctx, msg, |content| {
        Ok(content.to_lowercase() == "yes")
    }).await) {
        msg.channel_id.say(&ctx, "初めからやり直してください。\n--------ｷﾘﾄﾘ線--------").await?;
        discord_helper::purge(&ctx, msg.channel_id, msg.id).await?;
        return Ok(());
    }

    discord_helper::purge(&ctx, msg.channel_id, msg.id).await?;
    let new_auction: AuctionInfo = diesel::insert_into(auction_info).values(&new_auction).get_result(&conn)?;
    let embed_message = msg.channel_id.send_message(&ctx, |m| {
        m.content("オークションを開始します")
         .embed(|e| {
            e.title("オークション内容").field("ID", new_auction.id, false);
            embed_editter(e)
        })
    }).await?;
    embed_message.pin(&ctx).await?;
    diesel::update(auction_info).filter(auction_id_col.eq(new_auction.id)).set(embed_id.eq(Some(embed_message.id.0 as i64))).execute(&conn)?;
    diesel::update(channel_auction.find(channel_id)).set(auction_col.eq(new_auction.id)).execute(&conn)?;
    
    
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
    let tender_name = formats::display_name(&ctx, &msg.author, msg.guild(&ctx).await).await;
    let format_price = format!("{}{}", manager.unit, formats::stack_with_raw(price));

    let tend_result = manager.tend(&conn, msg.author.id.0, price);
    match tend_result {
        Ok(finished) => {
            if finished {
                msg.channel_id.send_message(
                    ctx, |m| {
                        m.embed(|e| {
                            e.description(format!("即決価格以上の入札がされました\n落札者: **{}**\n落札額: **{}**", tender_name, format_price))
                             .color(0x4259fb)
                        })
                    }
                ).await?;
                msg.channel_id.say(&ctx, "--------ｷﾘﾄﾘ線--------").await?;
                manager.finish(&ctx).await;
            } else {
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.description(format!("入札者: **{}**,\n入札額: **{}**", tender_name, format_price))
                         .color(0x4259fb)
                    })
                }).await?;
            }
        },
        Err(error) => {
            let content = match error {
                TendError::LessThanStartPrice => format!("入札価格が開始価格({})より低いです", manager.start_price),
                TendError::LastTendOrLess => format!("入札価格が現在の入札価格({})以下です", manager.tend.last().unwrap().price),
                TendError::SameTender => "同一人物による入札は出来ません。".into(),
                TendError::ByOwner => "出品者が入札は出来ません。".into(),
            };
            msg.channel_id.say(&ctx.http, content).await?;
        }
    }
    
    Ok(())
}


#[group]
#[commands(start, tend)]
struct AuctionDeal;