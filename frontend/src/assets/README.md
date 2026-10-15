# Assets

Everything here comes from the designers' first banner, `design/bannertest.png` (1346×755, a test
version: "112'nin Son Kış Turnuvası"). When the final banner arrives, re-export these from it the
same way and keep the file names.

| File | What it is | How it was made |
|---|---|---|
| `banner-1316.webp`, `banner-760.webp` | The landing page's hero from 1200px wide | The banner with its grey frame and its lettering cropped off (`-crop 1316x590+14+10`; the line "112'nin Son Kış Turnuvası" is set as live type instead), WebP at q82 / resized to 760 wide at q80 |
| `crest.webp` | The hero under 1200px: just the trophy and the crystal, so the matches stay in the first viewport | `-crop 640x580+353+40`, WebP at q82 |
| `mark.png` | The 112 crystal (an ice crystal drawn with the digits 1, 1 and 2), used as a CSS mask by `.brand-mark` | Cut from the banner (`-crop 230x240+568+222`), its luminance levelled (55%–92%) into an alpha matte over white, trimmed; 199×220 |
| `gear-tl.svg`, `gear-br.svg` | The faint gear linework in the page's corners, used as masks in `styles/base.css` | Drawn in code after the banner's corner cogs (not traced): stroke-only gears and rings |

`public/favicon.png` (64px) and `public/apple-touch-icon.png` (180px) are `mark.png` in the frost ink
on the slate ground.
