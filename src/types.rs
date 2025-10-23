use serde::Serialize;

#[derive(Debug)]
pub struct Category {
    pub(crate) role_id: u64,
    pub(crate) category_name: String,
    pub(crate) is_open: bool,
    pub(crate) category_limit: u64,
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
