# Assets

Everything here except `mark.png` comes from the designers' first banner, `design/bannertest.png` (1346×755, a test
version: "112'nin Son Kış Turnuvası"). When the final banner arrives, re-export these from it the
same way and keep the file names.

| File | What it is | How it was made |
|---|---|---|
| `banner-1316.webp`, `banner-760.webp` | The landing page's hero from 1200px wide | The banner with its grey frame and its lettering cropped off (`-crop 1316x590+14+10`; the line "112'nin Son Kış Turnuvası" is set as live type instead), WebP at q82 / resized to 760 wide at q80 |
| `crest.webp` | The hero under 1200px: just the trophy and the crystal, so the matches stay in the first viewport | `-crop 640x580+353+40`, WebP at q82 |
| `mark.png` | The tilted trophy (the banner's cup, drawn crisply by the designers), used as a CSS mask by `.brand-mark` | Cut from `design/trophy.png` (white on purple, 500×500): its green channel levelled (20%–96%) into an alpha matte over white, trimmed; 263×278 |
| `gear-tl.svg`, `gear-br.svg` | The faint gear linework in the page's corners, used as masks in `styles/base.css` | Drawn in code after the banner's corner cogs (not traced): stroke-only gears and rings |

`public/favicon.png` (64px) and `public/apple-touch-icon.png` (180px) are `mark.png` in the frost ink
on the slate ground (50px tall on a 64px rounded square with a 12px radius; 128px tall on a full 180px
square). The tilt puts the trophy's weight up and to the left of its bounding box, so it isn't
centred by that box: the point 70% of the way from the box's centre to the matte's centroid
(119.5, 127.8 in `mark.png`'s pixels) sits at the middle of the icon. The full centroid crowds
the right handle against the edge.
