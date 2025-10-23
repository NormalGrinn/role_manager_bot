use std::{env, fs};

use google_sheets4::{yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey}, Sheets};
use poise::CreateReply;

use crate::{database, types, utils::{self, is_host, write_to_sheet}, Context, Error};

#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn update(
    ctx: Context<'_>,
) -> Result<(), Error> {
    // Check if the user executing the command is a host
    if !is_host(&ctx, ctx.author()).await? {return Ok(())}

    let defer_reply = CreateReply::default().content("...Fetching data and updating the Google Sheet. This may take a moment...").ephemeral(true);
    ctx.send(defer_reply).await?;

    let jurors: Vec<types::CategoryWithJurors>;
    match utils::get_cats_with_users(&ctx).await {
        Ok(j) => {
            println!("{:?}", j);
            jurors = j;
        },
        Err(e) => {
            ctx.send(CreateReply::default().content("Error fetching role data").ephemeral(true)).await?;
            eprintln!("Error fetching role data: {}", e);
            return Ok(());
        },
    }

    let access_token;
    match utils::get_access_token("./key.json").await {
        Ok(a) => access_token = a,
        Err(e) => {
            ctx.send(CreateReply::default().content("Error fetching access token").ephemeral(true)).await?;
            eprintln!("Error fetching access token: {}", e);
            return Ok(());
        },
    }

    let spreadsheet_id = "1_T8UWoz_78ExJziMEsXF-7QC7q0URxyWVjaG1AxHXHM";
    let sheet_name = env::var("SHEET_NAME")
    .expect("Missing `SHEET_NAME` env var, see README for more information.");

    match write_to_sheet(&access_token, spreadsheet_id, &sheet_name, jurors).await {
        Ok(_) => {
            ctx.send(CreateReply::default().content("Successfully updated sheet").ephemeral(true)).await?;
        },
        Err(e) => {
            ctx.send(CreateReply::default().content("Error updating sheet").ephemeral(true)).await?;
            eprintln!("Error updating sheet: {}", e);
        },
    }

    Ok(())
}

//1430664638378410024