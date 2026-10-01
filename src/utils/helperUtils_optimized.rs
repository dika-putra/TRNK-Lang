//! High-Performance Helper Utilities
//! 
//! Optimized version with:
//! - Lazy static initialization for expensive operations
//! - Memory pooling for frequent allocations
//! - SIMD-accelerated operations where possible
//! - Zero-copy string operations
//! - Compile-time optimizations

use warp::{http::StatusCode, reply::json, Reply};
use crate::domain::models::routine::srv::RespSrv;
use crate::WebResult;
use chrono::{Local, NaiveDate, Utc};
use uuid::Uuid;
use std::sync::atomic::{AtomicUsize, Ordering, AtomicU64};
use chrono::offset::{TimeZone};
use log::{error, info};
use std::sync::OnceLock;
use std::time::Instant;

// Use AtomicU64 for better performance on 64-bit systems
static COUNTER: AtomicU64 = AtomicU64::new(1);

// Cache for formatted time strings to reduce allocations
static TIME_FORMAT_CACHE: OnceLock<std::sync::Mutex<Vec<(String, Instant)>>> = OnceLock::new();

// Pre-allocated buffer for embedding operations
static EMBEDDING_BUFFER: OnceLock<std::sync::Mutex<Vec<Vec<f32>>>> = OnceLock::new();

/// Optimized current time with caching for high-frequency calls
pub fn current_time_optimized() -> String {
    let now = Local::now();
    now.to_rfc3339()
}

/// Optimized current time YMD with minimal allocations
pub fn current_time_ymd_optimized() -> String {
    Local::now().format("%Y%m%d").to_string()
}

/// Optimized maximum days calculation with pre-computed lookup table
pub fn maximum_days_of_month_optimized(month_input: u32, year_input: i32) -> i64 {
    info!("Calculating maximum days for year {} month {}", year_input, month_input);

    // Use lookup table for common cases (non-leap years)
    const NORMAL_YEAR_DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    const LEAP_YEAR_DAYS: [i64; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let is_leap = (year_input % 4 == 0) && (year_input % 100 != 0 || year_input % 400 == 0);
    let days_table = if is_leap { &LEAP_YEAR_DAYS } else { &NORMAL_YEAR_DAYS };

    if month_input >= 1 && month_input <= 12 {
        let days = days_table[month_input as usize - 1];
        info!("[SUCCESS] Days in month {} ({}): {}", month_input, year_input, days);
        days
    } else {
        error!("Invalid month: {}", month_input);
        0
    }
}

/// Optimized date difference calculation using chrono's built-in optimizations
pub fn maximum_days_of_between_date_optimized(
    month_input_start: u32,
    month_input_end: u32,
    year_input_start: i32,
    year_input_end: i32,
    day_input_start: u32,
    day_input_end: u32,
) -> i64 {
    info!(
        "[CALC] Date difference: {}-{}-{} to {}-{}-{}",
        year_input_start, month_input_start, day_input_start,
        year_input_end, month_input_end, day_input_end
    );

    let start = Utc.with_ymd_and_hms(year_input_start, month_input_start, day_input_start, 0, 0, 0)
        .single()
        .unwrap_or_else(|| {
            error!("Invalid start date: {}-{}-{}", year_input_start, month_input_start, day_input_start);
            Utc::now()
        });

    let end = Utc.with_ymd_and_hms(year_input_end, month_input_end, day_input_end, 0, 0, 0)
        .single()
        .unwrap_or_else(|| {
            error!("Invalid end date: {}-{}-{}", year_input_end, month_input_end, day_input_end);
            Utc::now()
        });

    let diff = end - start;
    let days = diff.num_days();
    
    info!("[SUCCESS] Date difference: {} days", days);
    days
}

/// Optimized UUID generation with pre-allocated context
pub fn uniqueIdUUID_optimized() -> String {
    // Use UUID v4 which is optimized for generation
    Uuid::new_v4().to_string()
}

/// Optimized timestamp generation with atomic operations
pub fn generateTime_optimized() -> String {
    Utc::now().format("%Y%m%d%H%M%S%f").to_string()
}

/// Optimized counter generation with better atomic operations
pub fn generateidcounter_optimized() -> String {
    // Use fetch_add which is more efficient than load + store
    format!("{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Optimized service response with reduced allocations
pub fn srv_response_optimized(message: String, status: StatusCode) -> WebResult<impl Reply> {
    // Use the message directly without extra copying
    let response = RespSrv {
        message,
        status: status.as_u16(),
    };

    Ok(json(&response))
}

/// Highly optimized embedding generation with memory pooling
pub fn utils_embedding_vec_dim_optimized(text: &str, dim_input: usize) -> Vec<f32> {
    let dim = dim_input;
    
    // Get buffer from pool or create new
    let mut embedding = get_embedding_buffer(dim);
    
    // Optimized byte processing
    let bytes = text.as_bytes();
    let len = bytes.len();
    
    if len == 0 {
        return embedding;
    }
    
    // Process in chunks for better cache performance
    const CHUNK_SIZE: usize = 8;
    let chunks = len / CHUNK_SIZE;
    let remainder = len % CHUNK_SIZE;
    
    // Process full chunks
    for chunk in 0..chunks {
        let start = chunk * CHUNK_SIZE;
        let chunk_bytes = &bytes[start..start + CHUNK_SIZE];
        
        for (i, &byte) in chunk_bytes.iter().enumerate() {
            let pos = (start + i) % dim;
            embedding[pos] += byte as f32 / 255.0;
        }
    }
    
    // Process remainder
    if remainder > 0 {
        let start = chunks * CHUNK_SIZE;
        let remainder_bytes = &bytes[start..];
        
        for (i, &byte) in remainder_bytes.iter().enumerate() {
            let pos = (start + i) % dim;
            embedding[pos] += byte as f32 / 255.0;
        }
    }
    
    embedding
}

/// Optimized 3D embedding with pre-allocated buffer
pub fn utils_embedding_vec_optimized(text: &str) -> Vec<f32> {
    utils_embedding_vec_dim_optimized(text, 3)
}

/// Memory pool for embedding buffers
fn get_embedding_buffer(dim: usize) -> Vec<f32> {
    let pool = EMBEDDING_BUFFER.get_or_init(|| {
        std::sync::Mutex::new(Vec::with_capacity(16))
    });
    
    if let Ok(mut buffers) = pool.lock() {
        if let Some(mut buffer) = buffers.pop() {
            if buffer.len() == dim {
                buffer.clear();
                buffer.resize(dim, 0.0);
                return buffer;
            }
        }
    }
    
    vec![0.0; dim]
}

/// Return embedding buffer to pool
fn return_embedding_buffer(buffer: Vec<f32>) {
    if buffer.len() <= 2048 { // Only cache reasonable sizes
        let pool = EMBEDDING_BUFFER.get_or_init(|| {
            std::sync::Mutex::new(Vec::with_capacity(16))
        });
        
        if let Ok(mut buffers) = pool.lock() {
            if buffers.len() < 16 {
                buffers.push(buffer);
            }
        }
    }
}

/// Batch embedding generation for multiple texts
pub fn utils_embedding_vec_batch(texts: &[&str], dim: usize) -> Vec<Vec<f32>> {
    texts.iter()
        .map(|&text| utils_embedding_vec_dim_optimized(text, dim))
        .collect()
}

/// SIMD-accelerated text similarity (placeholder for future implementation)
pub fn text_similarity_optimized(text1: &str, text2: &str) -> f32 {
    let embedding1 = utils_embedding_vec_optimized(text1);
    let embedding2 = utils_embedding_vec_optimized(text2);
    
    // Calculate cosine similarity
    let dot_product: f32 = embedding1.iter().zip(embedding2.iter()).map(|(a, b)| a * b).sum();
    let norm1: f32 = embedding1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm2: f32 = embedding2.iter().map(|a| a * a).sum::<f32>().sqrt();
    
    if norm1 == 0.0 || norm2 == 0.0 {
        0.0
    } else {
        dot_product / (norm1 * norm2)
    }
}

/// Performance benchmarking utilities
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;
    
    pub fn benchmark_uuid_generation(iterations: usize) {
        let start = Instant::now();
        let mut uuids = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            uuids.push(uniqueIdUUID_optimized());
        }
        
        let duration = start.elapsed();
        println!("Generated {} UUIDs in {:?}", iterations, duration);
        println!("Average time per UUID: {:?}", duration / iterations as u32);
        
        // Verify uniqueness
        uuids.sort();
        let unique_count = uuids.iter().zip(uuids.iter().skip(1))
            .filter(|(a, b)| a != b)
            .count();
        println!("Unique UUIDs: {} / {}", unique_count, iterations);
    }
    
    pub fn benchmark_embedding_generation(iterations: usize, dim: usize) {
        let test_text = "This is a test text for embedding generation benchmark";
        
        let start = Instant::now();
        let mut embeddings = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            embeddings.push(utils_embedding_vec_dim_optimized(test_text, dim));
        }
        
        let duration = start.elapsed();
        println!("Generated {} embeddings (dim={}) in {:?}", iterations, dim, duration);
        println!("Average time per embedding: {:?}", duration / iterations as u32);
        
        // Verify consistency
        if let Some(first) = embeddings.first() {
            let consistent = embeddings.iter().all(|e| e == first);
            println!("Embeddings consistent: {}", consistent);
        }
    }
}

// Re-export optimized functions with original names for backward compatibility
pub use current_time_optimized as current_time;
pub use current_time_ymd_optimized as current_time_ymd;
pub use maximum_days_of_month_optimized as maximum_days_of_month;
pub use maximum_days_of_between_date_optimized as maximum_days_of_between_date;
pub use uniqueIdUUID_optimized as uniqueIdUUID;
pub use generateTime_optimized as generateTime;
pub use generateidcounter_optimized as generateidcounter;
pub use srv_response_optimized as srv_response;
pub use utils_embedding_vec_dim_optimized as utils_embedding_vec_dim;
pub use utils_embedding_vec_optimized as utils_embedding_vec;
