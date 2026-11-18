import { Suspense, useEffect } from "react";
import { Navigate, Outlet, useLocation } from "react-router-dom";
import type { User } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { canAccess, hasRole } from "../../auth/roles";
import { readString, writeString } from "../../lib/storage";
import { ADMIN_TABS } from "../../lib/adminTabs";

// The first tab a user can open is their default, except that hosts start on Users.
const USERS_TAB = "/admin/users";
const REFEREE_PAGE = "/admin/referee/";

const lastPageKey = (userId: string) => `admin-page:${userId}`;

/** Whether the user can still open a remembered page, so a lost role doesn't redirect in a loop. */
function canOpen(user: User, page: string) {
  const path = page.split(/[?#]/)[0];
  if (path.startsWith(REFEREE_PAGE)) return hasRole(user, "referee");
  return ADMIN_TABS.some((tab) => tab.path === path && canAccess(user, tab.access));
}

export function AdminLayout() {
  const user = useSignedInUser();
  const { pathname, search, hash } = useLocation();

  // Every admin page, with its query, so "Yönetim" comes back to it after the public pages.
  useEffect(() => {
    const page = pathname + search + hash;
    if (canOpen(user, page)) writeString(lastPageKey(user.id), page);
  }, [pathname, search, hash, user]);

  // The tabs are in the top bar (`NavBar`).
  return (
    <div className="page admin-page">
      {/* Admin pages load when first opened; the tabs stay meanwhile. */}
      <Suspense fallback={<p className="status page-loading">Yükleniyor…</p>}>
        <Outlet />
      </Suspense>
    </div>
  );
}

/** /admin: reopen the last admin page the user was on, falling back to their role's default tab. */
export function AdminIndexRedirect() {
  const user = useSignedInUser();
  const saved = readString(lastPageKey(user.id));

  let target = hasRole(user, "host")
    ? USERS_TAB
    : ADMIN_TABS.find((tab) => canAccess(user, tab.access))!.path;
  if (saved && canOpen(user, saved)) target = saved;

  return <Navigate to={target} replace />;
}
