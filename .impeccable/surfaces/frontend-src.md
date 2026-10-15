---
version: 1
slug: "frontend-src"
primary_target: "frontend/src"
related_targets: []
---

# Surface brief: whole site (frontend/src)

Scope: every public page, the player pages and the admin shell. Mode: Operate for the working pages (team, qualifiers, bracket, mappool, admin, referee); the landing page is the one Persuade-leaning surface (a "Şimdi" board people land on from Discord).

Audience and job: Turkish osu! players, spectators and staff (see PRODUCT.md). Find your match, time, pool or task without asking an organizer.

Constraint from the owner (2026-10-15, updated the same day): the designers' first banner (`design/bannertest.png`) arrived. The owner chose to adapt the site to it as "palette + atmosphere": its palette replaces the placeholders site-wide, its grain and gear linework go on the page ground, and the landing headline is rebuilt around the banner PNG as the hero. The fixture-sheet structure stays. The accent is frost white-teal, from the glow of the mark: an ice crystal drawn with the digits 1, 1 and 2. Every colour stays a role token; no artwork beyond the banner and the mark cut from it.

## Direction contract

THESIS: The tournament read like a Turkish sports fixture sheet (fikstür): big condensed headlines, numbers as the heroes, matches as versus rows, ruled lists instead of boxes in boxes. Refuses the dashboard default of same-size bordered cards and small grey labels.

OWN-WORLD: The banner's cold slate ground with faint gear linework in two corners and a film of grain behind the content; frost-white ink; one frost accent with the crystal's glow on lit things only (the live dot, the current stage); the wordmark and the banner's line in expanded light Archivo. Archivo (variable, condensed width, heavy) for headings, match codes, scores, countdowns and values, always tabular; Fira Sans for body and controls. Horizontal rules (one heavy, one hairline) group content instead of nested borders. Team red/blue is the structure of every head-to-head; one brand accent marks live, "yours" and the primary action only. Every colour is a role token, so a final palette is a value swap.

STORY: A visitor sees what stage the tournament is in and what is next, finds their own team and match, and reaches the pool or bracket in one click; staff see their tools with clearer weight and danger separation.

FIRST VIEWPORT: Landing: the banner feathered into the page (beside the headline from 1200px wide, otherwise the trophy and crystal alone above it); the wordmark "112servis 2026" in expanded light Archivo, the line "112'nin Son Kış Turnuvası" set as type, the facts and the login / "Maçlar" action under them; directly below, the "Sıradaki maçlar" fixture: 3-4 versus rows (time, red team, score or "vs", blue team, match code) at full width; a stage track (Elemeler → … → Grand Final) with the current stage lit; for a signed-in player, a "Takımın" strip above the fixture with their next match.

FORM: User-pinned direction (Turkish community poster / printed fixture), applied through type, density and one signature move per Operate rules; no concept-seed roll (pinned). Signature move: the versus fixture row, reused on the landing, team page, team matches and bracket list.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
