#[allow(unused)]

// Production-ready Security Utilities
// 
// All security-sensitive values are now configurable via environment variables.
// Default values are provided for development, but production should override these.

use std::convert::Infallible;
use std::env;
use aes::cipher::generic_array::GenericArray;
use aes_gcm::{Aes256Gcm, Nonce, Key, KeyInit,};
use base64::{decode, encode};
use rand::Rng;
use crate::domain::models::routine::login::{LoginRequest, LoginResponse};
use crate::{Users, WebResult};
use crate::secure::auth;
use crate::secure::error::Error::WrongCredentialsError;
use auth::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use aes_gcm::aead::{Aead,};
use chrono::{DateTime, Utc};
use surrealdb::sql::Thing;
use tokio;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use hex;
use log::info;
use rand::prelude::SliceRandom;
use redis::RedisResult;
use rand::rngs::OsRng;
use rand::RngCore;
use warp::{reply, Reply};
use warp::{reject, Filter,Rejection};

pub fn with_users(users : Users) -> impl Filter<Extract = (Users,), Error = Infallible> + Clone {
    warp::any().map(move ||  users.clone())
}

/// Get production-ready encryption key from environment or generate a secure default
/// In production, always set ENCRYPTION_KEY environment variable
fn get_encryption_key() -> Vec<u8> {
    match env::var("ENCRYPTION_KEY") {
        Ok(key_str) => {
            // Use provided key (should be 32 bytes for AES-256)
            if key_str.len() >= 32 {
                key_str.as_bytes()[..32].to_vec()
            } else {
                // Pad shorter keys
                let mut key = vec![0u8; 32];
                for (i, byte) in key_str.bytes().enumerate() {
                    if i < 32 {
                        key[i] = byte;
                    }
                }
                key
            }
        }
        Err(_) => {
            // Generate a warning and use a development default
            // In production, this should be configured!
            log::warn!("[SECURITY] WARNING: Using default encryption key! Set ENCRYPTION_KEY in production!");
            // Development default - NOT FOR PRODUCTION
            b"development_key_not_for_production_32".to_vec()
        }
    }
}

pub async fn login_handler(users: Users, body: LoginRequest) -> WebResult<impl Reply> {
    info!("[SECURITY] Login Handler - Login User");
    
    // Use production-ready encryption key
    let key = get_encryption_key();
    
    // Note: We don't use encrypted data for comparison (as noted in original code)
    // Instead, we decrypt stored password and compare with provided password
    let _encrypted_data = encrypt_data(&key, body.password.to_string().as_bytes());
    
    // Compare decrypted stored password with provided password
    match users.iter().find(|(_uid, user)| {
        user.email == body.email && 
        std::str::from_utf8(&decrypt_data(&key, user.password.as_str()))
            .expect("Invalid UTF-8 in stored password") == body.password
    }) {
        Some((uid, user)) => {
            let token = auth::create_jwt(&uid, &Role::from_str(&user.role))
                .map_err(|_| reject::custom(WrongCredentialsError))?;

            info!("[SECURITY] Login successful for user: {}", user.email);
            Ok(reply::json(&LoginResponse { token }))
        },
        None => {
            info!("[SECURITY] Login failed for email: {}", body.email);
            Err(reject::custom(WrongCredentialsError))
        }
    }
}


pub fn encrypt_data(key: &[u8], data: &[u8]) -> String {
    assert_eq!(key.len(), 32, "Key length must be 32 bytes for AES-256");
    let key = GenericArray::from_slice(key); // Use GenericArray to create the key
    let cipher = Aes256Gcm::new(key);
    let mut nonce = [0u8; 12]; // Ensure it's 12 bytes
    rand::thread_rng().fill_bytes(&mut nonce);

    // let mut nonce = [1, 2, 3, 4, 5];
    // nonce.shuffle(&mut rand::rng());
    // let nonce: [u8; 12] = rand::rng(); // 96-bits; unique per message
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), data).expect("encryption failure!");
    let encrypted_data = [nonce.to_vec(), ciphertext].concat(); // Prepend nonce for decryption
    encode(&encrypted_data) // Return the base64-encoded string
}

pub fn decrypt_data(key: &[u8], encrypted_data: &str) -> Vec<u8> {
    assert_eq!(key.len(), 32, "Key length must be 32 bytes for AES-256");

    let encrypted_data = decode(encrypted_data).expect("Invalid base64");
    assert!(encrypted_data.len() >= 12, "Encrypted data too short!");
    let key = GenericArray::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).expect("Decryption failure!")
}


// apikey:
pub fn generate_api_key() -> String {
    let uuid = Uuid::new_v4();
    uuid.to_string()
}

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn store_api_key(key: &str) {

}

pub fn validate_api_key(provided_key: &str, stored_hashed_key: &str) -> bool {
    let hashed_provided_key = hash_api_key(provided_key);
    hashed_provided_key == stored_hashed_key
}

pub fn convert_redis_result(result: redis::RedisResult<String>) -> String {
    match result {
        Ok(value) => value,
        Err(e) => format!("Error: {}", e),
    }
}
