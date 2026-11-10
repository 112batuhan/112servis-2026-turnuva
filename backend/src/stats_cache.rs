//! Keeps what the statistics page asks for (handlers/stats.rs), and the public bracket
//! (handlers/bracket.rs), between changes. Every open page reads them again after each
//! change, so the result is kept until the site next changes: one count per change,
//! however many are watching.

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
const STATS_TTL: Duration = Duration::from_secs(60);
// The bracket carries every team, whose names and banners change without the live hub
// hearing of it, so it's kept for less.
const BRACKET_TTL: Duration = Duration::from_secs(10);

// The statistics last counted, by stage (the stages' activity under the nil id), as the
// JSON sent. An entry stands until the live hub hears of any change (a saved score, a
// lobby's maps, accuracy from osu!, the bracket) or CACHE_TTL passes. A stage is counted
// by one request at a time: those arriving meanwhile wait for it and take its result.
#[derive(Clone)]
pub struct StatsCache(Arc<CacheInner>);

struct CacheInner {
    ttl: Duration,
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
    // For the statistics, by stage.
    pub fn stats() -> Self {
        Self::with_ttl(STATS_TTL)
    }

    // For the public bracket, under the nil id.
    pub fn bracket() -> Self {
        Self::with_ttl(BRACKET_TTL)
    }

    fn with_ttl(ttl: Duration) -> Self {
        Self(Arc::new(CacheInner {
            ttl,
            generation: AtomicU64::new(0),
            entries: Mutex::default(),
        }))
    }

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
            if e.generation == generation && e.at.elapsed() < self.0.ttl {
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
    let caches = [state.stats_cache.clone(), state.bracket_cache.clone()];
    let live = state.live.clone();
    crate::supervise::spawn("response cache invalidation", move || {
        let caches = caches.clone();
        let mut changes = live.changes();
        async move {
            let invalidate = || caches.iter().for_each(StatsCache::invalidate);
            // A change may have been missed while it wasn't listening.
            invalidate();
            loop {
                match changes.recv().await {
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => invalidate(),
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    });
}

// A cached JSON object with one more field added at its end, for what differs between
// requests (the bracket's `my_team_id`). `object` has to be a serialized struct or map.
pub fn with_field<T: serde::Serialize>(
    object: &Bytes,
    name: &str,
    value: &T,
) -> Result<Bytes, AppError> {
    let Some(head) = object.strip_suffix(b"}") else {
        return Err(AppError::Serialize(serde::ser::Error::custom(
            "a cached body isn't a JSON object",
        )));
    };
    let mut out = Vec::with_capacity(object.len() + name.len() + 48);
    out.extend_from_slice(head);
    if head != b"{" {
        out.push(b',');
    }
    serde_json::to_writer(&mut out, name)?;
    out.push(b':');
    serde_json::to_writer(&mut out, value)?;
    out.push(b'}');
    Ok(Bytes::from(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_a_field_to_an_object() {
        let body = Bytes::from_static(br#"{"matches":[],"teams":[{"id":1}]}"#);
        let out = with_field(&body, "my_team_id", &Some("a")).unwrap();
        assert_eq!(
            &out[..],
            br#"{"matches":[],"teams":[{"id":1}],"my_team_id":"a"}"#
        );
        let out = with_field(&Bytes::from_static(b"{}"), "x", &None::<u8>).unwrap();
        assert_eq!(&out[..], br#"{"x":null}"#);
        assert!(with_field(&Bytes::from_static(b"[]"), "x", &1).is_err());
    }
}
