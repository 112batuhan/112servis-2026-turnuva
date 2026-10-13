import { useEffect } from "react";
import { Navigate, NavLink, Outlet, useLocation } from "react-router-dom";
import { useSignedInUser } from "../../auth/AuthContext";
import { hasRole } from "../../auth/roles";
import { readString, writeString } from "../../lib/storage";
import "./AdminLayout.css";

const USERS_TAB = "/admin/users";
const MAPPOOL_TAB = "/admin/mappool";
const TEAMS_TAB = "/admin/teams";
const HOST_TABS = [USERS_TAB, TEAMS_TAB];

const lastTabKey = (userId: string) => `admin-tab:${userId}`;

const tabClass = ({ isActive }: { isActive: boolean }) => (isActive ? "admin-tab is-active" : "admin-tab");

export function AdminLayout() {
  const user = useSignedInUser();
  const { pathname } = useLocation();

  useEffect(() => {
    if (pathname === MAPPOOL_TAB || HOST_TABS.includes(pathname)) writeString(lastTabKey(user.id), pathname);
  }, [pathname, user.id]);

  return (
    <div className="page">
      <nav className="admin-tabs" aria-label="Admin sections">
        <NavLink to={MAPPOOL_TAB} className={tabClass}>
          Map Pool
        </NavLink>
        {hasRole(user, "host") && (
          <>
            <NavLink to={USERS_TAB} className={tabClass}>
              Users
            </NavLink>
            <NavLink to={TEAMS_TAB} className={tabClass}>
              Teams
            </NavLink>
          </>
        )}
      </nav>
      <Outlet />
    </div>
  );
}

/** /admin: reopen the last tab the user visited, falling back to their role's default. */
export function AdminIndexRedirect() {
  const user = useSignedInUser();
  const isHost = hasRole(user, "host");
  const saved = readString(lastTabKey(user.id));

  let target = isHost ? USERS_TAB : MAPPOOL_TAB;
  if (saved === MAPPOOL_TAB || (saved && HOST_TABS.includes(saved) && isHost)) target = saved;

  return <Navigate to={target} replace />;
}
