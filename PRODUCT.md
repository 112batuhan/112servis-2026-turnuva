# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

An open 2v2 osu!standard tournament for the Turkish osu! community, run by the 112servis group. Dozens of teams are expected, across a wide range of ranks.

- **Players** sign in with osu!, link Discord and form a two-player team whose combined rank value stays under the limit. They sign up for a qualifier lobby, propose and accept new times for their bracket matches, and check the map pools. Users with no staff role, or only Streamer, can play.
- **Spectators** follow the bracket, the schedule, results, teams and the published map pools. They can subscribe to a match's Discord announcement.
- **Staff** hold any set of roles: hosts (run the tournament, build the bracket, manage users, teams, stages and Discord), map poolers (build per-stage pools), referees (run live matches in osu! multiplayer rooms through the referee page) and streamers (take matches to broadcast).

No group's experience outranks the others when trade-offs come up. Public pages favour players and spectators, and `/admin` favours staff.

## Product Purpose

One place where the whole tournament runs, from sign-up to results: team eligibility, qualifier lobbies, the double-elimination bracket, rescheduling, map pools, live refereeing and the Discord server all work from the same data. It exists so organizers don't have to juggle spreadsheets and DMs, and so players and spectators always see the current state. Success means a tournament that runs from start to finish without manual bookkeeping, where players and spectators can find their match, time and pool on their own.

## Positioning

A purpose-built site for this one tournament and its community, not a generic tournament platform. Its distinctive mechanisms:

- **Rank-sum team eligibility**, using the tournament's own formula (`backend/src/team_rank.rs`). Teams are watched for drifting over the limit.
- **A referee page that speaks Bancho**: an in-browser IRC relay that creates and runs the osu! room, tracks bans, picks and scores against the pool, and saves room chat for moderation.
- **A Discord server kept in sync** with the site: roles, per-match channels with a live match card, reminders, announcements, results and staff confirm-time requests.

## Operating Context

- Discord is where the community lives. The bot pings players, staff and subscribers about matches, and those messages lead back to the site.
- Matches are played in osu! multiplayer rooms (`osu.ppy.sh/community/matches/<id>`). Rooms are named with the `112S` prefix.
- Referees run matches live: the referee page with the osu! client next to it, under time pressure, sometimes two matches at once.
- Hosts set the tournament up ahead of each phase: stages, pools, qualifier lobbies, the bracket board, team freezes and rank refreshes.
- The site is still being tested and isn't public yet. Dev login has to be off before launch.

## Capabilities and Constraints

- Sign-in only through osu! OAuth. Discord linking is required to invite or accept a teammate.
- Terms used across the UI: stage, mappool, qualifier lobby (Elemeler), bracket match (W1, L3…), referee, streamer, host, map pooler, NM/HD/HR/DT/FM/TB mod categories.
- Editor notes and referee notes never appear on public pages. Players' Discord accounts are staff-only. Referees' Discord accounts are public on purpose.
- Anything done by dragging also needs a button or select alternative. Icon-only buttons always carry an accessible name.
- The full feature and data model is documented in `CLAUDE.md`.

## Brand Commitments

- The name is **112servis**: one word, lowercase, no space. The site title is "112servis 2026 Turnuva".
- The site is **Turkish only**, and no English version is planned. Technical terms used by the osu! community (mappool, stage, bracket, referee) stay as the community says them.
- No visual identity is fixed yet. There's no logo or artwork. The current dark osu!-pink design is work in progress, not a commitment.
- Designers will provide the colour scheme and graphics later. Until then, keep every colour behind a role token and leave room for artwork without inventing any. When the designs arrive, they replace those placeholders.

## Evidence on Hand

- No logo, past editions, results, testimonials or sponsor material exist yet. Don't invent any.
- Real data on the site comes from the osu! API (ranks, beatmap metadata, match scores) and from team banners players upload.
- `backend/seed/` holds dummy accounts, teams and matches for testing only. They aren't real content.

## Product Principles

1. **The site is the source of truth.** Discord, osu! rooms and every view show the same state, and anything that changes it is visible right away.
2. **Every audience can help itself.** A player, spectator or staff member should find their match, time, pool or task without asking an organizer.
3. **Staff tools are built for the moment of use**: fast, recoverable and hard to get wrong mid-match.
4. **Private stays private.** Notes and contact details reach only the people meant to see them.
5. **Built for this community.** Turkish voice, osu! conventions and the tournament's own rules come before generic tournament-platform patterns.
