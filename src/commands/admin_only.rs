use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    macros::{group, command},
    Args,
    CommandResult,
};
use crate::utils::*;
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


#[group]
#[commands(execute_sql)]
#[required_permissions(ADMINISTRATOR)]
pub struct AdminOnly;