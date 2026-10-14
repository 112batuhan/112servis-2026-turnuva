import { useEffect } from "react";
import { Navigate, NavLink, Outlet, useLocation } from "react-router-dom";
import type { Role } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { hasRole } from "../../auth/roles";
import { IrcProvider } from "../../irc/IrcContext";
import { readString, writeString } from "../../lib/storage";
import "./AdminLayout.css";

// In tab order. The first tab a user can open is their default, except that hosts
// start on Users.
const TABS: { path: string; label: string; minRole: Role }[] = [
  { path: "/admin/stages", label: "Stage'ler", minRole: "host" },
  { path: "/admin/mappool", label: "Mappool", minRole: "map_pooler" },
  { path: "/admin/qualifiers", label: "Elemeler", minRole: "referee" },
  { path: "/admin/bracket", label: "Bracket", minRole: "streamer" },
  { path: "/admin/referee", label: "Hakem", minRole: "referee" },
  { path: "/admin/users", label: "Kullanıcılar", minRole: "host" },
  { path: "/admin/teams", label: "Takımlar", minRole: "host" },
];
const USERS_TAB = "/admin/users";

const lastTabKey = (userId: string) => `admin-tab:${userId}`;

const tabClass = ({ isActive }: { isActive: boolean }) => (isActive ? "admin-tab is-active" : "admin-tab");

export function AdminLayout() {
  const user = useSignedInUser();
  const { pathname } = useLocation();
  const tabs = TABS.filter((tab) => hasRole(user, tab.minRole));

  useEffect(() => {
    if (TABS.some((tab) => tab.path === pathname)) writeString(lastTabKey(user.id), pathname);
  }, [pathname, user.id]);

  return (
    <div className="page">
      <nav className="admin-tabs" aria-label="Yönetim bölümleri">
        {tabs.map((tab) => (
          <NavLink key={tab.path} to={tab.path} className={tabClass}>
            {tab.label}
          </NavLink>
        ))}
      </nav>
      {/* Here rather than on the referee page, so the Bancho connection outlives tab switches. */}
      <IrcProvider>
        <Outlet />
      </IrcProvider>
    </div>
  );
}

/** /admin: reopen the last tab the user visited, falling back to their role's default. */
export function AdminIndexRedirect() {
  const user = useSignedInUser();
  const tabs = TABS.filter((tab) => hasRole(user, tab.minRole)).map((tab) => tab.path);
  const saved = readString(lastTabKey(user.id));

  let target = hasRole(user, "host") ? USERS_TAB : tabs[0];
  if (saved && tabs.includes(saved)) target = saved;

  return <Navigate to={target} replace />;
}
