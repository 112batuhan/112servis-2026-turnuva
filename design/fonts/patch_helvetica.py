"""Makes Helvetica Neue LT Pro's Turkish letters render everywhere. Run by design/fonts/build.sh.

1. Ş ş stay Turkish. The Linotype files ship one GSUB lookup that turns Ş ş (cedilla) into Ș ș
   (comma below), the Romanian letters, and hang it from the 'locl' feature of every language they
   list (the default, Turkish, Afrikaans, Dutch and Romanian) plus a feature with a blank tag.
   Browsers apply 'locl' for any page language, so a Turkish page got Romanian letters. This leaves
   the lookup only under Romanian (and Moldavian), where it belongs, and drops the blank-tagged
   feature.

2. Accented letters get their own outlines. Some weights (65 Medium, 35 Thin Italic) build every
   accented letter but Ç Ş (Ö Ü Ğ İ ö ü ğ among them) with `seac`, the Type 1 operator that draws a
   base letter and an accent picked by StandardEncoding code, hidden at the end of a subroutine.
   FreeType draws it, but DirectWrite (Chrome and Edge on Windows) does not draw it right, and
   the letters came out as a smear. Each such glyph is redrawn as the plain outline FreeType would
   have drawn (base plus shifted accent), so no renderer has to know seac."""
import sys
from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.t2CharStringPen import T2CharStringPen

src, dst = sys.argv[1], sys.argv[2]
name = src.replace("\\", "/").rsplit("/", 1)[-1]


def seac_glyphs(path):
    """Glyphs whose charstring ends in seac (endchar with four or more arguments), subroutines
    inlined so one hidden in a subroutine counts too."""
    probe = TTFont(path)
    cff = probe["CFF "].cff
    cff.desubroutinize()
    charstrings = cff.topDictIndex[0].CharStrings
    found = []
    for glyph in probe.getGlyphOrder():
        cs = charstrings[glyph]
        cs.decompile()
        program = cs.program
        if "endchar" not in program:
            continue
        args = 0
        for token in reversed(program[: program.index("endchar")]):
            if isinstance(token, str):
                break
            args += 1
        if args >= 4:
            found.append(glyph)
    return found


def bounds(glyph_set, glyph):
    pen = BoundsPen(glyph_set)
    glyph_set[glyph].draw(pen)
    return pen.bounds


f = TTFont(src)

# ---- 2. Redraw seac glyphs as plain outlines ----
seac = seac_glyphs(src)
if seac:
    cff = f["CFF "].cff
    top = cff.topDictIndex[0]
    assert not hasattr(top, "ROS"), "CID-keyed CFF not handled"
    private = top.Private
    glyph_set = f.getGlyphSet()
    before = {g: bounds(glyph_set, g) for g in seac}
    redrawn = {}
    for glyph in seac:
        width = f["hmtx"][glyph][0]
        # The advance goes in the charstring relative to nominalWidthX, or not at all when it equals
        # defaultWidthX, so DirectWrite (which reads it there) and FreeType (which reads hmtx) agree.
        if width == getattr(private, "defaultWidthX", 0):
            cs_width = None
        else:
            cs_width = width - getattr(private, "nominalWidthX", 0)
        pen = T2CharStringPen(cs_width, glyph_set)
        glyph_set[glyph].draw(pen)  # fontTools decomposes seac into base + accent components
        redrawn[glyph] = pen.getCharString(private=private, globalSubrs=cff.GlobalSubrs)
    for glyph, charstring in redrawn.items():
        top.CharStrings[glyph] = charstring

# ---- 1. Keep Ş ş Turkish ----
COMMA = {"Scedilla": "Scommaaccent", "scedilla": "scommaaccent", "Tcedilla": "Tcommaaccent", "tcedilla": "tcommaaccent"}
KEEP_IN = {"ROM ", "MOL "}
dropped = 0
gsub = f["GSUB"].table if "GSUB" in f else None
if gsub is not None and gsub.FeatureList is not None:
    features = gsub.FeatureList.FeatureRecord
    lookups = gsub.LookupList.Lookup

    def swaps_to_comma(feature_index):
        for li in features[feature_index].Feature.LookupListIndex:
            for st in lookups[li].SubTable:
                st = getattr(st, "ExtSubTable", st)
                mapping = getattr(st, "mapping", {})
                if any(mapping.get(a) == b for a, b in COMMA.items()):
                    return True
        return False

    for sr in gsub.ScriptList.ScriptRecord:
        lang_systems = [("dflt", sr.Script.DefaultLangSys)]
        lang_systems += [(r.LangSysTag, r.LangSys) for r in sr.Script.LangSysRecord]
        for tag, ls in lang_systems:
            if ls is None:
                continue
            keep = []
            for fi in ls.FeatureIndex:
                blank = features[fi].FeatureTag.strip() == ""
                if blank or (tag not in KEEP_IN and swaps_to_comma(fi)):
                    dropped += 1
                    continue
                keep.append(fi)
            ls.FeatureIndex = keep
            ls.FeatureCount = len(keep)

f.save(dst)

# ---- Checks on the saved file ----
out = TTFont(dst)
# No seac left, and every redrawn letter covers exactly the area it did before.
assert not seac_glyphs(dst), f"{name}: seac glyphs left"
if seac:
    out_set = out.getGlyphSet()
    moved = [g for g in seac if bounds(out_set, g) != before[g]]
    assert not moved, f"{name}: redrawn glyphs changed shape: {moved}"
# Every renderer sees the same advance.
out_set = out.getGlyphSet()
bad = [g for g in out.getGlyphOrder() if out_set[g].width != out["hmtx"][g][0]]
assert not bad, f"{name}: CFF and hmtx advances disagree for {bad}"
# Outside Romanian nothing reaches the comma letters any more.
if "GSUB" in out and out["GSUB"].table.FeatureList is not None:
    table = out["GSUB"].table
    for sr in table.ScriptList.ScriptRecord:
        lang_systems = [("dflt", sr.Script.DefaultLangSys)]
        lang_systems += [(r.LangSysTag, r.LangSys) for r in sr.Script.LangSysRecord]
        for tag, ls in lang_systems:
            if ls is not None and tag not in KEEP_IN:
                for fi in ls.FeatureIndex:
                    for li in table.FeatureList.FeatureRecord[fi].Feature.LookupListIndex:
                        for st in table.LookupList.Lookup[li].SubTable:
                            m = getattr(getattr(st, "ExtSubTable", st), "mapping", {})
                            assert not any(m.get(a) == b for a, b in COMMA.items()), f"{sr.ScriptTag}/{tag} still swaps to comma"
print(f"{name}: redrew {len(seac)} seac glyphs, unhooked {dropped} feature references")
