use std::{cmp::Ordering, str::FromStr};

use rusqlite::{types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef}, ToSql};
use serde::Serialize;
use poise::ChoiceParameter;

#[derive(Debug, Clone, ChoiceParameter)]
pub enum CategoryType {
    #[name = "Genre"]
    Genre,
    #[name = "Production"]
    Production,
    #[name = "Character"]
    Character,
    #[name = "Main"]
    Main,
}

#[derive(Debug)]
pub struct Category {
    pub(crate) role_id: u64,
    pub(crate) category_name: String,
    pub(crate) is_open: bool,
    pub(crate) category_limit: u64,
    pub(crate) category_type: CategoryType,
    pub(crate) category_host: u64,
}

#[derive(Debug)]
pub struct CategoryWithJurors {
    pub(crate) category: Category,
    pub(crate) jurors: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct Claims<'a> {
    pub(crate) iss: &'a str,
    pub(crate) scope: &'a str,
    pub(crate) aud: &'a str,
    pub(crate) iat: i64,
    pub(crate) exp: i64,
}

pub struct RgbColour {
    pub(crate) red: f64,
    pub(crate) green: f64,
    pub(crate) blue: f64,
}

impl FromStr for CategoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "genre" => Ok(Self::Genre),
            "production" => Ok(Self::Production),
            "character" => Ok(Self::Character),
            "main" => Ok(Self::Main),
            _ => Err(format!("Unknown category type: {}", s)),
        }
    }
}

impl FromSql for CategoryType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str() {
            Ok(s) => CategoryType::from_str(s)
                .map_err(|_| FromSqlError::Other(Box::new(std::fmt::Error))),
            Err(_) => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for CategoryType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = match self {
            CategoryType::Genre => "genre",
            CategoryType::Production => "production",
            CategoryType::Character => "character",
            CategoryType::Main => "main",
        };
        Ok(ToSqlOutput::from(s))
    }
}


impl Ord for CategoryType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for CategoryType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CategoryType {
    fn eq(&self, other: &Self) -> bool {
        self.rank() == other.rank()
    }
}

impl Eq for CategoryType {}

impl CategoryType {
    fn rank(&self) -> u8 {
        match self {
            CategoryType::Main => 0,
            CategoryType::Production => 1,
            CategoryType::Genre => 2,
            CategoryType::Character => 3,
        }
    }
}