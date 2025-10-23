use std::{env, fs};
use chrono::{Utc, Duration};

use jsonwebtoken::{encode, EncodingKey, Header};
use poise::{serenity_prelude as serenity, CreateReply};
use reqwest::Client;
use serde_json::{json, Value};
use ::serenity::all::RoleId;

use crate::{database, types::{self, Category, CategoryWithJurors, Claims}, Context};

pub async fn is_host(ctx: &Context<'_>, user: &serenity::User) -> Result<bool, serenity::Error>  {
    let guild_id_int: u64 = env::var("GUILD_ID")
    .expect("Missing `GUILD_ID` env var, see README for more information.")
    .parse().expect("Error parsing guild id to int");
    let role_id_int: u64 = env::var("HOST_ID")
    .expect("Missing `HOST_ID` env var, see README for more information.")
    .parse().expect("Error parsing host id to int");


    let guild_id = serenity::GuildId::new(guild_id_int);
    let role_id = serenity::RoleId::new(role_id_int);


    let res = user.has_role(ctx.http(), guild_id, role_id).await?;

    if !res {
        ctx.send(CreateReply::default().content("You do not have the host role").ephemeral(true)).await?;
    }

    Ok(res)
}

pub async fn get_cats_with_users(ctx: &Context<'_>) -> Result<Vec<CategoryWithJurors>, serenity::Error> {
    let guild_id_int: u64 = env::var("GUILD_ID")
    .expect("Missing `GUILD_ID` env var, see README for more information.")
    .parse().expect("Error parsing guild id to int");
    let guild_id = serenity::GuildId::new(guild_id_int);

    let mut cats: Vec<Category> = Vec::new();
    match database::get_all_categories().await {
        Ok(c) => cats = c,
        Err(_) => return Ok(Vec::new()),
    }
    
    let mut cat_map: std::collections::HashMap<RoleId, Category> = cats
        .into_iter()
        .map(|c| (RoleId::new(c.role_id), c))
        .collect();

    let members = match guild_id.members(ctx.http(), None, None).await {
        Ok(m) => m,
        Err(e) => {
            let message = format!("Error fetching member: {}", e);
            ctx.send(CreateReply::default().content(message).ephemeral(true)).await?;
            return Err(e)
        }
    };

    let mut jurors_map: std::collections::HashMap<RoleId, Vec<String>> = std::collections::HashMap::new();

    for m in members {
        let display_name = m.display_name().to_string();

        for role in m.roles {
            if cat_map.contains_key(&role) {
                jurors_map.entry(role).or_default().push(display_name.clone());
            }
        }
    }

    let mut cats_with_jurors: Vec<CategoryWithJurors> = Vec::new();

    for (role_id, category) in cat_map {
        let jurors = jurors_map.remove(&role_id).unwrap_or_default();
        cats_with_jurors.push(CategoryWithJurors {
            category,
            jurors,
        });
    }

    Ok(cats_with_jurors)
}

pub async fn get_access_token(service_account_path: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>  {
    let data = fs::read_to_string(service_account_path)?;
    let json: Value = serde_json::from_str(&data)?;

    let email = json["client_email"].as_str().unwrap();
    let private_key = json["private_key"].as_str().unwrap();

    let now = Utc::now();
    let claims = Claims {
        iss: email,
        scope: "https://www.googleapis.com/auth/spreadsheets",
        aud: "https://oauth2.googleapis.com/token",
        iat: now.timestamp(),
        exp: (now + Duration::minutes(60)).timestamp(),
    };

    let jwt = encode(
        &Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(private_key.as_bytes())?,
    )?;

    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt),
    ];

    let client = Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(resp["access_token"].as_str().unwrap().to_string())
}

pub async fn write_to_sheet(
    access_token: &str,
    spreadsheet_id: &str,
    range: &str,
    cats_with_jurors: Vec<CategoryWithJurors>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut values: Vec<Vec<String>> = Vec::new();

    for cat in cats_with_jurors {
        values.push(vec![format!(
            "{}",
            cat.category.category_name,
        )]);

        if cat.jurors.is_empty() {
            values.push(vec!["(No jurors)".to_string()]);
        } else {
            for juror in cat.jurors {
                values.push(vec![juror]);
            }
        }

        // Add a blank line for spacing
        values.push(vec!["".to_string()]);
    }

    let body = json!({
        "range": range,
        "majorDimension": "ROWS",
        "values": values
    });

    let client = Client::new();
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
        spreadsheet_id,
        range
    );

    let _resp = client
        .put(&url)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    Ok(())
}