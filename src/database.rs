use rusqlite::{params, Connection, Result};

use crate::types;

// Path of the database, should be in a folder outside of src
const PATH: &str = "databases/categories.db";

pub async fn add_cat(cat: &types::Category) -> Result<()> {
    const ADD_CATEGORY: &str = "
    INSERT INTO categories (role_id, category_name, is_open, category_limit)
    VALUES (?1, ?2, ?3, ?4);
    ";
    let conn = Connection::open(PATH).map_err(|e| {
        eprintln!("Failed to open database: {}", e);
        e
    })?;
    conn.execute(ADD_CATEGORY, params![cat.role_id, cat.category_name, cat.is_open, cat.category_limit]).map_err(|e| {
        eprintln!("Problem adding category to database: {}", e);
        e
    })?;
    Ok(())
}

pub async fn remove_cat(cat_name: &String) -> Result<()> {
    const REMOVE_CAT: &str = "
    DELETE FROM categories
    WHERE category_name = ?1;
    ";
    let conn = Connection::open(PATH).map_err(|e| {
        eprintln!("Failed to open database: {}", e);
        e
    })?;
    conn.execute(REMOVE_CAT, params![cat_name]).map_err(|e| {
        eprintln!("Problem adding category to database: {}", e);
        e
    })?;
    Ok(())
}

pub async fn get_all_categories() -> Result<Vec<types::Category>> {
    const GET_CATS: &str = "
    SELECT * FROM CATEGORIES;    
    ";
    let conn = Connection::open(PATH).map_err(|e| {
        eprintln!("Failed to open database: {}", e);
        e
    })?;
    let mut query = conn.prepare(GET_CATS)?;
    let cat_iter = query.query_map((), |row|
    Ok(types::Category {
        role_id: row.get(0)?,
        category_name: row.get(1)?,
        is_open: row.get(2)?,
        category_limit: row.get(3)?,
    })
    )?;
    let mut cats: Vec<types::Category> = Vec::new();
    for c in cat_iter {
        match c {
            Ok(cat) => cats.push(cat),
            Err(_) => {continue;},
        }
    }
    Ok(cats)
}