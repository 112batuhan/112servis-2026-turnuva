use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::role::Role;

// Tokens are valid for 7 days; there's no refresh flow in this starter.
pub const TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 7;

// Carries the user id plus the role and token version, so authorization checks
// don't need a database lookup. `tv` (token_version) is compared against the
// stored value on privileged routes to reject tokens issued before a role change.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: Role,
    pub tv: i32,
    // Whether the user has a linked Discord account. Defaults to false so older
    // tokens without the field decode as unverified (fail closed).
    #[serde(default)]
    pub discord_verified: bool,
    pub iat: usize,
    pub exp: usize,
}

pub fn encode_token(
    user_id: Uuid,
    role: Role,
    token_version: i32,
    discord_verified: bool,
    secret: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = Claims {
        sub: user_id,
        role,
        tv: token_version,
        discord_verified,
        iat: now as usize,
        exp: (now + TOKEN_TTL_SECS) as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn decode_token(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|d| d.claims)
}
