// osu!standard stat math for applying a category's modifier to a beatmap's nomod
// stats at display time. Star rating is NOT recomputed here (that needs osu!'s
// difficulty calculator), so callers keep showing the nomod SR.

const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

// AR <-> preempt time (ms)
const arToMs = (ar) => (ar > 5 ? 1200 - (ar - 5) * 150 : 1200 + (5 - ar) * 120);
const msToAr = (ms) => (ms < 1200 ? 5 + (1200 - ms) / 150 : 5 - (ms - 1200) / 120);
// OD <-> 300 hit window (ms)
const odToMs = (od) => 79.5 - 6 * od;
const msToOd = (ms) => (79.5 - ms) / 6;

function parseMods(modifier) {
  if (!modifier) return [];
  return modifier.toUpperCase().match(/.{1,2}/g) ?? [];
}

// Returns { bpm, total_length, cs, ar, od, hp } with the modifier applied.
export function applyMods(stats, modifier) {
  let { bpm, total_length, cs, ar, od, hp } = stats;
  const mods = parseMods(modifier);

  if (mods.includes("HR")) {
    cs = clamp(cs * 1.3, 0, 10);
    ar = clamp(ar * 1.4, 0, 10);
    od = clamp(od * 1.4, 0, 10);
    hp = clamp(hp * 1.4, 0, 10);
  } else if (mods.includes("EZ")) {
    cs *= 0.5;
    ar *= 0.5;
    od *= 0.5;
    hp *= 0.5;
  }

  let rate = 1;
  if (mods.includes("DT") || mods.includes("NC")) rate = 1.5;
  else if (mods.includes("HT")) rate = 0.75;

  if (rate !== 1) {
    bpm *= rate;
    total_length = Math.round(total_length / rate);
    ar = clamp(msToAr(arToMs(ar) / rate), 0, 11);
    od = clamp(msToOd(odToMs(od) / rate), 0, 11);
  }

  return { bpm, total_length, cs, ar, od, hp };
}

export function formatLength(seconds) {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
