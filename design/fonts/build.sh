#!/bin/sh
# Rebuilds frontend/src/assets/fonts from the two zips in design/ (see that folder's README).
# Needs fontTools with brotli: `uv venv /tmp/ft && uv pip install --python /tmp/ft/bin/python fonttools brotli`,
# then `PY=/tmp/ft/bin/python design/fonts/build.sh` from the repo root.
set -eu
PY=${PY:-python3}
OUT=frontend/src/assets/fonts
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
unzip -q "design/Hecate Display.zip" -d "$WORK/hecate"
unzip -q design/helvetica-neue-lt-pro.zip -d "$WORK/helv"
H="$WORK/helv/helvetica-neue-lt-pro"

"$PY" -I design/fonts/patch_hecate.py "$WORK/hecate/Hecate Display-Regular.otf" "$WORK/hecate-regular.otf"
"$PY" -I design/fonts/patch_hecate.py "$WORK/hecate/Hecate Display-Italic.otf" "$WORK/hecate-italic.otf"

U="U+0020-007E,U+00A0-017F,U+0192,U+02C6-02DD,U+2000-206F,U+20AC,U+2122,U+2212,U+FB01-FB02"
sub() {
  "$PY" -I -m fontTools.subset "$1" --unicodes="$U" --layout-features='*' --name-IDs='*' \
    --name-languages='*' --flavor=woff2 --output-file="$OUT/$2.woff2"
}
sub "$WORK/hecate-regular.otf" hecate-display-regular
sub "$WORK/hecate-italic.otf" hecate-display-italic
for pair in Roman:roman It:italic Md:medium Bd:bold BdIt:bold-italic \
  MdCn:medium-cn BdCn:bold-cn HvCn:heavy-cn LtEx:light-ex MdEx:medium-ex; do
  "$PY" -I design/fonts/patch_helvetica.py "$H/HelveticaNeueLTPro${pair%%:*}.otf" "$WORK/helv-${pair##*:}.otf"
  sub "$WORK/helv-${pair##*:}.otf" "helvetica-neue-${pair##*:}"
done
