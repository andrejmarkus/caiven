use rocket::serde::{Deserialize, Serialize};

use crate::entities::{cart_versions, carts};

/// A cart plus its latest version's file info, denormalized for list/detail
/// views without an extra round trip.
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
    pub owner: Option<String>,
    pub rating_avg: f64,
    pub rating_count: i64,
    pub latest_version: i32,
    pub cart_size: i64,
    pub has_screenshot: bool,
}

impl Cart {
    pub fn from_model(
        m: carts::Model,
        owner: Option<String>,
        latest: Option<&cart_versions::Model>,
    ) -> Self {
        let rating_avg = if m.rating_count > 0 {
            m.rating_sum as f64 / m.rating_count as f64
        } else {
            0.0
        };
        Cart {
            tags: if m.tags.is_empty() {
                vec![]
            } else {
                m.tags.split(',').map(str::to_string).collect()
            },
            id: m.id,
            title: m.title,
            author: m.author,
            description: m.description,
            uploaded_at: m.uploaded_at,
            downloads: m.downloads,
            owner,
            rating_avg,
            rating_count: m.rating_count,
            latest_version: latest.map(|v| v.version).unwrap_or(0),
            cart_size: latest.map(|v| v.cart_size).unwrap_or(0),
            has_screenshot: latest.map(|v| v.has_screenshot).unwrap_or(false),
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

#[derive(Debug, Default, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionMeta {
    #[serde(default)]
    pub changelog: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CartPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CartVersionInfo {
    pub version: i32,
    pub cart_size: i64,
    pub changelog: String,
    pub has_screenshot: bool,
    pub created_at: String,
}

impl From<cart_versions::Model> for CartVersionInfo {
    fn from(v: cart_versions::Model) -> Self {
        CartVersionInfo {
            version: v.version,
            cart_size: v.cart_size,
            changelog: v.changelog,
            has_screenshot: v.has_screenshot,
            created_at: v.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CartDetail {
    #[serde(flatten)]
    pub cart: Cart,
    pub versions: Vec<CartVersionInfo>,
    pub own_rating: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CartList {
    pub carts: Vec<Cart>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserProfile {
    pub username: String,
    pub is_admin: bool,
    pub created_at: String,
    pub carts: Vec<Cart>,
    pub total: u64,
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

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RatingInput {
    pub score: i32,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CommentInput {
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CommentInfo {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}
