// Remembers each user's last-visited admin tab (in localStorage) so they land back on it
// instead of the role default — e.g. a host who works in the map pool doesn't have to
// click away from the Users tab every visit.
const ADMIN_TABS = ["/admin/users", "/admin/mappool"];

const key = (user) => `admin-tab:${user?.id}`;

export function saveAdminTab(user, path) {
  if (!ADMIN_TABS.includes(path)) return;
  try {
    localStorage.setItem(key(user), path);
  } catch {
    // ignore write failures (private mode / quota)
  }
}

export function readAdminTab(user) {
  try {
    const path = localStorage.getItem(key(user));
    return ADMIN_TABS.includes(path) ? path : null;
  } catch {
    return null;
  }
}
