---
version: 1
slug: "frontend-src"
primary_target: "frontend/src"
related_targets: []
---

# Surface brief: whole site (frontend/src)

Scope: every public page, the player pages and the admin shell. Mode: Operate for the working pages (team, qualifiers, bracket, mappool, admin, referee); the landing page is the one Persuade-leaning surface (a "Şimdi" board people land on from Discord).

Audience and job: Turkish osu! players, spectators and staff (see PRODUCT.md). Find your match, time, pool or task without asking an organizer.

Constraint from the owner (2026-10-15): the colour scheme and graphics will come from designers later. Structure, typography and hierarchy are built now; every colour stays a role token with a placeholder value, and no invented artwork or logo is added.

## Direction contract

THESIS: The tournament read like a Turkish sports fixture sheet (fikstür): big condensed headlines, numbers as the heroes, matches as versus rows, ruled lists instead of boxes in boxes. Refuses the dashboard default of same-size bordered cards and small grey labels.

OWN-WORLD: Placeholder near-black ground. Archivo (variable, condensed width, heavy) for headings, match codes, scores, countdowns and values, always tabular; Fira Sans for body and controls. Horizontal rules (one heavy, one hairline) group content instead of nested borders. Team red/blue is the structure of every head-to-head; one brand accent marks live, "yours" and the primary action only. Every colour is a role token waiting for the designers' palette.

STORY: A visitor sees what stage the tournament is in and what is next, finds their own team and match, and reaches the pool or bracket in one click; staff see their tools with clearer weight and danger separation.

FIRST VIEWPORT: Landing: the wordmark headline "112servis 2026" set huge in condensed Archivo with the current stage name beside it; directly below, the "Sıradaki maçlar" fixture: 3-4 versus rows (time, red team, score or "vs", blue team, match code) at full width; a stage track (Elemeler → … → Grand Final) with the current stage lit; for a signed-in player, a "Takımın" strip above the fixture with their next match; the login or "Maçlara git" action at top right of the headline block.

FORM: User-pinned direction (Turkish community poster / printed fixture), applied through type, density and one signature move per Operate rules; no concept-seed roll (pinned). Signature move: the versus fixture row, reused on the landing, team page, team matches and bracket list.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
