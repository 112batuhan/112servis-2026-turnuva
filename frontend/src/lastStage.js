// Remembers the last stage a user had open in the map pool editor (per user, in
// localStorage) so returning to the page reopens it instead of the first stage.
const key = (user) => `mappool-stage:${user?.id}`;

export function saveLastStage(user, stageId) {
  try {
    localStorage.setItem(key(user), stageId);
  } catch {
    // ignore write failures (private mode / quota)
  }
}

export function readLastStage(user) {
  try {
    return localStorage.getItem(key(user));
  } catch {
    return null;
  }
}
