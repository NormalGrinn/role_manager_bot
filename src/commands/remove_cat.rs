use poise::CreateReply;

use crate::{database, utils::is_host, Context, Error};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn remove_cat(
    ctx: Context<'_>,
    #[description = "Name of the category"]
    category_name: String,
) -> Result<(), Error> {
    // Check if the user executing the command is a host
    if !is_host(&ctx, ctx.author()).await? {return Ok(())}

    match database::remove_cat(&category_name).await {
        Ok(_) => {
            ctx.send(CreateReply::default().content("Succesfully removed the category").ephemeral(true)).await?;
        },
        Err(e) => {
            ctx.send(CreateReply::default().content("Error removing the category").ephemeral(true)).await?;
            eprintln!("Error deleting cat: {}", e);
        },
    }
    Ok(())
}