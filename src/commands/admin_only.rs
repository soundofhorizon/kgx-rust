use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    macros::{group, command},
    Args,
    CommandResult,
};
use crate::utils::*;
use crate::schema::{
    demo_auction_info::dsl::demo_auction_info as demo_auction_info_table,
    channel_auction::dsl::channel_auction as channel_auction_table,
};
use crate::models::*;
use diesel;
use diesel::prelude::*;

#[command]
#[aliases("es", "sql")]
async fn execute_sql(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let d = ctx.data.read().await;
    let conn = d.get::<ConnectionMapKey>().unwrap().lock().await;

    let result = diesel::sql_query(args.message()).execute(&*conn)?;
    msg.channel_id.say(&ctx.http, format!("結果: {}", result)).await?;

    Ok(())
}


#[command]
async fn select(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let d = ctx.data.read().await;
    let conn = d.get::<ConnectionMapKey>().unwrap().lock().await;

    let result = match &args.single::<String>()?[..] {
        "demo_auction_info" => {
            let result: Vec<AuctionInfo> = demo_auction_info_table.load(&*conn)?;
            result.iter().map(|row| format!("{:?}", row)).collect::<Vec<_>>().join("\n")
        },
        "channel_auction" => {
            let result: Vec<ChannelAuction> = channel_auction_table.load(&*conn)?;
            result.iter().map(|row| format!("{:?}", row)).collect::<Vec<_>>().join("\n")
        },
        _ => "設定されていないテーブルです".to_string(),
    };
    msg.channel_id.say(&ctx.http, result).await?;
    Ok(())
}


#[group]
#[commands(execute_sql, select)]
#[required_permissions(ADMINISTRATOR)]
pub struct AdminOnly;