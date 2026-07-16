import { Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./AuthContext.jsx";
import { hasRole } from "./roles.js";
import NavBar from "./components/NavBar.jsx";
import LoginPage from "./pages/LoginPage.jsx";
import LandingPage from "./pages/LandingPage.jsx";
import DiscordLoginPage from "./pages/DiscordLoginPage.jsx";
import AdminPage from "./pages/AdminPage.jsx";
import MapPoolPage from "./pages/MapPoolPage.jsx";

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
      {user && <NavBar />}
      <Routes>
        <Route path="/login" element={user ? <Navigate to="/" replace /> : <LoginPage />} />
        <Route
          path="/"
          element={
            <RequireAuth>
              <LandingPage />
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
        <Route
          path="/mappool"
          element={
            <RequireAuth minRole="map_pooler">
              <MapPoolPage />
            </RequireAuth>
          }
        />
        <Route
          path="/admin"
          element={
            <RequireAuth minRole="host">
              <AdminPage />
            </RequireAuth>
          }
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </>
  );
}

// Redirects to the osu! login page when signed out, and home when the user's role
// is below `minRole` for a gated route.
function RequireAuth({ children, minRole }) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  if (minRole && !hasRole(user, minRole)) return <Navigate to="/" replace />;
  return children;
}
