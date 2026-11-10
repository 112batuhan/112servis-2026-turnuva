//! Keeps what the statistics page asks for (handlers/stats.rs) between changes. Every
//! open statistics page reads the statistics again after each saved map, so the result
//! is kept until the site next changes: one count per change, however many are watching.

use std::{
    collections::HashMap,
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use axum::body::Bytes;
use tokio::{
    sync::broadcast,
    time::{Duration, Instant},
};
use uuid::Uuid;

use crate::{error::AppError, AppState};

// What's counted is kept this long at most, for the changes the live hub doesn't hear of
// (a pool edited, a team renamed).
const CACHE_TTL: Duration = Duration::from_secs(60);

// The statistics last counted, by stage (the stages' activity under the nil id), as the
// JSON sent. An entry stands until the live hub hears of any change (a saved score, a
// lobby's maps, accuracy from osu!, the bracket) or CACHE_TTL passes. A stage is counted
// by one request at a time: those arriving meanwhile wait for it and take its result.
#[derive(Clone, Default)]
pub struct StatsCache(Arc<CacheInner>);

#[derive(Default)]
struct CacheInner {
    // Bumped by every change.
    generation: AtomicU64,
    entries: Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<Option<CacheEntry>>>>>,
}

struct CacheEntry {
    generation: u64,
    at: Instant,
    body: Bytes,
}

impl StatsCache {
    pub async fn get_or_count<F, Fut>(&self, key: Uuid, count: F) -> Result<Bytes, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Bytes, AppError>>,
    {
        let slot = self
            .0
            .entries
            .lock()
            .expect("stats cache poisoned")
            .entry(key)
            .or_default()
            .clone();
        let mut entry = slot.lock().await;
        // Taken before reading, so a change made while counting makes the next request
        // count again.
        let generation = self.0.generation.load(Ordering::Acquire);
        if let Some(e) = entry.as_ref() {
            if e.generation == generation && e.at.elapsed() < CACHE_TTL {
                return Ok(e.body.clone());
            }
        }
        let body = count().await?;
        *entry = Some(CacheEntry {
            generation,
            at: Instant::now(),
            body: body.clone(),
        });
        Ok(body)
    }

    fn invalidate(&self) {
        self.0.generation.fetch_add(1, Ordering::AcqRel);
    }
}

// Drops what's counted whenever the live hub hears of a change.
pub fn spawn_invalidation(state: &AppState) {
    let cache = state.stats_cache.clone();
    let mut changes = state.live.changes();
    tokio::spawn(async move {
        loop {
            match changes.recv().await {
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => cache.invalidate(),
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}
