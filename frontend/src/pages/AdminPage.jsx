import { useEffect } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { useAuth } from "../AuthContext.jsx";
import { hasRole } from "../roles.js";
import { saveAdminTab } from "../adminTab.js";
import "./AdminPage.css";

// Admin console layout: a tab bar over nested routes. The Users tab is host-only;
// the Map Pool tab is map_pooler+. The active tab renders in the <Outlet />.
export default function AdminPage() {
  const { user } = useAuth();
  const isHost = hasRole(user, "host");

  // Remember the active tab so the next visit to /admin lands back on it.
  const { pathname } = useLocation();
  useEffect(() => {
    saveAdminTab(user, pathname);
  }, [pathname, user]);

  const tabClass = ({ isActive }) => (isActive ? "admin-tab admin-tab-active" : "admin-tab");

  return (
    <div className="content admin">
      <div className="admin-tabs">
        {isHost && (
          <NavLink to="/admin/users" className={tabClass}>
            Users
          </NavLink>
        )}
        <NavLink to="/admin/mappool" className={tabClass}>
          Map Pool
        </NavLink>
      </div>
      <Outlet />
    </div>
  );
}
