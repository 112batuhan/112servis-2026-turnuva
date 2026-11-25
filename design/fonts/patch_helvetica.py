"""Keeps Helvetica Neue LT Pro's Ş ş Turkish. Run by design/fonts/build.sh.

The Linotype files ship one GSUB lookup that turns Ş ş (cedilla) into Ș ș (comma below), the
Romanian letters, and hang it from the 'locl' feature of every language they list (the default,
Turkish, Afrikaans, Dutch and Romanian) plus a feature with a blank tag. Browsers apply 'locl' for
any page language, so a Turkish page got Romanian letters. This leaves the lookup only under
Romanian (and Moldavian), where it belongs, and drops the blank-tagged feature."""
import sys
from fontTools.ttLib import TTFont

src, dst = sys.argv[1], sys.argv[2]
f = TTFont(src)
COMMA = {"Scedilla": "Scommaaccent", "scedilla": "scommaaccent", "Tcedilla": "Tcommaaccent", "tcedilla": "tcommaaccent"}
KEEP_IN = {"ROM ", "MOL "}

gsub = f["GSUB"].table if "GSUB" in f else None
if gsub is None or gsub.FeatureList is None:
    f.save(dst)
    sys.exit()
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


dropped = 0
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

# Check: outside Romanian nothing reaches the comma letters any more.
out = TTFont(dst)["GSUB"].table
for sr in out.ScriptList.ScriptRecord:
    lang_systems = [("dflt", sr.Script.DefaultLangSys)]
    lang_systems += [(r.LangSysTag, r.LangSys) for r in sr.Script.LangSysRecord]
    for tag, ls in lang_systems:
        if ls is not None and tag not in KEEP_IN:
            for fi in ls.FeatureIndex:
                lk = [out.LookupList.Lookup[i] for i in out.FeatureList.FeatureRecord[fi].Feature.LookupListIndex]
                for lookup in lk:
                    for st in lookup.SubTable:
                        m = getattr(getattr(st, "ExtSubTable", st), "mapping", {})
                        assert not any(m.get(a) == b for a, b in COMMA.items()), f"{sr.ScriptTag}/{tag} still swaps to comma"
print(f"{src.rsplit('/', 1)[-1]}: unhooked {dropped} feature references")
