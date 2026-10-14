// localStorage can throw (private mode, quota, blocked site data), so every access goes
// through here and failures fall back silently. Only use it for per-browser conveniences.

export function readString(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeString(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // ignore
  }
}

export function readJson<T>(key: string, fallback: T): T {
  const raw = readString(key);
  if (raw === null) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function writeJson(key: string, value: unknown) {
  writeString(key, JSON.stringify(value));
}

export function removeString(key: string) {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}
