use rocket::serde::{Deserialize, Serialize};
use webauthn_rs::prelude::{PublicKeyCredential, RegisterPublicKeyCredential};

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
    pub plays: i64,
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
            plays: m.plays,
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
    pub total_plays: i64,
    pub follower_count: u64,
    pub following_count: u64,
    pub followed_by_me: bool,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginInput {
    /// Username or email.
    pub identifier: String,
    pub password: String,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
    pub email: Option<String>,
    pub email_verified: bool,
    pub password_set: bool,
}

/// `login` either completes immediately (`user` set) or, for MFA-enabled
/// accounts, hands back a short-lived `pending_token` for `/auth/login/mfa`.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginOutcome {
    pub mfa_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginMfaInput {
    pub pending_token: String,
    /// A live TOTP code or an unused backup code.
    pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MfaStatus {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MfaSetupInfo {
    pub secret: String,
    pub otpauth_url: String,
    pub qr_png_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MfaConfirmInput {
    pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MfaConfirmed {
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MfaDisableInput {
    pub current_password: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SetPasswordInput {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VerifyEmailInput {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ForgotPasswordInput {
    pub email: String,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ResetPasswordInput {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AuthConfigInfo {
    pub turnstile_site_key: Option<String>,
    pub providers: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_seen_at: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub current: bool,
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

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PlayInput {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PlayResult {
    pub counted: bool,
    pub plays: i64,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CollectionCreate {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub featured_rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CollectionPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub featured_rank: Option<Option<i32>>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CollectionCartInput {
    pub cart_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CollectionOrderInput {
    pub cart_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CollectionInfo {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub featured_rank: Option<i32>,
    pub owner: String,
    pub cart_count: u64,
    pub follower_count: u64,
    pub followed_by_me: bool,
    pub carts: Vec<Cart>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JamCreate {
    pub title: String,
    pub slug: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub rules: String,
    pub starts_at: String,
    pub submissions_close_at: String,
    pub ends_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JamPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub starts_at: Option<String>,
    pub submissions_close_at: Option<String>,
    pub ends_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JamEntryInput {
    pub cart_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct JamInfo {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub rules: String,
    pub starts_at: String,
    pub submissions_close_at: String,
    pub ends_at: String,
    pub status: String,
    pub entry_count: u64,
    pub creator_count: u64,
    pub carts: Vec<Cart>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FeedEvent {
    pub kind: String,
    pub actor: String,
    pub occurred_at: String,
    pub cart: Cart,
    pub version: Option<i32>,
    pub collection_slug: Option<String>,
    pub collection_title: Option<String>,
    pub jam_slug: Option<String>,
    pub jam_title: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FeedPage {
    pub events: Vec<FeedEvent>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MetricWindow {
    pub current: i64,
    pub previous: i64,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DailyMetric {
    pub date: String,
    pub plays: i64,
    pub unique_players: i64,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DashboardInfo {
    pub plays: MetricWindow,
    pub unique_players: MetricWindow,
    pub rating_avg: f64,
    pub followers: i64,
    pub new_followers: i64,
    pub daily: Vec<DailyMetric>,
    pub carts: Vec<Cart>,
}

// --- Passkeys (WebAuthn) ---

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct WebauthnStartResponse {
    pub token: String,
    /// The `CreationChallengeResponse`/`RequestChallengeResponse`, forwarded
    /// to the browser's `navigator.credentials` call as-is.
    pub options: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct WebauthnRegisterFinishInput {
    pub token: String,
    pub label: String,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct WebauthnLoginStartInput {
    pub identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct WebauthnLoginFinishInput {
    pub token: String,
    pub credential: PublicKeyCredential,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PasskeyInfo {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

// --- Audit log ---

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AuditEntry {
    pub event: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

// --- Account deletion ---

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct DeleteAccountInput {
    pub current_password: String,
    /// Required only when the account has MFA enabled.
    #[serde(default)]
    pub code: Option<String>,
}
