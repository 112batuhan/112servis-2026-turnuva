//! A map: getting it ready, checking the mods and starting it.

use time::Duration;

use crate::bancho::{
    names::same_name,
    room::commands::{map_command, mods_command},
    room::split_mods,
    tally::PoolEntry,
};
use crate::db::autoref::LogLevel;

use super::{
    log, send, span, Action, AutoRef, CountdownKind, Phase, ReadyFields, Snapshot, BRACKET_READY,
    MODS_CHECK, NF_GRACE, QUALIFIER_READY, START_COUNTDOWN,
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
            check_at,
            forced,
            starting,
            ..
        } = &mut self.phase
        {
            let mut fields = ReadyFields {
                until: *until,
                nf_warned: *nf_warned,
                check_at: *check_at,
                forced: *forced,
                starting: *starting,
            };
            f(&mut fields);
            *until = fields.until;
            *nf_warned = fields.nf_warned;
            *check_at = fields.check_at;
            *forced = fields.forced;
            *starting = fields.starting;
        }
    }

    // Everyone is ready (or the timer ran out): FM and TB maps check everyone's NoFail
    // first.
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
        let freemod = s.entry(map_id).is_some_and(|e| e.freemod);
        if freemod && s.no_fail {
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
        let Phase::Ready { nf_warned, .. } = self.phase.clone() else {
            return;
        };
        self.set_ready(|r| r.check_at = None);
        let missing: Vec<&str> = s
            .room
            .slots
            .iter()
            .filter(|slot| {
                slot.mods_known
                    && !split_mods(&slot.mods).iter().any(|m| m == "NF")
                    && s.roster.iter().any(|p| {
                        slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username)
                    })
            })
            .map(|slot| slot.username.as_str())
            .collect();
        if missing.is_empty() {
            return self.start(s, out);
        }
        if forced && nf_warned {
            send(out, "Süre doldu, map NoFail'siz oyuncularla başlıyor.");
            log(
                out,
                LogLevel::Warn,
                format!("NF açmadan başladı: {}", missing.join(", ")),
            );
            return self.start(s, out);
        }
        send(
            out,
            format!(
                "{}: NoFail (NF) modunu seçmediniz, lütfen NF açın.",
                missing.join(", ")
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!("NF uyarısı: {}", missing.join(", ")),
        );
        self.set_ready(|r| r.nf_warned = true);
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
