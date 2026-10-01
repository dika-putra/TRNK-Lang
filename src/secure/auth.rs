use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
    Guest,
}

impl Role {
    pub fn from_str(s: &str) -> Role {
        match s.to_ascii_lowercase().as_str() {
            "admin" => Role::Admin,
            "guest" => Role::Guest,
            _ => Role::User,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Role::Admin => "admin",
            Role::User => "user",
            Role::Guest => "guest",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

#[derive(Debug)]
pub struct JwtError(pub String);

impl fmt::Display for JwtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JwtError {}

fn signing_key() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "trunk-lang-dev-secret-change-me".to_string())
}

/// Sign a JWT for `uid`. Expires after `TRU_JWT_TTL_SECS` (default 24h).
pub fn create_jwt(uid: &str, role: &Role) -> Result<String, JwtError> {
    let ttl: i64 = std::env::var("TRU_JWT_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(86_400);

    let claims = Claims {
        sub: uid.to_string(),
        role: role.to_string(),
        exp: (chrono::Utc::now().timestamp() + ttl) as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(signing_key().as_bytes()),
    )
    .map_err(|e| JwtError(e.to_string()))
}

/// Verify signature + expiry and return the `sub` claim.
pub fn verify_jwt(token: &str) -> Result<String, JwtError> {
    let data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(signing_key().as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|e| JwtError(e.to_string()))?;
    Ok(data.claims.sub)
}
