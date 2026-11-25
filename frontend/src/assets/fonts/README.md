# Fonts

Declared in `src/styles/fonts.css`, used through the `--font-*` tokens in `src/styles/tokens.css`.

| Files                    | Face                                                                          | Source                             |
| ------------------------ | ----------------------------------------------------------------------------- | ---------------------------------- |
| `hecate-display-*.woff2` | Hecate Display Regular and Italic (Roomspace Creative Lab)                    | `design/Hecate Display.zip`        |
| `helvetica-neue-*.woff2` | Helvetica Neue LT Pro (Linotype): normal, condensed (`-cn`), extended (`-ex`) | `design/helvetica-neue-lt-pro.zip` |

How they were made (`design/fonts/build.sh`, rerun it to rebuild them):

- Hecate Display has no Ğ ğ İ Ş ş. `design/fonts/patch_hecate.py` builds them from the font's own G, g, I, S, s
  and its spacing breve, dot accent and cedilla, lifted and centred the way the font sets its own Â/â and
  Ç/ç (the italic is un-slanted to centre, then re-slanted), and gives them their base letter's kerning.
  Each new glyph's advance is written to both `hmtx` (what FreeType on Linux and Android reads) and its
  CFF charstring, relative to `nominalWidthX` (what DirectWrite on Windows reads); the script checks
  the two agree.
- Helvetica Neue LT Pro's own files turn Ş ş into the Romanian Ș ș (comma below) under the `locl`
  feature of every language, Turkish included. `design/fonts/patch_helvetica.py` leaves that swap
  under Romanian only.
- Every face is subset to Latin-1, Latin Extended-A and general punctuation with all its OpenType
  features and name records kept, and saved as WOFF2.

Only the Helvetica weights the CSS asks for are shipped. A face downloads only when a page renders text in it.
