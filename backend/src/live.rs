//! The hub every open page's live socket listens to (the socket itself is
//! handlers/live.rs), so a score saved on the referee page shows everywhere at once.
//!
//! Everyone gets `score` (a bracket match's score changed, with the new figures) and
//! `bracket` (something else about the bracket changed: reload it). Staff who can
//! open the bracket admin page also get who has each match's referee page open: a
//! tab tells the server which match's referee page it shows (`viewing`), and every
//! staff tab hears the new list for that match (`presence`), plus all of them when it
//! connects (`presence_all`). Nothing here is stored; it's what the open tabs say.
//!
//! Staff also hear `referee`: something only the referee page shows changed about a
//! match (its room options, the maps left out of the score, where its results are
//! read from), so the referee pages open on it reload it.
//!
//! The public match page also hears `match` (something about one match changed that
//! the bracket doesn't show, like its bans: reload that match) and, for the match it
//! says it shows (`watch`), every line said in its rooms as the room's bot saves it
//! (`chat`), the room itself as the bots read it whenever it changes (`room`, the same
//! view on every page) and who else is watching it (`spectators`: the signed-in ones
//! by name, each user once, and a count of the tabs nobody is signed in on). Staff
//! also hear `bancho` when a bot connects, drops or takes a room, for the bots page.
//!
//! A signed-in user's tabs hear `notifications` when their notifications change (one
//! arrives, is read or answered, or is replaced), so the bell reads its count again.
//!
//! The room manager (crate::bancho) and the statistics cache listen too, through
//! `changes`: whatever the pages are told changed about a match, they read again.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    bancho::view::LiveRoom,
    db::{autoref::LogEntry, chat::ChatMessage},
};

// How many events a slow tab may fall behind before it's told to reload instead.
const BUFFER: usize = 256;

#[derive(Debug, Clone, Serialize)]
pub struct Viewer {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg<'a> {
    Score {
        match_id: Uuid,
        team1_score: Option<i32>,
        team2_score: Option<i32>,
    },
    Bracket,
    Match {
        match_id: Uuid,
    },
    Referee {
        match_id: Uuid,
    },
    Chat {
        mp_id: i64,
        message: &'a ChatMessage,
    },
    Room {
        match_id: Uuid,
        room: &'a LiveRoom,
    },
    AutorefLog {
        match_id: Uuid,
        entry: &'a LogEntry,
    },
    Bancho,
    Notifications,
    Presence {
        match_id: Uuid,
        viewers: &'a [Viewer],
    },
    Spectators {
        match_id: Uuid,
        viewers: &'a [Viewer],
        guests: usize,
    },
    PresenceAll {
        matches: HashMap<Uuid, Vec<Viewer>>,
    },
}

// One event, serialized once for every tab.
#[derive(Debug)]
pub struct Event {
    json: String,
    to: Audience,
}

// Which tabs an event is for: every tab, unless one of these narrows it.
#[derive(Debug, Default)]
struct Audience {
    staff_only: bool,
    // Only the tabs watching this room's match.
    room: Option<i64>,
    // Only the tabs counted among this match's spectators.
    watchers_of: Option<Uuid>,
    // Only the tabs watching this match, spectating or not.
    watching: Option<Uuid>,
    // Only this user's tabs.
    user: Option<Uuid>,
}

// What a tab has asked for, which decides the events it's sent.
#[derive(Debug, Default)]
pub struct TabFilter {
    pub staff: bool,
    // Who is signed in on the tab.
    pub user: Option<Uuid>,
    // The rooms whose chat the tab is sent, and their match.
    pub rooms: HashSet<i64>,
    pub watched: Option<Uuid>,
    // The match whose public page the tab shows, counted among its spectators.
    pub spectating: Option<Uuid>,
}

impl Event {
    pub fn json(&self) -> &str {
        &self.json
    }

    // Whether a tab is sent this event.
    pub fn reaches(&self, tab: &TabFilter) -> bool {
        let to = &self.to;
        (!to.staff_only || tab.staff)
            && to.room.is_none_or(|room| tab.rooms.contains(&room))
            && to.watchers_of.is_none_or(|m| tab.spectating == Some(m))
            && to.watching.is_none_or(|m| tab.watched == Some(m))
            && to.user.is_none_or(|u| tab.user == Some(u))
    }
}

// What the room manager hears: a match changed (read it again), or something that may
// touch any of them did.
#[derive(Debug, Clone, Copy)]
pub enum Change {
    Match(Uuid),
    All,
}

// Who has a page of each match open (its referee page, or its public page): by match,
// each tab with its user (none for a tab nobody is signed in on), in the order they
// opened it.
#[derive(Default)]
struct Presence {
    tabs: HashMap<Uuid, Vec<(u64, Option<Viewer>)>>,
}

impl Presence {
    // The match's signed-in viewers, each user once, in the order they first opened it.
    fn viewers(&self, match_id: Uuid) -> Vec<Viewer> {
        let mut out: Vec<Viewer> = Vec::new();
        for viewer in self
            .tabs
            .get(&match_id)
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_ref())
        {
            if !out.iter().any(|v| v.id == viewer.id) {
                out.push(viewer.clone());
            }
        }
        out
    }

    // The match's tabs nobody is signed in on.
    fn guests(&self, match_id: Uuid) -> usize {
        self.tabs
            .get(&match_id)
            .map_or(0, |tabs| tabs.iter().filter(|(_, v)| v.is_none()).count())
    }

    fn all(&self) -> HashMap<Uuid, Vec<Viewer>> {
        self.tabs.keys().map(|&id| (id, self.viewers(id))).collect()
    }

    // Moves a tab to another match (or none). Returns the matches whose lists changed.
    fn set(&mut self, tab: u64, viewer: Option<&Viewer>, match_id: Option<Uuid>) -> Vec<Uuid> {
        let mut changed = Vec::new();
        self.tabs.retain(|&id, tabs| {
            let before = tabs.len();
            tabs.retain(|(t, _)| *t != tab);
            if tabs.len() != before && Some(id) != match_id {
                changed.push(id);
            }
            !tabs.is_empty()
        });
        if let Some(id) = match_id {
            self.tabs
                .entry(id)
                .or_default()
                .push((tab, viewer.cloned()));
            changed.push(id);
        }
        changed
    }
}

struct Inner {
    events: broadcast::Sender<Arc<Event>>,
    changes: broadcast::Sender<Change>,
    // Referee pages.
    presence: Mutex<Presence>,
    // Public match pages.
    spectators: Mutex<Presence>,
    next_tab: AtomicU64,
    // Users whose roles changed or who logged out: their open sockets check again what
    // they're allowed.
    users: broadcast::Sender<Uuid>,
}

// The hub every tab's socket listens to. Cheap to clone.
#[derive(Clone)]
pub struct Live(Arc<Inner>);

impl Default for Live {
    fn default() -> Self {
        Self(Arc::new(Inner {
            events: broadcast::channel(BUFFER).0,
            changes: broadcast::channel(BUFFER).0,
            presence: Mutex::default(),
            spectators: Mutex::default(),
            next_tab: AtomicU64::new(1),
            users: broadcast::channel(BUFFER).0,
        }))
    }
}

impl Live {
    // The user's roles changed, or they logged out.
    pub fn user_changed(&self, user_id: Uuid) {
        let _ = self.0.users.send(user_id);
    }

    pub fn user_changes(&self) -> broadcast::Receiver<Uuid> {
        self.0.users.subscribe()
    }

    fn emit(&self, msg: &ServerMsg, staff_only: bool) {
        self.send(
            msg,
            Audience {
                staff_only,
                ..Audience::default()
            },
        );
    }

    fn send(&self, msg: &ServerMsg, to: Audience) {
        let Ok(json) = serde_json::to_string(msg) else {
            return;
        };
        // An error only means no tab is listening.
        let _ = self.0.events.send(Arc::new(Event { json, to }));
    }

    fn changed(&self, change: Change) {
        let _ = self.0.changes.send(change);
    }

    // Every change the pages hear about, for the room manager.
    pub fn changes(&self) -> broadcast::Receiver<Change> {
        self.0.changes.subscribe()
    }

    // Every event, for a tab's socket (which sends on the ones that reach it).
    pub fn events(&self) -> broadcast::Receiver<Arc<Event>> {
        self.0.events.subscribe()
    }

    // A number that tells this tab apart in the presence lists.
    pub fn next_tab(&self) -> u64 {
        self.0.next_tab.fetch_add(1, Ordering::Relaxed)
    }

    // `presence_all`: who's on every referee page now, for a staff tab that just connected.
    pub fn presence_json(&self) -> Option<String> {
        let matches = self.0.presence.lock().unwrap().all();
        serde_json::to_string(&ServerMsg::PresenceAll { matches }).ok()
    }

    // `bracket`, for a tab that fell behind: a reload brings its page back in step.
    pub fn reload_json() -> Option<String> {
        serde_json::to_string(&ServerMsg::Bracket).ok()
    }

    // A bracket match's score was saved. Call after the change is committed.
    pub fn score(&self, match_id: Uuid, team1_score: Option<i32>, team2_score: Option<i32>) {
        self.changed(Change::Match(match_id));
        self.emit(
            &ServerMsg::Score {
                match_id,
                team1_score,
                team2_score,
            },
            false,
        );
    }

    // Anything else on the bracket changed (teams, times, rooms, staff, conclusions),
    // so pages reload it. Call after the change is committed.
    pub fn bracket_changed(&self) {
        self.changed(Change::All);
        self.emit(&ServerMsg::Bracket, false);
    }

    // Something about one match changed that the bracket doesn't show (bans, picks,
    // hand-awarded results, a lobby's teams), so its match page reloads it. Call after
    // the change is committed.
    pub fn match_changed(&self, match_id: Uuid) {
        self.changed(Change::Match(match_id));
        self.emit(&ServerMsg::Match { match_id }, false);
    }

    // Something only the referee page shows changed about a match (its room options,
    // "Sayma", where its results are read from); staff only. Call after the change is
    // committed.
    pub fn referee_changed(&self, match_id: Uuid) {
        self.changed(Change::Match(match_id));
        self.emit(&ServerMsg::Referee { match_id }, true);
    }

    // A line said in a multiplayer room, once it's saved; only tabs watching the match
    // the room belongs to get it.
    pub fn chat(&self, mp_id: i64, message: &ChatMessage) {
        self.send(
            &ServerMsg::Chat { mp_id, message },
            Audience {
                room: Some(mp_id),
                ..Audience::default()
            },
        );
    }

    // The match's room as the bots read it now; only tabs watching the match get it.
    pub fn room(&self, match_id: Uuid, room: &LiveRoom) {
        self.send(
            &ServerMsg::Room { match_id, room },
            Audience {
                watching: Some(match_id),
                ..Audience::default()
            },
        );
    }

    // A line of the match's auto referee log; only staff tabs watching the match get it.
    pub fn autoref_log(&self, match_id: Uuid, entry: &LogEntry) {
        self.send(
            &ServerMsg::AutorefLog { match_id, entry },
            Audience {
                staff_only: true,
                watching: Some(match_id),
                ..Audience::default()
            },
        );
    }

    // These users' notifications changed: each one's tabs read the bell's count again.
    // Call after the change is committed, with what the db::notifications writes return.
    pub fn notifications_changed(&self, users: impl IntoIterator<Item = Uuid>) {
        let users: HashSet<Uuid> = users.into_iter().collect();
        for user in users {
            self.send(
                &ServerMsg::Notifications,
                Audience {
                    user: Some(user),
                    ..Audience::default()
                },
            );
        }
    }

    // A bot connected, dropped, or took or left a room; staff only.
    pub fn bancho_changed(&self) {
        self.emit(&ServerMsg::Bancho, true);
    }

    // Moves a tab to another match's referee page (or none); staff hear the new lists.
    pub fn set_viewing(&self, tab: u64, viewer: &Viewer, match_id: Option<Uuid>) {
        let mut presence = self.0.presence.lock().unwrap();
        for id in presence.set(tab, Some(viewer), match_id) {
            let viewers = presence.viewers(id);
            self.emit(
                &ServerMsg::Presence {
                    match_id: id,
                    viewers: &viewers,
                },
                true,
            );
        }
    }

    // Moves a tab to another match's public page (or none); the tabs on each match whose
    // list changed hear the new one.
    pub fn set_spectating(&self, tab: u64, viewer: Option<&Viewer>, match_id: Option<Uuid>) {
        let mut spectators = self.0.spectators.lock().unwrap();
        for id in spectators.set(tab, viewer, match_id) {
            let viewers = spectators.viewers(id);
            self.send(
                &ServerMsg::Spectators {
                    match_id: id,
                    viewers: &viewers,
                    guests: spectators.guests(id),
                },
                Audience {
                    watchers_of: Some(id),
                    ..Audience::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer(n: u128) -> Viewer {
        Viewer {
            id: Uuid::from_u128(n),
            username: format!("user{n}"),
            avatar_url: String::new(),
        }
    }

    #[test]
    fn lists_each_user_once() {
        let mut p = Presence::default();
        let m = Uuid::from_u128(100);
        assert_eq!(p.set(1, Some(&viewer(1)), Some(m)), vec![m]);
        p.set(2, Some(&viewer(1)), Some(m));
        p.set(3, Some(&viewer(2)), Some(m));
        let names: Vec<_> = p.viewers(m).into_iter().map(|v| v.username).collect();
        assert_eq!(names, ["user1", "user2"]);
    }

    #[test]
    fn moving_and_leaving_update_both_matches() {
        let mut p = Presence::default();
        let (a, b) = (Uuid::from_u128(100), Uuid::from_u128(200));
        p.set(1, Some(&viewer(1)), Some(a));
        let mut changed = p.set(1, Some(&viewer(1)), Some(b));
        changed.sort();
        assert_eq!(changed, vec![a, b]);
        assert!(p.viewers(a).is_empty());
        assert_eq!(p.set(1, Some(&viewer(1)), None), vec![b]);
        assert!(p.all().is_empty());
    }

    #[test]
    fn counts_guest_tabs_apart_from_names() {
        let mut p = Presence::default();
        let m = Uuid::from_u128(100);
        p.set(1, None, Some(m));
        p.set(2, None, Some(m));
        p.set(3, Some(&viewer(1)), Some(m));
        assert_eq!(p.guests(m), 2);
        assert_eq!(p.viewers(m).len(), 1);
        p.set(1, None, None);
        assert_eq!(p.guests(m), 1);
    }
}
