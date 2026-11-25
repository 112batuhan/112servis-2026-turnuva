"""Adds the Turkish letters Hecate Display lacks (Ğ ğ İ Ş ş), built from its own base letters and
spacing accents and placed the way the font places its own Â/â and Ç/ç. Run by design/fonts/build.sh."""
import math, sys
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

src, dst = sys.argv[1], sys.argv[2]
f = TTFont(src)
gs = f.getGlyphSet()
slant = -math.tan(math.radians(f["post"].italicAngle))  # x grows by slant*y

def points(name):
    rp = DecomposingRecordingPen(gs); gs[name].draw(rp)
    return [pt for _, args in rp.value for pt in args]

def ucenter(pts):  # horizontal centre in un-slanted space
    xs = [x - y * slant for x, y in pts]
    return (min(xs) + max(xs)) / 2

def ymax(pts): return max(y for _, y in pts)
def ymin(pts): return min(y for _, y in pts)

# Vertical lifts, from the font's own circumflex placement.
circ = points("circumflex")
cap_lift = ymax([p for p in points("Acircumflex") if p[1] > 720]) - ymax(circ)
lc_lift = ymax([p for p in points("acircumflex") if p[1] > 530]) - ymax(circ)
# Horizontal offsets of an accent from its base's centre, as the font sets them.
cap_dx = ucenter([p for p in points("Acircumflex") if p[1] > 720]) - ucenter(points("A"))
ced_pts = [p for p in points("Ccedilla") if p[1] < -15]
ced_lift = ymin(ced_pts) - ymin(points("cedilla"))
ced_dx_C = ucenter(ced_pts) - ucenter(points("C"))
ced_dx_c = ucenter([p for p in points("ccedilla") if p[1] < -15]) - ucenter(points("c"))
print(f"slant={slant:.3f} cap_lift={cap_lift} lc_lift={lc_lift} cap_dx={cap_dx:.1f} ced_lift={ced_lift} ced_dx C={ced_dx_C:.1f} c={ced_dx_c:.1f}")

def accent_shift(base, accent, lift, dx):
    """Translation that puts `accent` `lift` units up and centred (in un-slanted space) on `base` + dx."""
    acc = [(x, y + lift) for x, y in points(accent)]
    tx = ucenter(points(base)) + dx - ucenter(acc)
    return tx, lift

# For ğ and Ğ centre the breve on the base; for İ the dot; cedillas as in Ç/ç.
plan = {
    0x011E: ("Gbreve", "G", "breve", cap_lift, cap_dx),
    0x011F: ("gbreve", "g", "breve", lc_lift, 0.0),
    0x0130: ("Idotaccent", "I", "dotaccent", cap_lift, cap_dx),
    0x015E: ("Scedilla", "S", "cedilla", ced_lift, ced_dx_C),
    0x015F: ("scedilla", "s", "cedilla", ced_lift, ced_dx_c),
}

cff = f["CFF "].cff
top = cff.topDictIndex[0]
assert not hasattr(top, "ROS"), "CID-keyed CFF not handled"
cs = top.CharStrings
order = list(f.getGlyphOrder())
cmap_tables = [t for t in f["cmap"].tables if t.isUnicode()]
for cp, (name, base, accent, lift, dx) in plan.items():
    if name in order:
        continue
    tx, ty = accent_shift(base, accent, lift, dx)
    width = gs[base].width
    # A CFF charstring stores its advance as an offset from the Private dict's nominalWidthX, or
    # none at all when it equals defaultWidthX. FreeType (Linux) reads advances from hmtx and never
    # sees this one, but DirectWrite (Windows) reads it, so a raw width there widens the letter by
    # nominalWidthX.
    priv = top.Private
    if width == getattr(priv, "defaultWidthX", 0):
        cs_width = None
    else:
        cs_width = width - getattr(priv, "nominalWidthX", 0)
    pen = T2CharStringPen(cs_width, gs)
    gs[base].draw(pen)
    gs[accent].draw(TransformPen(pen, (1, 0, 0, 1, round(tx), round(ty))))
    charstring = pen.getCharString(private=top.Private, globalSubrs=cff.GlobalSubrs)
    cs.charStringsIndex.append(charstring)
    cs.charStrings[name] = len(cs.charStringsIndex) - 1
    order.append(name)
    f.setGlyphOrder(order)
    top.charset = order
    f["hmtx"].metrics[name] = (width, 0)
    gs = f.getGlyphSet()
    bp = BoundsPen(gs); gs[name].draw(bp)
    f["hmtx"].metrics[name] = (width, round(bp.bounds[0]))
    for t in cmap_tables:
        if cp <= 0xFFFF or t.format == 12:
            t.cmap[cp] = name
    print(f"added {name} = {base} + {accent} at ({tx:.0f}, {ty:.0f})")

# Let the new letters kern like their bases: copy each base's class membership in GPOS kern.
gpos = f["GPOS"].table if "GPOS" in f else None
if gpos:
    bases = {name: base for _, (name, base, *_r) in plan.items()}
    for lookup in gpos.LookupList.Lookup:
        for st in lookup.SubTable:
            st = getattr(st, "ExtSubTable", st)
            if getattr(st, "Format", None) == 1 and hasattr(st, "PairSet"):
                cov = st.Coverage.glyphs
                for new, base in bases.items():
                    if base in cov and new not in cov:
                        i = cov.index(base)
                        cov.append(new); st.PairSet.append(st.PairSet[i])
                # second glyphs inside pair sets
                for ps in st.PairSet:
                    for rec in list(ps.PairValueRecord):
                        if rec.SecondGlyph in bases.values():
                            for new, base in bases.items():
                                if base == rec.SecondGlyph and not any(r.SecondGlyph == new for r in ps.PairValueRecord):
                                    import copy
                                    r2 = copy.deepcopy(rec); r2.SecondGlyph = new
                                    ps.PairValueRecord.append(r2)
                    ps.PairValueCount = len(ps.PairValueRecord)
                st.PairSetCount = len(st.PairSet)
            elif getattr(st, "Format", None) == 2:
                cov = st.Coverage.glyphs
                for new, base in bases.items():
                    if base in cov and new not in cov:
                        cov.append(new)
                    for cd in (st.ClassDef1, st.ClassDef2):
                        if base in cd.classDefs and new not in cd.classDefs:
                            cd.classDefs[new] = cd.classDefs[base]
f.save(dst)

# Every renderer must see the same advance: check the saved CFF widths against hmtx.
out = TTFont(dst)
out_gs = out.getGlyphSet()
bad = [n for n in out.getGlyphOrder() if out_gs[n].width != out["hmtx"][n][0]]
assert not bad, f"CFF and hmtx advances disagree for {bad}"
print("saved", dst)
