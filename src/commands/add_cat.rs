use poise::CreateReply;
use serenity::utils;

use crate::{database, types, utils::is_host, Context, Error};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn add_cat(
    ctx: Context<'_>,
    #[description = "The ID of the category role"]
    role_id_string: String,
    #[description = "Name of the category"]
    category_name: String,
    #[description = "Is the category open?"]
    is_open: bool,
    #[description = "What is the cap of the category"]
    category_limit: u64,
    #[description = "What is the type of the category"]
    category_type: types::CategoryType,
) -> Result<(), Error> {
    // Check if the user executing the command is a host
    if !is_host(&ctx, ctx.author()).await? {return Ok(())}

    let role_id: u64 = role_id_string
    .parse()
    .map_err(|_| poise::serenity_prelude::Error::Other("Invalid role ID".into()))?;

    let cat =  types::Category { role_id, category_name, is_open, category_limit, category_type };

    match database::add_cat(&cat).await {
        Ok(_) => {
            ctx.send(CreateReply::default().content("Succesfully created the category").ephemeral(true)).await?;
        },
        Err(e) => {
            ctx.send(CreateReply::default().content("Error creating the category").ephemeral(true)).await?;
            eprintln!("Error adding cat: {}", e);
        },
    }
    Ok(())
}