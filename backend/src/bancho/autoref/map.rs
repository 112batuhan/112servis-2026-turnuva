//! A map: getting it ready, checking the mods (NoFail for everyone, and on an FM map one of
//! HD, HR, EZ or FL for each team) and starting it.

use time::Duration;

use crate::bancho::{
    names::same_name,
    room::commands::{map_command, mods_command},
    room::{split_mods, RoomSlot},
    tally::{PoolEntry, RosterPlayer},
};
use crate::db::autoref::LogLevel;

use super::{
    log, send, span, Action, AutoRef, CountdownKind, Phase, ReadyFields, Snapshot, BRACKET_READY,
    FM_MODS, MODS_CHECK, NF_GRACE, QUALIFIER_READY, START_COUNTDOWN,
};

impl AutoRef {
    pub(super) fn enter_ready(&mut self, s: &Snapshot, entry: &PoolEntry, out: &mut Vec<Action>) {
        let secs = if s.bracket {
            BRACKET_READY
        } else {
            QUALIFIER_READY
        };
        send(out, map_command(entry.beatmap_id, &entry.mode));
        send(out, mods_command(&entry.mods, entry.freemod, s.no_fail));
        log(out, LogLevel::Info, format!("{} açıldı.", entry.label));
        self.phase = Phase::Ready {
            map_id: entry.map_id,
            until: s.now + Duration::seconds(secs),
            nf_warned: false,
            fm_warned: false,
            check_at: None,
            forced: false,
            starting: None,
        };
        self.start_countdown(
            s,
            CountdownKind::Ready,
            secs,
            format!(
                "Sıradaki map: {}. Herkes hazır olduğunda ya da {} sonra map başlayacak.",
                entry.label,
                span(secs)
            ),
            out,
        );
    }

    pub(super) fn set_ready(&mut self, f: impl FnOnce(&mut ReadyFields)) {
        if let Phase::Ready {
            until,
            nf_warned,
            fm_warned,
            check_at,
            forced,
            starting,
            ..
        } = &mut self.phase
        {
            let mut fields = ReadyFields {
                until: *until,
                nf_warned: *nf_warned,
                fm_warned: *fm_warned,
                check_at: *check_at,
                forced: *forced,
                starting: *starting,
            };
            f(&mut fields);
            *until = fields.until;
            *nf_warned = fields.nf_warned;
            *fm_warned = fields.fm_warned;
            *check_at = fields.check_at;
            *forced = fields.forced;
            *starting = fields.starting;
        }
    }

    // Everyone is ready (or the timer ran out): FM and TB maps check everyone's mods
    // first (NoFail, and on FM a mod for each team).
    pub(super) fn try_start(&mut self, s: &Snapshot, forced: bool, out: &mut Vec<Action>) {
        let Phase::Ready {
            map_id,
            check_at,
            starting,
            ..
        } = self.phase.clone()
        else {
            return;
        };
        if check_at.is_some() || starting.is_some() {
            return;
        }
        // A bracket map waits for all four players unless its timer ran out.
        if !forced && s.bracket && !s.everyone_present() {
            return;
        }
        let entry = s.entry(map_id);
        let freemod = entry.is_some_and(|e| e.freemod);
        let fm = entry.is_some_and(|e| e.freemod && !e.tiebreaker);
        if freemod && (s.no_fail || fm) {
            send(out, "!mp settings");
            self.set_ready(|r| {
                r.check_at = Some(s.now + MODS_CHECK);
                r.forced = forced;
            });
            return;
        }
        self.start(s, out);
    }

    pub(super) fn check_mods(&mut self, s: &Snapshot, forced: bool, out: &mut Vec<Action>) {
        let Phase::Ready {
            map_id,
            nf_warned,
            fm_warned,
            ..
        } = self.phase.clone()
        else {
            return;
        };
        self.set_ready(|r| r.check_at = None);
        let fm = s.entry(map_id).is_some_and(|e| e.freemod && !e.tiebreaker);
        let missing_nf = if s.no_fail { missing_nf(s) } else { Vec::new() };
        let missing_fm = if fm { missing_fm(s) } else { Vec::new() };
        if missing_nf.is_empty() && missing_fm.is_empty() {
            return self.start(s, out);
        }
        // A team still without a mod once the time ran out, though told: the match stops
        // for a referee to settle it.
        if forced && fm_warned && !missing_fm.is_empty() {
            let names = missing_fm.join(", ");
            send(
                out,
                format!(
                    "{names}: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. \
                     Seçilmediği için maç durduruldu, hakemler çağrıldı."
                ),
            );
            out.push(Action::Handover(format!(
                "FM mapinde mod seçilmedi ({names}): takımdan en az bir oyuncu HD, HR, EZ ya da FL \
                 seçmeli."
            )));
            log(
                out,
                LogLevel::Warn,
                format!("FM modu seçilmedi, maç durduruldu: {names}"),
            );
            // Switched back to auto, the map is opened again with its full time.
            self.countdown = None;
            self.phase = Phase::Next;
            return;
        }
        // Only NoFail missing, though told: it starts without it.
        if forced && nf_warned && missing_fm.is_empty() {
            send(out, "Süre doldu, map NoFail'siz oyuncularla başlıyor.");
            log(
                out,
                LogLevel::Warn,
                format!("NF açmadan başladı: {}", missing_nf.join(", ")),
            );
            return self.start(s, out);
        }
        if !missing_nf.is_empty() {
            send(
                out,
                format!(
                    "{}: NoFail (NF) modunu seçmediniz, lütfen NF açın.",
                    missing_nf.join(", ")
                ),
            );
            log(
                out,
                LogLevel::Info,
                format!("NF uyarısı: {}", missing_nf.join(", ")),
            );
        }
        if !missing_fm.is_empty() {
            send(
                out,
                format!(
                    "{}: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. \
                     Seçmezseniz maç durdurulup hakem çağrılacak.",
                    missing_fm.join(", ")
                ),
            );
            log(
                out,
                LogLevel::Info,
                format!("FM mod uyarısı: {}", missing_fm.join(", ")),
            );
        }
        let (nf, fm) = (!missing_nf.is_empty(), !missing_fm.is_empty());
        self.set_ready(|r| {
            r.nf_warned |= nf;
            r.fm_warned |= fm;
        });
        if forced {
            self.start_countdown(
                s,
                CountdownKind::Ready,
                NF_GRACE,
                format!("Map {} sonra başlayacak.", span(NF_GRACE)),
                out,
            );
        }
    }

    fn start(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        send(out, format!("!mp start {START_COUNTDOWN}"));
        self.set_ready(|r| r.starting = Some(s.now));
    }
}

// The match's players in the room, as their slots read them.
fn player_slots<'a>(s: &'a Snapshot) -> impl Iterator<Item = (&'a RoomSlot, &'a RosterPlayer)> {
    s.room.slots.iter().filter_map(|slot| {
        let p = s
            .roster
            .iter()
            .find(|p| slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username))?;
        Some((slot, p))
    })
}

// The players in the room without NoFail, by name; those whose mods weren't read aren't
// counted.
fn missing_nf<'a>(s: &'a Snapshot) -> Vec<&'a str> {
    player_slots(s)
        .filter(|(slot, _)| slot.mods_known && !split_mods(&slot.mods).iter().any(|m| m == "NF"))
        .map(|(slot, _)| slot.username.as_str())
        .collect()
}

// The teams in the room none of whose players took HD, HR, EZ or FL, by name: on an FM
// map each team needs one. A team with a player whose mods weren't read isn't counted.
fn missing_fm<'a>(s: &'a Snapshot) -> Vec<&'a str> {
    s.teams
        .iter()
        .filter(|team| {
            let mut slots = player_slots(s)
                .filter(|(_, p)| p.team_id == team.id)
                .peekable();
            slots.peek().is_some()
                && slots.all(|(slot, _)| {
                    slot.mods_known
                        && !split_mods(&slot.mods)
                            .iter()
                            .any(|m| FM_MODS.contains(&m.as_str()))
                })
        })
        .map(|team| team.name.as_str())
        .collect()
}
