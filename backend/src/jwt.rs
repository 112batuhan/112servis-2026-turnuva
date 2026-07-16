use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// Tokens are valid for 7 days; there's no refresh flow in this starter.
pub const TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 7;

// Just enough to look the user up in postgres.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub iat: usize,
    pub exp: usize,
}

pub fn encode_token(user_id: Uuid, secret: &[u8]) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let claims = Claims {
        sub: user_id,
        iat: now as usize,
        exp: (now + TOKEN_TTL_SECS) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}

pub fn decode_token(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &Validation::default()).map(|d| d.claims)
}
