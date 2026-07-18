import { Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./AuthContext.jsx";
import { hasRole } from "./roles.js";
import { readAdminTab } from "./adminTab.js";
import NavBar from "./components/NavBar.jsx";
import MiniPlayer from "./components/MiniPlayer.jsx";
import LandingPage from "./pages/LandingPage.jsx";
import ProfilePage from "./pages/ProfilePage.jsx";
import DiscordLoginPage from "./pages/DiscordLoginPage.jsx";
import AdminPage from "./pages/AdminPage.jsx";
import UsersPage from "./pages/UsersPage.jsx";
import MapPoolPage from "./pages/MapPoolPage.jsx";
import PublicPoolPage from "./pages/PublicPoolPage.jsx";

export default function App() {
  const { user, error } = useAuth();

  if (error) {
    return (
      <div className="page">
        <p className="status status-error">{error}</p>
      </div>
    );
  }

  if (user === undefined) {
    return (
      <div className="page">
        <p className="status">Checking session…</p>
      </div>
    );
  }

  return (
    <>
      <NavBar />
      <Routes>
        {/* Public — accessible signed out. */}
        <Route path="/" element={<LandingPage />} />
        <Route path="/mappool" element={<PublicPoolPage />} />
        {/* Signed-in only. */}
        <Route
          path="/profile"
          element={
            <RequireAuth>
              <ProfilePage />
            </RequireAuth>
          }
        />
        <Route
          path="/discord"
          element={
            <RequireAuth>
              <DiscordLoginPage />
            </RequireAuth>
          }
        />
        {/* Admin console — map_pooler+ reach it; the Users tab is host-only. */}
        <Route
          path="/admin"
          element={
            <RequireAuth minRole="map_pooler">
              <AdminPage />
            </RequireAuth>
          }
        >
          <Route index element={<AdminIndex />} />
          <Route
            path="users"
            element={
              <RequireAuth minRole="host" redirect="/admin/mappool">
                <UsersPage />
              </RequireAuth>
            }
          />
          <Route path="mappool" element={<MapPoolPage />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
      <MiniPlayer />
    </>
  );
}

// Sends guests and under-privileged users to `redirect` (the public landing page by
// default). Guests sign in via the "Log in with osu!" button in the nav bar.
function RequireAuth({ children, minRole, redirect = "/" }) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/" replace />;
  if (minRole && !hasRole(user, minRole)) return <Navigate to={redirect} replace />;
  return children;
}

// Admin landing tab: the user's last-visited tab if it's still allowed, otherwise the
// role default (hosts → Users, map poolers → Map Pool).
function AdminIndex() {
  const { user } = useAuth();
  const isHost = hasRole(user, "host");
  const saved = readAdminTab(user);
  // The Users tab is host-only, so ignore a saved Users tab for non-hosts.
  const allowed = saved === "/admin/mappool" || (saved === "/admin/users" && isHost);
  const target = allowed ? saved : isHost ? "/admin/users" : "/admin/mappool";
  return <Navigate to={target} replace />;
}
