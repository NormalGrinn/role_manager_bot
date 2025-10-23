use poise::CreateReply;

use crate::{database, utils::is_host, Context, Error};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn overview(
    ctx: Context<'_>,
) -> Result<(), Error> {
    // Check if the user executing the command is a host
    if !is_host(&ctx, ctx.author()).await? {return Ok(())}

    match database::get_all_categories().await {
        Ok(cats) => {
            println!("{:?}", cats);
            ctx.send(CreateReply::default().content("Succesfully got the categories").ephemeral(true)).await?;
        },
        Err(e) => {
            ctx.send(CreateReply::default().content("Error getting the categories").ephemeral(true)).await?;
            eprintln!("Error fetching cats: {}", e);
        },
    }
    Ok(())
}