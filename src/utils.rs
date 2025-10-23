use std::{env, fs};
use chrono::{Utc, Duration};

use jsonwebtoken::{encode, EncodingKey, Header};
use poise::{serenity_prelude as serenity, CreateReply};
use reqwest::Client;
use serde_json::{json, Value};
use ::serenity::all::RoleId;

use crate::{database, types::{self, Category, CategoryType, CategoryWithJurors, Claims, RgbColour}, Context};

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
        exp: (now + Duration::minutes(2)).timestamp(),
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

fn build_vertical_column(cat: &CategoryWithJurors) -> (String, Vec<String>, usize) {
    let header = cat.category.category_name.clone();
    let limit = cat.category.category_limit; 
    let mut juror_rows: Vec<String> = Vec::new();

    if cat.jurors.is_empty() {
        juror_rows.push("(No jurors)".to_string());
    } else {
        for juror in &cat.jurors {
            juror_rows.push(juror.clone());
        }
    }
    
    juror_rows.push("".to_string());

    (header, juror_rows, limit.try_into().unwrap())
}

fn transcribe_block(cats: &[CategoryWithJurors]) -> (Vec<Vec<String>>, Vec<usize>) {
    if cats.is_empty() {
        return (Vec::new(), Vec::new());
    }
    
    let mut all_cols: Vec<(String, Vec<String>, usize)> = cats.iter()
        .map(|cat| build_vertical_column(cat))
        .collect();
    
    let block_limits: Vec<usize> = all_cols.iter().map(|(_, _, limit)| *limit).collect();
        
    let max_height = all_cols.iter()
        .map(|(_, jurors, limit)| {
            jurors.len().max(*limit + 1)
        })
        .max()
        .unwrap_or(1); 
        
    let total_rows_needed = 1 + max_height;
    let mut block: Vec<Vec<String>> = vec![vec![]; total_rows_needed]; 

    for (header, mut juror_rows, _) in all_cols {
        while juror_rows.len() < max_height {
            juror_rows.push("".to_string());
        }

        block[0].push(header);

        for i in 0..max_height {
            block[i + 1].push(juror_rows[i].clone());
        }
    }
    
    (block, block_limits)
}

fn darken_color(color: &RgbColour) -> RgbColour {
    RgbColour {
        red: (color.red * 0.85).max(0.0),
        green: (color.green * 0.85).max(0.0),
        blue: (color.blue * 0.85).max(0.0),
    }
}

fn create_update_range_request(
    start_row: usize,
    end_row: usize,
    start_col: usize,
    end_col: usize,
    sheet_id: i32,
    color: &RgbColour,
    bold: bool,
) -> serde_json::Value {
    let mut cell_format = json!({
        "userEnteredFormat": {
            "backgroundColor": {
                "red": color.red,
                "green": color.green,
                "blue": color.blue
            }
        }
    });

    if bold {
        cell_format["userEnteredFormat"]["textFormat"] = json!({ "bold": true });
    }

    json!({
        "repeatCell": {
            "range": {
                "sheetId": sheet_id,
                "startRowIndex": start_row,
                "endRowIndex": end_row,
                "startColumnIndex": start_col,
                "endColumnIndex": end_col
            },
            "cell": cell_format,
            "fields": "userEnteredFormat(backgroundColor,textFormat.bold)"
        }
    })
}


fn process_category_block(
    label: &str,
    cats: &[CategoryWithJurors],
    base_color: &RgbColour,
    sheet_id: i32,
    values: &mut Vec<Vec<String>>,
    formatting_requests: &mut Vec<serde_json::Value>,
    current_row_index: &mut usize,
) {
    if cats.is_empty() {
        return;
    }

    const ROW_OFFSET: usize = 1; // since we start writing at row 2
    const COL_OFFSET: usize = 1; // since we start at column B

    let overage_color = darken_color(base_color);
    let header_color = darken_color(base_color);

    // Add section label
    values.push(vec![format!("{}:", label)]);
    *current_row_index += 1;

    // Build block of category data
    let (block, block_limits) = transcribe_block(cats);
    let block_start_row = *current_row_index;
    let block_row_count = block.len();
    values.extend(block.clone());

    // === Header row formatting ===
    let header_end_col = block_limits.len();
    if header_end_col > 0 {
        let header_req = create_update_range_request(
            block_start_row + ROW_OFFSET,
            block_start_row + ROW_OFFSET + 1,
            0 + COL_OFFSET,
            header_end_col + COL_OFFSET,
            sheet_id,
            &header_color,
            true
        );
        formatting_requests.push(header_req);
    }

    // === Column formatting ===
    for (col_index, &limit) in block_limits.iter().enumerate() {
        let actual_jurors = cats.get(col_index).map(|c| c.jurors.len()).unwrap_or(0);

        // Within limit
        if limit > 0 {
            let limit_start_row = block_start_row + 1 + ROW_OFFSET;
            let limit_end_row = limit_start_row + limit;

            let limit_req = create_update_range_request(
                limit_start_row,
                limit_end_row,
                col_index + COL_OFFSET,
                col_index + 1 + COL_OFFSET,
                sheet_id,
                base_color,
                false
            );
            formatting_requests.push(limit_req);
        }

        // Overage
        if actual_jurors > limit {
            let overage_start_row = block_start_row + 1 + limit + ROW_OFFSET;
            let overage_end_row = block_start_row + 1 + actual_jurors + ROW_OFFSET;

            let overage_req = create_update_range_request(
                overage_start_row,
                overage_end_row,
                col_index + COL_OFFSET,
                col_index + 1 + COL_OFFSET,
                sheet_id,
                &overage_color,
                false
            );
            formatting_requests.push(overage_req);
        }
    }

    *current_row_index += block_row_count;

    // Separator row
    values.push(vec!["".to_string()]);
    *current_row_index += 1;
}

pub async fn write_to_sheet(
    access_token: &str,
    spreadsheet_id: &str,
    sheet_name: &str,
    mut cats_with_jurors: Vec<CategoryWithJurors>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const GENRE_COLOUR: RgbColour = RgbColour { red: 1.0, green: 1.0, blue: 0.8 };
    const PRODUCTION_COLOUR: RgbColour = RgbColour { red: 1.0, green: 0.8, blue: 0.8 }; 
    const CHAR_COLOUR: RgbColour = RgbColour { red: 1.0, green: 0.5, blue: 1.0 }; 
    const MAIN_COLOUR: RgbColour = RgbColour { red: 1.0, green: 0.0, blue: 1.0 }; 

    let client = Client::new();

    let sheet_id_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}",
        spreadsheet_id
    );
    let resp = client.get(&sheet_id_url).bearer_auth(access_token).send().await?;
    let resp_json: serde_json::Value = resp.json().await?;

    let sheet_id: i32 = resp_json["sheets"]
        .as_array()
        .and_then(|sheets| {
            sheets.iter().find(|sheet| sheet["properties"]["title"] == sheet_name)
        })
        .and_then(|sheet| sheet["properties"]["sheetId"].as_i64())
        .ok_or_else(|| format!("Sheet '{}' not found.", sheet_name))? as i32;

    cats_with_jurors.sort_by_key(|c| (
        c.category.category_type.clone(),
        !c.category.is_open,
        c.category.category_name.clone(),
    ));

    let mut genres: Vec<CategoryWithJurors> = Vec::new();
    let mut productions: Vec<CategoryWithJurors> = Vec::new();
    let mut characters: Vec<CategoryWithJurors> = Vec::new();
    let mut mains: Vec<CategoryWithJurors> = Vec::new();

    for cat in cats_with_jurors {
        match cat.category.category_type {
            CategoryType::Genre => genres.push(cat),
            CategoryType::Production => productions.push(cat),
            CategoryType::Character => characters.push(cat),
            CategoryType::Main => mains.push(cat),
        }
    }

    let mut values: Vec<Vec<String>> = Vec::new();
    let mut formatting_requests: Vec<serde_json::Value> = Vec::new();
    let mut current_row_index = 0;

    // GENRES
    process_category_block(
        "Genre Categories", &genres, &GENRE_COLOUR, sheet_id,
        &mut values, &mut formatting_requests, &mut current_row_index
    );

    // PRODUCTIONS
    process_category_block(
        "Production Categories", &productions, &PRODUCTION_COLOUR, sheet_id,
        &mut values, &mut formatting_requests, &mut current_row_index
    );

    // CHARACTERS
    process_category_block(
        "Character Categories", &characters, &CHAR_COLOUR, sheet_id,
        &mut values, &mut formatting_requests, &mut current_row_index
    );

    // MAINS
    process_category_block(
        "Main Categories", &mains, &MAIN_COLOUR, sheet_id,
        &mut values, &mut formatting_requests, &mut current_row_index
    );

    let max_width = values.iter().map(|row| row.len()).max().unwrap_or(0);
    for row in &mut values {
        while row.len() < max_width {
            row.push("".to_string());
        }
    }

    let values_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}!B2?valueInputOption=RAW",
        spreadsheet_id, sheet_name
    );

    let values_body = json!({
        "range": format!("{}!B2", sheet_name),
        "majorDimension": "ROWS",
        "values": values
    });

    let clear_request = json!({
        "updateCells": {
            "range": { "sheetId": sheet_id },
            "fields": "userEnteredFormat" // Clears formatting ONLY, leaving existing values alone
        }
    });

    let batch_url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
        spreadsheet_id
    );
    
    let clear_body = json!({ "requests": [clear_request] });
    
    let resp = client
        .post(&batch_url)
        .bearer_auth(access_token)
        .json(&clear_body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("Error clearing sheet formatting: {}", resp.text().await?).into());
    }

    let resp = client
        .put(&values_url)
        .bearer_auth(access_token)
        .json(&values_body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("Error updating sheet values: {}", resp.text().await?).into());
    }

    if formatting_requests.len() > 1 {
        let batch_url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
            spreadsheet_id
        );

        let batch_body = json!({ "requests": formatting_requests });
        
        let resp = client
            .post(&batch_url)
            .bearer_auth(access_token)
            .json(&batch_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Error updating sheet formatting: {}", resp.text().await?).into());
        }
    }

    Ok(())
}
