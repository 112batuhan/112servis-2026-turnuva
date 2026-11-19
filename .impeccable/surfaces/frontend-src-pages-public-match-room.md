---
version: 1
slug: "frontend-src-pages-public-match-room"
primary_target: "frontend/src/pages/public/match/room"
related_targets: []
---

# Surface brief: the players' match room (`/match/:id/oda`)

Scope: the room page a match's own players run their match from, beside the osu! client. Mode: Operate. Inherits the site's world (DESIGN.md) unchanged; the composition is pinned by the owner.

Audience and job: the four players of a bracket match (or a lobby's players). Choose the team's warmup, ban and pick on the clock, see whose turn it is and how long is left at every moment, abort a broken start, report a problem, pull the emergency stop. Picks, bans and warmups are taken only here (removed from the osu! chat, 2026-11-19); -abort, -rapor and -durdur work in both. The chat on the page is a read-only mirror.

Constraints: hold-to-confirm 3 s on a map (keyboard: hold Space/Enter); every refusal comes back as a page error, not a room line; the public match page stays for spectators, and links say "Maç odası" for the match's own players.

## Direction contract

THESIS: The pool is the page. A match room where the mappool is the playing field the players act on, not a sidebar to a scoreboard; it refuses the cockpit's three equal panels and the spectator page's score-first middle.

OWN-WORLD: The site's slate sheet with its frost accent and team sides. Map tiles over their cover art behind the scrim, grouped under 3px rules in each category's colour, labels as condensed display figures in that colour. Frost means "your turn" and "yours"; osu!'s own blue and red (the referee page's `.osu-team-colors`, owner's call 2026-11-19) mean whose ban or pick and name the teams in the header; danger means ban. The whole biggest pool (6 NM 2 HD 3 HR 4 DT 3 FM 1 TB) stays in view on a desktop: compact tiles (slot, title, difficulty, cover), one centred row per category. Boxed tool panels only where a tool needs one (score column, chat well).

STORY: A player opens it from Discord or their team page, sees at once what the match waits for and whose clock is running, holds their ban for three seconds and watches it land, then follows the score and the chat without leaving the page.

FIRST VIEWPORT: Desktop: under a one-line header, left two-thirds the turn bar (phase in display type, the team in its colour, a 48px countdown; frost when it's your team's turn, warmup input or roll instruction in its place) above the pool tiles; right third the score sheet with the abort / rapor / durdur block pinned at its foot; the room chat as a full-width well along the bottom. Phone: turn bar sticky, Havuz / Maç / Sohbet tabs.

FORM: Owner-pinned composition (pool left, score right, chat bottom); no roll. Signature interaction: the 3-second hold, a fill sweeping across the tile in the ban or pick colour with the seconds counting down, draining back if released.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
