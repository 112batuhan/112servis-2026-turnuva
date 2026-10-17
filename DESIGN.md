---
name: 112servis 2026 Turnuva
description: A Turkish 2v2 osu! tournament read like a printed fixture sheet, in the cold slate and frost of the designers' test banner; every colour is consumed by role.
colors:
  ground: "#171e23"
  ground-sunken: "#11171b"
  surface: "#1d262c"
  surface-raised: "#26323a"
  hover-wash: "rgba(207, 236, 242, 0.06)"
  hairline: "#2c3a41"
  hairline-strong: "#40535c"
  linework: "#2a383f"
  ink: "#e9eef0"
  ink-muted: "#a0b1b8"
  ink-subtle: "#8a9da4"
  brand: "#a3dae4"
  brand-hover: "#c3e8ee"
  on-brand: "#0e171b"
  brand-soft: "rgba(163, 218, 228, 0.13)"
  focus: "#cdeef4"
  team-red: "#f06c74"
  team-red-soft: "rgba(240, 108, 116, 0.14)"
  team-blue: "#62acff"
  team-blue-soft: "rgba(98, 172, 255, 0.14)"
  success: "#5fd38d"
  warning: "#e8b84a"
  danger: "#f3877e"
  danger-soft: "rgba(243, 135, 126, 0.12)"
  info: "#aab2ff"
  info-soft: "rgba(88, 101, 242, 0.18)"
  now-line: "#d93036"
  discord: "#5865f2"
  discord-hover: "#4752c4"
  admin-bar: "#1f2438"
  admin-bar-border: "#3b4470"
typography:
  display:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "clamp(44px, 7.4vw, 88px)"
    fontWeight: 300
    lineHeight: 0.95
    letterSpacing: "-0.03em"
    fontVariation: "\"wdth\" 125"
  tagline:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 300
    lineHeight: 1.2
    fontVariation: "\"wdth\" 125"
  wordmark:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    fontVariation: "\"wdth\" 125"
  headline:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "44px"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.01em"
    fontVariation: "\"wdth\" 68"
  title:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.01em"
    fontVariation: "\"wdth\" 68"
  section:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 800
    lineHeight: 1
    fontVariation: "\"wdth\" 68"
  figure:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 700
    lineHeight: 1.1
    fontFeature: "\"tnum\""
    fontVariation: "\"wdth\" 68"
  score:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "30px"
    fontWeight: 800
    lineHeight: 1
    fontFeature: "\"tnum\""
    fontVariation: "\"wdth\" 68"
  body:
    fontFamily: "Fira Sans, system-ui, -apple-system, Segoe UI, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.5
  control:
    fontFamily: "Fira Sans, system-ui, -apple-system, Segoe UI, sans-serif"
    fontSize: "14px"
    fontWeight: 600
  label:
    fontFamily: "Fira Sans, system-ui, -apple-system, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 600
  mono:
    fontFamily: "Fira Code, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "13px"
    fontWeight: 400
rounded:
  sm: "6px"
  md: "8px"
  lg: "12px"
  full: "999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "24px"
  "6": "32px"
  "7": "48px"
components:
  button-primary:
    backgroundColor: "{colors.brand}"
    textColor: "{colors.on-brand}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "36px"
  button-primary-hover:
    backgroundColor: "{colors.brand-hover}"
    textColor: "{colors.on-brand}"
  button-primary-disabled:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.ink-subtle}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "36px"
  button-secondary-hover:
    textColor: "{colors.ink}"
  button-danger:
    backgroundColor: "transparent"
    textColor: "{colors.danger}"
    rounded: "{rounded.md}"
    padding: "0 16px"
    height: "36px"
  button-danger-hover:
    backgroundColor: "{colors.danger-soft}"
    textColor: "{colors.danger}"
  input:
    backgroundColor: "{colors.ground-sunken}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "36px"
  badge:
    backgroundColor: "{colors.hover-wash}"
    textColor: "{colors.ink-muted}"
    typography: "{typography.label}"
    rounded: "{rounded.full}"
    padding: "2px 10px"
  nav-link-active:
    backgroundColor: "{colors.brand-soft}"
    textColor: "{colors.ink}"
    typography: "{typography.control}"
  fixture-row:
    textColor: "{colors.ink}"
    padding: "12px 8px"
  fixture-row-hover:
    backgroundColor: "{colors.hover-wash}"
  fixture-score:
    textColor: "{colors.ink-muted}"
    typography: "{typography.score}"
    rounded: "{rounded.sm}"
    width: "76px"
  own-row:
    backgroundColor: "{colors.brand-soft}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
---

# Design System: 112servis 2026 Turnuva

## Overview

**Creative North Star: "The Fikstür Sheet"**

The tournament reads like a Turkish sports fixture sheet: big condensed headlines, numbers as the heroes, matches as versus rows, and content grouped by horizontal rules rather than boxed in nested cards. Every public page (landing, teams, qualifiers, bracket list, team page) is a stack of ruled sections under a heavy-ruled headline, with dense rows on hairlines beneath. The admin side keeps boxed tool panels where a tool genuinely needs a container (the referee page, the map pool editor, the bracket board), but its page headers follow the same sheet.

The sheet is printed in the world of the designers' first banner (`design/bannertest.png`, "112'nin Son Kış Turnuvası", still a test version): a cold slate ground, frost-white ink, and the icy glow of the tournament's mark, an ice crystal drawn with the digits 1, 1 and 2. The banner also lends the page its atmosphere (faint gear linework in two corners and a film of grain, behind the content) and the wordmark its voice (the display face expanded and light). The artwork is the designers'; the site uses what was cut from it and invents none of its own.

Density is high and calm: a 16px body, a dense 4px-based spacing scale, rows of about 60px, and a 1440px content width. Motion is limited to short colour and background transitions on hover and focus.

**Key Characteristics:**
- Condensed heavy Archivo (68% width, 700–900) for every heading, code, score, time, countdown and value; always tabular figures.
- The wordmark and the banner's line in the same Archivo expanded to 125% and light; nowhere else.
- Fira Sans for body and controls; Fira Code only for ids.
- A heavy ink rule opens each page and section; hairline rules separate rows.
- Team red and team blue structure every head-to-head; one frost accent marks live, yours, current and the primary action.
- Slate ground with gear linework and grain behind the content; the crystal mark in the nav and favicon; the banner as the landing hero.
- Every colour is a role token taken from the test banner, so the final banner is a value swap.

## Colors

A cold slate palette from the banner: a blue-grey ground and its frost ink, one frost accent from the crystal's glow, the two osu! team colours, and status colours.

**The Banner Palette Rule.** The palette comes from the designers' test banner, and every colour is still consumed by role only: components reference role tokens (`--color-*`, `--rule*`, `--glow` in `frontend/src/styles/tokens.css`), and a raw colour value in a component stylesheet is a defect (mask gradients, which carry only alpha, are the exception). When the final banner arrives: (1) change the values in `tokens.css` only, keeping every role name; (2) keep `on-brand` legible on `brand` (frost is a light fill, so it takes slate ink); (3) keep team red and team blue distinguishable from each other and from `brand` and `danger`; (4) re-export the assets from the final banner as `frontend/src/assets/README.md` describes, keeping the file names; (5) re-run the document pass so this file and its sidecar record the new values. Soft variants (`brand-soft`, `team-*-soft`, `danger-soft`) are the base colour at 12–18% alpha, and several components derive tints with `color-mix()` from the base team and brand colours, so they follow automatically.

### Primary
- **Crystal Frost** (`brand`): the glow around the 112 crystal, the tournament's one accent. Used only for: live matches ("Canlı" with a dot), anything that is the viewer's own (own team name in a fixture row, bracket row and bracket card, own lobby, the "Takımın" strip), the current position (active nav link, the lit stage on the stage track), the one primary action per view, and the wordmark's year. `brand-hover` is its lighter hover step; `on-brand` is the slate ink on a frost fill. `focus` is a paler frost for focus outlines.
- **Frost Wash** (`brand-soft`): the "yours" background wash (own bracket row and card row, own team strip, active nav link).

### Secondary
- **Team Blue** and **Team Red**: osu! multiplayer sides, as the tournament room seats them: team 1 is always blue (slots 1-2), team 2 always red (slots 3-4), in every head-to-head: both halves of a fixture score, bracket card and list score cells, and the referee page's team columns. `team-*-soft` are their washes.

### Tertiary
- **Status**: `success`, `warning`, `danger`, `info`, each with a soft wash where used as a badge or alert ground. `now-line` is the calendar's current-time line. `discord` is Discord's own button colour, used only on the Discord link action. Status colours carry state, never decoration.

### Neutral
- **Slate Ground** (`ground`, the banner's slate): the page. **Ground Sunken**: input wells. **Surface** and **Surface Raised**: admin tool panels, cards on the profile and Discord pages, popovers, bracket board cards. **Hover Wash**: translucent frost over any row on hover.
- **Hairline** and **Hairline Strong**: row rules and control borders.
- **Gear Linework** (`linework`): the colour of the gear masks in the page corners, a step off the ground so they stay faint. Used for nothing else.
- **Frost Ink**, **Ink Muted**, **Ink Subtle**: primary text, secondary text (losing team, L codes, stage names, countdowns, the landing tagline), and placeholder or empty text.
- **Admin Bar** and **Admin Bar Border**: the indigo-tinted top bar on `/admin/*`, so staff always know which side of the site they are on.

**The One Accent Rule.** Frost appears only for live, yours, current position, the single primary action and the wordmark's year. It never decorates a heading, a divider or a category.

**The Strong Yours Rule.** Frost sits close to the ink, so "yours" is never carried by a wash alone: an own team name is frost at 700, an own lobby title frost at 900, and an own bracket row that is hovered steps to a deeper frost wash (10%) so hover never reads as just "yours".

**The Sides Rule.** Any time two teams face each other, team 1 is blue and team 2 is red (the room's slots 1-2 and 3-4), and both colours appear. A head-to-head without its side colours is incomplete.

## Typography

**Display Font:** Archivo variable (with Arial Narrow, system-ui), set at 68% width (`--font-display-stretch`), weights 700–900; loaded across widths 62–125 and weights 200–900.
**Wordmark voice:** the same Archivo at 125% width (`--font-wordmark-stretch`), light (`--font-wordmark-weight`), after the banner's lettering.
**Body Font:** Fira Sans (with system-ui, -apple-system, Segoe UI), 400–700.
**Mono Font:** Fira Code (with ui-monospace), for ids and code only.

**Character:** A poster headline face over a sturdy humanist sans. The condensed display face carries everything people scan for or compare; the sans carries reading and operating; the expanded light cut is the tournament's own name, said quietly.

### Hierarchy
- **Display** (300 at 125% width, clamp 44–88px, line-height 0.95, -0.03em): the landing wordmark "112servis 2026" only (`--text-hero`).
- **Tagline** (300 at 125% width, 24px, muted ink): the banner's line "112'nin Son Kış Turnuvası", set as live type under the landing wordmark.
- **Wordmark** (500 at 125% width, 16px): "112servis 2026" in the top bar beside the crystal.
- **Headline** (800, 44px, 34px under 560px wide; line-height 1): page titles, every other `h1` (`--text-title`). A count beside the title is set as a muted display figure at 30px, not a pill.
- **Title** (800, 40px): landing section heads ("Sıradaki maçlar", "Son sonuçlar", "Takımın").
- **Section** (800, 24px): every `h2`.
- **Score** (800 winner / 600 loser, 30px; 24px in narrow rows): the score halves of a fixture row.
- **Figure** (700, 18px, tabular): match times, match codes (800), stage names on the track, "Canlı", bracket card scores; countdowns at 14px/600. The `.num` utility applies the display face with tabular figures to any value inline.
- **Body** (400, 16px, line-height 1.5): reading text; the landing lead line is 18px muted and caps at 72ch.
- **Control** (600, 14px; 13px small): buttons, nav links, inputs.
- **Label** (600, 12px): badges, stage names under match codes, small meta.
- **Mono** (Fira Code): osu! ids and multiplayer room ids only.

**The Two Widths Rule.** Archivo has two voices and they never trade places: condensed and heavy for headings and numbers, expanded and light for the tournament's name and the banner's line. A page title, a score or a section head is never set expanded; the wordmark is never set condensed.

**The Numbers Are Heroes Rule.** Any score, time, countdown, match code, rank or team value is set in the condensed display face with tabular figures. Never set a number someone compares in the body sans.

**The Mono Is For Ids Rule.** Fira Code appears only on identifiers. It is not a "technical" flavour face.

## Layout

Pages sit in a 1440px column (`--content-width`, narrow pages 960px) with 32px top, 16px side and 48px bottom padding. Every page opens with a header: the `h1` on the left, a count beside it, trailing controls pushed right, and a heavy rule under it with 24px below.

The landing head is the exception that keeps the rule: from 1200px wide it is two columns, the wordmark, tagline, facts and actions on the left and the banner (up to 680px) on the right, bottom-aligned, over the heavy rule; under 1200px it stacks, with the crest-only crop (trophy and crystal, at most clamp(160px, 28svh, 260px) tall) centred above the wordmark so the fixture stays in the first viewport.

Spacing follows a dense 4px-based scale (4, 8, 12, 16, 24, 32, 48). Rows pad 12px vertically and 8px horizontally; sections on the landing are separated by 48px; two-up grids (teams, lobbies) gap 32px by 24px and collapse to one column under 720px (teams) or 1100px (lobbies, which need about 500px per slot).

**The Ruled Sheet Rule.** Content is grouped by rules, not boxes: `--rule-heavy` (3px ink) under page and section headings and over each lobby and team block; `--rule` (1px hairline) between rows. Public pages prefer ruled lists over bordered cards. Admin tool panels may keep boxes.

Responsive behaviour is by container, not page: the fixture list is a size container and its rows rearrange under 720px of their own width. Controls grow to 44px tall on coarse pointers (`--control-height`, `--icon-button-size`). Reduced motion collapses every transition.

## Elevation & Depth

Flat by default. Depth on public pages comes from rules and tonal steps (ground, surface, surface raised), not shadows. Shadows exist only for things that genuinely float: popovers and menus, and the single centred card on the profile and Discord-link pages. Brand-coloured 1px rings and 3px soft-brand focus halos are state indicators, not elevation.

Behind everything, fixed to the viewport, sits the banner's atmosphere: gear linework masks in the top-left (up to 560px) and bottom-right (up to 640px) corners in the linework colour, and a neutral grey grain at 7% in overlay blend, which averages out so it never lifts the slate. Neither is depth: they lie on the ground, under the content, and surfaces stay clean.

### Shadow Vocabulary
- **Popover** (`0 10px 28px rgba(0,0,0,0.5)`): the notification panel, team popovers, the mini player, map card menus and the bracket board's floating controls.
- **Floating card** (`0 12px 32px rgba(0,0,0,0.35)`): the single centred card on card pages.
- **Crystal glow** (`--glow`, a 10px zero-offset frost halo): only on lit things.

**The Flat Sheet Rule.** A row, a section or a team block never takes a shadow. If it needs separation, it gets a rule.

**The Lit Things Rule.** The crystal's glow is the one sanctioned exception to the flat sheet and to zero-offset halos, earned by the banner: it is allowed only on the live dot and on the current stage's rule on the stage track. A button, a card, a heading or the primary action never glows.

## Shapes

Gently rounded controls (8px) on a square-edged sheet. Rules run edge to edge with no rounding; buttons, inputs and the "yours" strip use 8px; the score block uses 6px; admin panels and centred cards 12px; badges, avatars and round icon buttons are full pills. The live dot is an 8px circle.

The one drawn silhouette is the 112 crystal (`.brand-mark`): an alpha matte cut from the banner, used as a CSS mask so it takes the current text colour, at a 199:220 aspect. The banner on the landing has no frame: its edges are feathered into the ground with mask gradients (a linear fade on all four sides of the wide banner; an elliptical fade on the crest crop, whose centre is lighter than the page).

## Components

### Buttons
- **Shape:** gently rounded (8px), 36px tall (30px small, 48px large, 44px on touch).
- **Primary:** frost fill with slate ink, 600 weight 14px. One per view.
- **Hover / Focus:** brand-hover fill; focus is a 2px focus-colour outline offset 2px.
- **Disabled primary:** grey, not dim frost: surface-raised fill, hairline border, subtle ink, full opacity. Other buttons drop to 50% opacity.
- **Secondary:** transparent with a strong hairline border and muted ink; hover and pressed raise to ink.
- **Danger:** transparent with a danger border at 45% and danger ink; hover adds the danger wash. Kept visually apart from primary on staff screens.
- **Icon-only:** 32px square, transparent, muted ink; hover adds the hover wash. Always carries an accessible label.

### Badges
- **Style:** full pill, 12px/600, hover-wash ground and muted ink; status variants use the status colour at 16% over its own ink.

### Inputs / Fields
- **Style:** sunken well, strong hairline border, 8px radius, 36px tall.
- **Focus:** border turns frost with a 3px frost-wash halo.

### Navigation
- **Top bar:** the brand first: the crystal mark (26px tall, in ink) beside the wordmark "112servis 2026" in the expanded Archivo at 500, the year in frost; then text links in muted 14px ink; hover adds the wash; the active link gets a frost-wash pill with a 2px frost underline. On `/admin/*` the bar turns the admin tint and its links become the admin tabs.

### Landing hero
The banner with its lettering cropped off, feathered into the ground, beside the headline from 1200px wide (the 1316px export, 760px for lower densities); under 1200px the crest-only crop (trophy and crystal) with an elliptical feather above the headline. Under it or beside it: the wordmark in Display, the tagline "112'nin Son Kış Turnuvası" in Tagline type (muted), the facts line, and the actions (login as the one primary, Mappool or "Maçlar" secondary) under the facts, all over the heavy rule. The image carries a Turkish alt text describing the crystal and trophy.

### Fixture Row (signature)
The versus row of the fixture sheet (`components/FixtureRow`), used on the landing and in team match history (the team page and every team card's "Maçlar" overlay); the bracket list repeats its score treatment. A five-column line on a hairline: **time | blue team | score | red team | code and stage**.
- **Time:** display figure 18px, countdown under it in muted 14px; a live match shows "Canlı" in frost with an 8px dot that carries the crystal glow.
- **Teams:** team 1 hugs the score from the left (right-aligned), team 2 from the right; the score halves carry the team colours, the names stay in ink; banners sit beside the name and disappear when empty. Losing team in muted 400; the viewer's own team in frost 700; empty slots in subtle ink.
- **Score:** two halves, each tinted in its team colour at 30% and underlined with a 3px bar in that team colour; winner's figure in ink 800, loser's muted 600; unplayed shows a muted "vs" across both halves.
- **Code and stage:** right-aligned; winners-side codes (W58) in ink, losers-side codes (L12) muted; the stage name under it in 12px muted.
- **Narrow (under 720px of its own width):** time and code move to a line above; banners hide; scores drop to 24px.
- **Hover:** hover wash across the row.

### Ruled blocks (lobbies, teams)
Each lobby and each team on public pages is a block under a heavy rule, not a box; the viewer's own block swaps the rule to frost and its lobby title to frost at 900; an over-limit team swaps it to danger. Rows inside use hairlines.

### Bracket card and list
Board cards are compact surface boxes (the board is an admin-style tool surface) with each team's score cell tinted in its team colour (34%); winner in ink 800, loser muted; the viewer's team row carries the frost wash and its name in frost 700; the selected card gets a frost ring. The list view uses the fixture score treatment (tinted, underlined halves), the frost wash on own rows with the own team's name in frost 700, a deeper 10% frost wash when an own row is hovered, muted (not subtle) ink for empty slots on own rows, and a 3px frost inset on the selected row.

### Match page
The public `/match/:id` wears the referee page's cockpit unchanged (pool | match | chat in boxed tool panels, one screen tall, phone tabs under 760px): it is a tool surface people watch, so it keeps the admin-style boxes rather than the ruled sheet. Nothing that runs the room appears. The header is one row on a grid: the back link and the match's code, then the facts (the time as a display figure, the stage as plain text, "Hakem"/"Yayıncı" muted with the names in ink, set apart by muted dots; spacing alone on a phone, where the facts drop to a second line), then the match's state pinned to the right. While it's live that state is the page's only live mark: a 40px frost capsule (frost wash, frost border at 45%) holding "Canlı" large (24px, 10px glowing dot) and, behind a frost hairline, who's watching: an eye and the word muted around the count as an ink display figure (the word drops on narrow phones), a click opening a raised popover headed "N kişi izliyor" with the signed-in spectators (avatar and osu! name, the viewer first, marked "sen"), then "+N giriş yapmamış izleyici" and a subtle note that signed-in viewers appear by name. A finished match shows "Bitti" (or "Hükmen bitti") there in success display type. Nothing else on the page says "Canlı": the score sheet's line reads "Map bittikçe güncellenir", the chat has no foot while it streams, and the phone's chat tab counts the messages. The room bar ends in a secondary "osu!'da aç"; before the room opens, the map clock's place holds a dashed-outline block with the start time as a display figure and its countdown. The chat's "Sadeleştir" is a small switch (sunken track, thumb turning frost when on) with the hidden count beside it, and its foot appears only while reconnecting (a warning dot) or once the match is over (the saved chat). The referee's own lines take the info colour. Each player row leads with the slot they're in as a jersey number: a solid 32px square in the colour of the team the room has them on, the figure in slate ink at 900 (a raised neutral square without a colour, a hollow dashed one with a dash while they're away); rows stay pinned in slot order (blue's 1-2, then red's 3-4) wherever the player actually is, so a chip whose colour or figure doesn't match its rule and row is out of place. On the referee page such a chip also carries a 10px danger dot on its corner (where they belong is in its tooltip and label, not on the row), and the chip is a button that opens the slot picker: a raised popover with every slot in four columns (1-2 edged blue, 3-4 red), each holding its occupant's chip and name (a free slot hollow, the player's own marked "yeri"), the two colours as pressed toggles, and a primary "Yerine gönder" while they're out of place. Until the match is over each team's players sit under its banner (a 4:1 crop with 8px radius, under the side's 3px rule, the name and "x/2 odada" set on its scrim-darkened foot with the team card's outline; no banner shows the team's initial, cropped, so both sides keep the same height). When the stage has a best of, the score sheet sets it as a muted display figure beside "Skor" ("Bo9"), lays each team's wins as 4px pips under its figure (ink when won, ink at 22% when not, as many as it needs), and under the heavy rule a race line in display type: "Maç sayısı" with the team in its colour, "<team> maçı aldı" on its team's soft wash (the referee's "Maçı bitir" at its end), or a warning-coloured "Skor Bo9'u aşıyor". Once it's over the teams are the teams page's ruled `TeamCard`s, two to a row, each card's heavy rule in its side's colour. The link to the page (`MatchPageLink`) is a monitor icon (text "Maç sayfası" where there's room), muted at rest and frost while the match is live ("Canlı takip et"); a live fixture row says "Canlı takip et" in frost where its stage would be. Admin pages leave it out for referees and hosts, who have the referee page.

### Room bot (referee page)
The bot that runs the room sits in the match column's room bar as a 24px pill with a strong hairline: a 7px status dot (success in the room, warning on its way, danger kept out), the bot icon, its name in ink, and the state in muted words unless it's simply in the room; kept out, the pill's border turns danger at 45% and the state takes the danger ink. When the bot can't run the room, the column opens with one notice block (danger at 45% border, danger wash, 8px radius, never a side stripe): the problem as a 14px ink title, the reason muted under it, and the one way out at its end ("Başka botla yeni oda aç" as the primary, or a secondary "Tekrar dene" for a room waiting on its maker's `!mp addref`). The room's referees live in the room-settings drawer as a hairline-ruled list (name, a close icon to take them off) over a select and a secondary "Ekle". Every command button stays visible and goes disabled, with the bot's reason as its title, rather than disappearing.

### Bancho bots page
`/admin/bancho` is a ruled list under the page's heavy rule, one hairline row per bot read across: a 9px status dot and the username (600, ink; muted when off) with its state under it (the refusal in danger at 600) and the connection's reason in subtle 12px; the room meter, one 14px cell per room Bancho allows, filled ink for each room that counts (muted when the bot is offline) beside the count as a display figure, and the rooms as small outlined chips with their match code in display figures (dashed and muted when the bot isn't in them, "hakem" for one it was only added to); the lines sent in the last window as a display figure, with what's waiting in warning under it; and the controls (secondary "Kapat" or primary "Aç", then icon buttons for the password and delete). The header's count beside the title is the connected bots over the active ones, and its aside the rooms in use over the rooms the connected bots allow, in warning when full. Under 760px of its own width the row stacks: who and the controls, then the rooms, then the lines. The add form follows under its own heavy rule.

### "Yours" strip
The signed-in player's team and next match on the landing: a frost-wash row with 8px radius (22% frost on hover), its section head ruled in frost instead of ink, the "Takımın" label in frost display type.

### Stage track
The stages in play order as a horizontal strip of 3px top rules: done in muted, upcoming in hairline, current in frost with the crystal glow on its rule and "Şu an" under its name.

### Atmosphere and artwork
- **Gear linework and grain:** on the page ground only, site-wide, behind all content. Never inside a surface, a card or a section.
- **The crystal mark:** the nav brand and the favicon / touch icon (the mark in frost ink on the slate ground) only.
- **The banner:** the landing hero only, in the two crops above.
All three come from the designers' banner and are re-exported per `frontend/src/assets/README.md`; the gear SVGs are drawn after the banner's cogs, not traced.

## Do's and Don'ts

### Do:
- **Do** reference role tokens from `frontend/src/styles/tokens.css` for every colour, size, radius and timing.
- **Do** take the palette from the designers' banner and swap it by changing values in `tokens.css` while keeping role names; re-export the assets per `frontend/src/assets/README.md`.
- **Do** set every score, time, countdown, code and value in condensed Archivo (68% width) with tabular figures.
- **Do** keep the expanded light Archivo (125%) for the wordmark and the banner's line.
- **Do** group public content with a heavy rule under headings and over blocks, and hairlines between rows.
- **Do** show both team colours in every head-to-head: team 1 blue, team 2 red.
- **Do** reuse the fixture row for any list of matches.
- **Do** mark the viewer's own team, lobby or row with frost ink at full weight, a frost rule or the frost wash, never the wash alone on a name.
- **Do** show disabled primary buttons as grey (surface-raised, subtle ink), not as a dimmed frost fill.
- **Do** feather the banner into the ground rather than framing it.

### Don't:
- **Don't** invent artwork, illustration or a new mark; use only what the designers supplied, in the places listed under Atmosphere and artwork.
- **Don't** put the glow on anything but the live dot and the current stage.
- **Don't** use frost for decoration, headings, dividers or categories; it means live, yours, current or primary.
- **Don't** box public lists in same-size bordered cards or nest boxes inside boxes; use rules.
- **Don't** put shadows on rows, sections or blocks, or let the gear linework and grain into a surface.
- **Don't** set numbers in the body sans or use Fira Code outside ids.
- **Don't** add small uppercase tracked labels above headings or fields; let the heading and the value carry the hierarchy.
