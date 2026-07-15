use rocket::serde::{Deserialize, Serialize};

use crate::entities::carts;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Cart {
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub uploaded_at: String,
    pub downloads: i64,
    pub has_screenshot: bool,
    pub rom_size: i64,
}

impl From<carts::Model> for Cart {
    fn from(m: carts::Model) -> Self {
        Cart {
            tags: if m.tags.is_empty() {
                vec![]
            } else {
                m.tags.split(',').map(str::to_string).collect()
            },
            has_screenshot: m.has_screenshot,
            id: m.id,
            title: m.title,
            author: m.author,
            description: m.description,
            uploaded_at: m.uploaded_at,
            downloads: m.downloads,
            rom_size: m.rom_size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CartMeta {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CartList {
    pub carts: Vec<Cart>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct TokenCreate {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TokenCreated {
    pub id: String,
    pub name: String,
    /// Plaintext token, shown only in this response.
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}
