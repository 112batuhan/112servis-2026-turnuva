---
name: 112servis 2026 Turnuva
description: A Turkish 2v2 osu! tournament read like a printed fixture sheet; every colour is a placeholder behind a role token.
colors:
  ground: "#15161b"
  ground-sunken: "#111217"
  surface: "#1e2027"
  surface-raised: "#272932"
  hover-wash: "rgba(255, 255, 255, 0.06)"
  hairline: "#33353f"
  hairline-strong: "#4a4d5a"
  ink: "#f1f2f5"
  ink-muted: "#a5a8b3"
  ink-subtle: "#8b8e9a"
  brand: "#ff66ab"
  brand-hover: "#ff85bc"
  on-brand: "#1c0b14"
  brand-soft: "rgba(255, 102, 171, 0.14)"
  focus: "#ffb3d6"
  team-red: "#f2666e"
  team-red-soft: "rgba(242, 102, 110, 0.14)"
  team-blue: "#5ea8ff"
  team-blue-soft: "rgba(94, 168, 255, 0.14)"
  success: "#5fd38d"
  warning: "#e8b84a"
  danger: "#f3877e"
  danger-soft: "rgba(243, 135, 126, 0.12)"
  info: "#aab2ff"
  info-soft: "rgba(88, 101, 242, 0.18)"
  now-line: "#d93036"
  discord: "#5865f2"
  discord-hover: "#4752c4"
  admin-bar: "#22243a"
  admin-bar-border: "#3d4170"
typography:
  display:
    fontFamily: "Archivo, Arial Narrow, system-ui, sans-serif"
    fontSize: "clamp(56px, 10vw, 96px)"
    fontWeight: 900
    lineHeight: 0.88
    letterSpacing: "-0.02em"
    fontVariation: "\"wdth\" 68"
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

The colour layer is deliberately provisional. **Every colour value in this system is a placeholder.** The tournament's designers will supply the real scheme and graphics; until then the build carries a dark ground with an osu!-pink accent purely so the roles are visible. What is fixed is the structure: which role a colour plays, where it may appear, and how much of a screen it may take. No logo or artwork exists, and none is invented; the wordmark is set in type.

Density is high and calm: a 16px body, a dense 4px-based spacing scale, rows of about 60px, and a 1440px content width. Motion is limited to short colour and background transitions on hover and focus.

**Key Characteristics:**
- Condensed heavy Archivo (68% width, 700–900) for every heading, code, score, time, countdown and value; always tabular figures.
- Fira Sans for body and controls; Fira Code only for ids.
- A heavy ink rule opens each page and section; hairline rules separate rows.
- Team red and team blue structure every head-to-head; one brand accent marks live, yours and the primary action.
- All colours are role tokens with placeholder values, swappable in one file.

## Colors

A placeholder palette: a near-black ground and its ink, one brand accent, the two osu! team colours, and status colours. The values are stand-ins; the roles are the system.

**The Placeholder Palette Rule.** No hex value in this file is the brand. Colours are documented and consumed by role only. Components reference role tokens (`--color-*`, `--rule*` in `frontend/src/styles/tokens.css`); a raw colour value in a component stylesheet is a defect.

**Swap procedure.** When the designers' scheme arrives: (1) change the values in `frontend/src/styles/tokens.css` only, keeping every role name; (2) keep `on-brand` legible on `brand` (the placeholder needs dark ink because white on osu! pink fails contrast; a new brand may flip this); (3) keep team red and team blue distinguishable from each other and from `brand` and `danger`; (4) re-run the document pass so this file and its sidecar record the new values. Soft variants (`brand-soft`, `team-*-soft`, `danger-soft`) are the base colour at 12–18% alpha, and several components derive tints with `color-mix()` from the base team and brand colours, so they follow automatically.

### Primary
- **Brand Accent** (placeholder osu! pink): the tournament's one accent. Used only for: live matches ("Canlı" with a dot), anything that is the viewer's own (own team name in a fixture row, own lobby, own bracket row, the "Takımın" strip), the current position (active nav link, the lit stage on the stage track), the one primary action per view, and the wordmark's year. `brand-hover` is its hover step; `on-brand` is the ink on a brand fill.
- **Brand Soft** (brand at 14% alpha): the "yours" background wash (own bracket row, own team strip, active nav link).

### Secondary
- **Team Red** and **Team Blue** (placeholders): osu! multiplayer sides. Team 1 is always red, team 2 always blue, in every head-to-head: both halves of a fixture score, bracket card and list score cells, and the referee page's team columns. `team-*-soft` are their washes.

### Tertiary
- **Status** (placeholders): `success`, `warning`, `danger`, `info`, each with a soft wash where used as a badge or alert ground. `now-line` is the calendar's current-time line. `discord` is Discord's own button colour, used only on the Discord link action. Status colours carry state, never decoration.

### Neutral
- **Ground** (darkest): the page. **Ground Sunken**: input wells. **Surface** and **Surface Raised**: admin tool panels, cards on the profile and Discord pages, popovers, bracket board cards. **Hover Wash**: translucent white over any row on hover.
- **Hairline** and **Hairline Strong**: row rules and control borders.
- **Ink**, **Ink Muted**, **Ink Subtle**: primary text, secondary text (losing team, L codes, stage names, countdowns), and placeholder or empty text.
- **Admin Bar** and **Admin Bar Border**: the tinted top bar on `/admin/*`, so staff always know which side of the site they are on.

**The One Accent Rule.** The brand colour appears only for live, yours, current position and the single primary action. It never decorates a heading, a divider or a category.

**The Sides Rule.** Any time two teams face each other, team 1 is red and team 2 is blue, and both colours appear. A head-to-head without its side colours is incomplete.

## Typography

**Display Font:** Archivo variable (with Arial Narrow, system-ui), set at 68% width (`--font-display-stretch`), weights 700–900.
**Body Font:** Fira Sans (with system-ui, -apple-system, Segoe UI), 400–700.
**Mono Font:** Fira Code (with ui-monospace), for ids and code only.

**Character:** A poster headline face over a sturdy humanist sans. The condensed display face carries everything people scan for or compare; the sans carries reading and operating.

### Hierarchy
- **Display** (900, clamp 56–96px, line-height 0.88): the landing wordmark only (`--text-hero`).
- **Headline** (800, 44px, 34px under 560px wide; line-height 1): page titles, every `h1` (`--text-title`). A count beside the title is set as a muted display figure at 30px, not a pill.
- **Title** (800, 40px): landing section heads ("Sıradaki maçlar", "Son sonuçlar", "Takımın").
- **Section** (800, 24px): every `h2`.
- **Score** (800 winner / 600 loser, 30px; 24px in narrow rows): the score halves of a fixture row.
- **Figure** (700, 18px, tabular): match times, match codes (800), stage names on the track, "Canlı", bracket card scores; countdowns at 14px/600. The `.num` utility applies the display face with tabular figures to any value inline.
- **Body** (400, 16px, line-height 1.5): reading text; the landing lead line caps at 72ch.
- **Control** (600, 14px; 13px small): buttons, nav links, inputs.
- **Label** (600, 12px): badges, stage names under match codes, small meta.
- **Mono** (Fira Code): osu! ids and multiplayer room ids only.

**The Numbers Are Heroes Rule.** Any score, time, countdown, match code, rank or team value is set in the condensed display face with tabular figures. Never set a number someone compares in the body sans.

**The Mono Is For Ids Rule.** Fira Code appears only on identifiers. It is not a "technical" flavour face.

## Layout

Pages sit in a 1440px column (`--content-width`, narrow pages 960px) with 32px top, 16px side and 48px bottom padding. Every page opens with a header: the `h1` on the left, a count beside it, trailing controls pushed right, and a heavy rule under it with 24px below.

Spacing follows a dense 4px-based scale (4, 8, 12, 16, 24, 32, 48). Rows pad 12px vertically and 8px horizontally; sections on the landing are separated by 48px; two-up grids (teams, lobbies) gap 32px by 24px and collapse to one column under 720px (teams) or 1100px (lobbies, which need about 500px per slot).

**The Ruled Sheet Rule.** Content is grouped by rules, not boxes: `--rule-heavy` (3px ink) under page and section headings and over each lobby and team block; `--rule` (1px hairline) between rows. Public pages prefer ruled lists over bordered cards. Admin tool panels may keep boxes.

Responsive behaviour is by container, not page: the fixture list is a size container and its rows rearrange under 720px of their own width. Controls grow to 44px tall on coarse pointers (`--control-height`, `--icon-button-size`). Reduced motion collapses every transition.

## Elevation & Depth

Flat by default. Depth on public pages comes from rules and tonal steps (ground, surface, surface raised), not shadows. Shadows exist only for things that genuinely float: popovers and menus, and the single centred card on the profile and Discord-link pages. Brand-coloured 1px rings and 3px soft-brand focus halos are state indicators, not elevation.

### Shadow Vocabulary
- **Popover** (`0 10px 28px rgba(0,0,0,0.5)`): the notification panel, team popovers, the mini player, map card menus and the bracket board's floating controls.
- **Floating card** (`0 12px 32px rgba(0,0,0,0.35)`): the single centred card on card pages.

**The Flat Sheet Rule.** A row, a section or a team block never takes a shadow. If it needs separation, it gets a rule.

## Shapes

Gently rounded controls (8px) on a square-edged sheet. Rules run edge to edge with no rounding; buttons, inputs and the "yours" strip use 8px; the score block uses 6px; admin panels and centred cards 12px; badges, avatars and round icon buttons are full pills. The live dot is an 8px circle.

## Components

### Buttons
- **Shape:** gently rounded (8px), 36px tall (30px small, 48px large, 44px on touch).
- **Primary:** brand fill with on-brand ink, 600 weight 14px. One per view.
- **Hover / Focus:** brand-hover fill; focus is a 2px focus-colour outline offset 2px.
- **Disabled primary:** grey, not dim brand: surface-raised fill, hairline border, subtle ink, full opacity. Other buttons drop to 50% opacity.
- **Secondary:** transparent with a strong hairline border and muted ink; hover and pressed raise to ink.
- **Danger:** transparent with a danger border at 45% and danger ink; hover adds the danger wash. Kept visually apart from primary on staff screens.
- **Icon-only:** 32px square, transparent, muted ink; hover adds the hover wash. Always carries an accessible label.

### Badges
- **Style:** full pill, 12px/600, hover-wash ground and muted ink; status variants use the status colour at 16% over its own ink.

### Inputs / Fields
- **Style:** sunken well, strong hairline border, 8px radius, 36px tall.
- **Focus:** border turns brand with a 3px brand-soft halo.

### Navigation
- **Top bar:** "112servis" wordmark with the year in brand, then text links in muted 14px ink; hover adds the wash; the active link gets a brand-soft pill with a 2px brand underline. On `/admin/*` the bar turns the admin tint and its links become the admin tabs.

### Fixture Row (signature)
The versus row of the fixture sheet (`components/FixtureRow`), used on the landing and in team match history (the team page and every team card's "Maçlar" overlay); the bracket list repeats its score treatment. A five-column line on a hairline: **time | red team | score | blue team | code and stage**.
- **Time:** display figure 18px, countdown under it in muted 14px; a live match shows "Canlı" in brand with an 8px dot instead.
- **Teams:** team 1 hugs the score from the left (right-aligned), team 2 from the right; the score halves carry the team colours, the names stay in ink; banners sit beside the name and disappear when empty. Losing team in muted 400; the viewer's own team in brand 700; empty slots in subtle ink.
- **Score:** two halves, each tinted in its team colour at 30% and underlined with a 3px bar in that team colour; winner's figure in ink 800, loser's muted 600; unplayed shows a muted "vs" across both halves.
- **Code and stage:** right-aligned; winners-side codes (W58) in ink, losers-side codes (L12) muted; the stage name under it in 12px muted.
- **Narrow (under 720px of its own width):** time and code move to a line above; banners hide; scores drop to 24px.
- **Hover:** hover wash across the row.

### Ruled blocks (lobbies, teams)
Each lobby and each team on public pages is a block under a heavy rule, not a box; the viewer's own block swaps the rule to brand (and the lobby title to brand); an over-limit team swaps it to danger. Rows inside use hairlines.

### Bracket card and list
Board cards are compact surface boxes (the board is an admin-style tool surface) with each team's score cell tinted in its team colour (34%); winner in ink 800, loser muted; the viewer's team row carries the brand-soft wash; the selected card gets a brand ring. The list view uses the fixture score treatment (tinted, underlined halves), brand-soft on own rows and a 3px brand inset on the selected row.

### "Yours" strip
The signed-in player's team and next match on the landing: a brand-soft wash row with 8px radius, its section head ruled in brand instead of ink, the "Takımın" label in brand display type.

### Stage track
The stages in play order as a horizontal strip of 3px top rules: done in muted, upcoming in hairline, current in brand with "Şu an" under its name.

## Do's and Don'ts

### Do:
- **Do** reference role tokens from `frontend/src/styles/tokens.css` for every colour, size, radius and timing.
- **Do** treat every colour value as a placeholder and swap palettes by changing values in `tokens.css` while keeping role names.
- **Do** set every score, time, countdown, code and value in condensed Archivo (68% width) with tabular figures.
- **Do** group public content with a heavy rule under headings and over blocks, and hairlines between rows.
- **Do** show both team colours in every head-to-head: team 1 red, team 2 blue.
- **Do** reuse the fixture row for any list of matches.
- **Do** mark the viewer's own team, lobby or row with brand ink, a brand rule or the brand-soft wash.
- **Do** show disabled primary buttons as grey (surface-raised, subtle ink), not as a dimmed brand fill.

### Don't:
- **Don't** present the placeholder hex values as the brand, or add a logo, artwork or illustration before the designers supply one.
- **Don't** use the brand colour for decoration, headings, dividers or categories; it means live, yours, current or primary.
- **Don't** box public lists in same-size bordered cards or nest boxes inside boxes; use rules.
- **Don't** put shadows on rows, sections or blocks.
- **Don't** set numbers in the body sans or use Fira Code outside ids.
- **Don't** add small uppercase tracked labels above headings or fields; let the heading and the value carry the hierarchy.
