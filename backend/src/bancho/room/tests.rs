use super::*;

fn now() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

fn read(lines: &[&str]) -> RoomState {
    read_lobby(lines.iter().map(|l| (*l, now())))
}

#[test]
fn reads_settings() {
    let room = read(&[
        "Room name: 112S: (A) vs (B), History: https://osu.ppy.sh/mp/123",
        "Beatmap: https://osu.ppy.sh/b/42 Artist - Title [Hard]",
        "Team mode: TeamVs, Win condition: ScoreV2",
        "Active mods: Freemod",
        "Players: 2",
        "Slot 1  Not Ready https://osu.ppy.sh/u/-5 Dev Player [Host / Team Blue / Hidden, HardRock]",
        "Slot 3  Ready     https://osu.ppy.sh/u/7 [Toy]           [Team Red]",
    ]);
    assert_eq!(room.name.as_deref(), Some("112S: (A) vs (B)"));
    assert_eq!(room.beatmap_id, Some(42));
    assert_eq!(room.team_mode.as_deref(), Some("TeamVs"));
    assert!(is_freemod(room.mods.as_deref()));
    assert_eq!(room.slots.len(), 2);
    let a = &room.slots[0];
    assert_eq!(
        (a.slot, a.user_id, a.username.as_str()),
        (1, Some(-5), "Dev Player")
    );
    assert_eq!(a.team, Some(SlotTeam::Blue));
    assert!(a.host);
    assert_eq!(a.mods, "HDHR");
    let b = &room.slots[1];
    assert_eq!(
        (b.username.as_str(), b.status, b.team),
        ("[Toy]", SlotStatus::Ready, Some(SlotTeam::Red))
    );
}

#[test]
fn follows_live_events() {
    let mut room = read(&[
        "A joined in slot 1 for team blue.",
        "B b joined in slot 2.",
        "B b moved to slot 4",
        "B b changed to Red",
        "Changed beatmap to https://osu.ppy.sh/b/9 Song [Insane]",
        "Enabled Hidden, NoFail, disabled FreeMod",
        "Added Some_Ref to the match referees",
    ]);
    assert_eq!(
        room.slots.iter().map(|s| s.slot).collect::<Vec<_>>(),
        [1, 4]
    );
    assert_eq!(room.slots[1].team, Some(SlotTeam::Red));
    assert_eq!(room.beatmap_id, Some(9));
    assert_eq!(room.mods.as_deref(), Some("Hidden, NoFail"));
    assert!(room.is_ref("some ref"));
    room.apply(MATCH_STARTED, now());
    assert!(room.playing);
    room.apply("A finished playing (Score: 1000, PASSED).", now());
    room.apply("B b finished playing (Score: 900, FAILED).", now());
    room.apply("The match has finished!", now());
    assert!(!room.playing);
    let result = &room.results[0];
    assert_eq!(result.beatmap_id, Some(9));
    assert_eq!(result.room_mods, "HDNF");
    assert!(!result.freemod);
    assert_eq!(result.scores[1].mods.as_deref(), Some("HDNF"));
    assert!(!result.scores[1].passed);
    room.apply("Kicked A from the match.", now());
    room.apply("B b left the game.", now());
    assert!(room.slots.is_empty());
    room.apply("Removed Some_Ref from the match referees", now());
    assert!(room.refs.is_empty());
}

#[test]
fn parses_replies() {
    assert_eq!(
        parse_created("Created the tournament match https://osu.ppy.sh/mp/111 112S: (A) vs (B)"),
        Some(111)
    );
    assert_eq!(
        parse_joined("x joined in slot 3 for team red."),
        Some(("x", 3, Some(SlotTeam::Red)))
    );
    assert_eq!(parse_left("Kicked y from the match."), Some("y"));
    assert_eq!(mod_acronyms("Hidden, DoubleTime"), "HDDT");
    assert_eq!(split_mods("hdhrNM"), ["HD", "HR"]);
    let mut room = RoomState::default();
    room.apply(
        "Beatmap changed to: Song [Hard] (https://osu.ppy.sh/b/5)",
        now(),
    );
    assert_eq!(room.beatmap_id, Some(5));
    room.apply("Changed match settings to 4 slots, TeamVs, ScoreV2", now());
    assert_eq!(room.size, Some(4));
}
