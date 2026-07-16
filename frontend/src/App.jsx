import { Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./AuthContext.jsx";
import NavBar from "./components/NavBar.jsx";
import LoginPage from "./pages/LoginPage.jsx";
import LandingPage from "./pages/LandingPage.jsx";
import DiscordLoginPage from "./pages/DiscordLoginPage.jsx";
import AdminPage from "./pages/AdminPage.jsx";

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
          path="/admin"
          element={
            <RequireAuth requireHost>
              <AdminPage />
            </RequireAuth>
          }
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </>
  );
}

// Redirects to the osu! login page when signed out, and home when a non-host
// tries to reach a host-only route.
function RequireAuth({ children, requireHost }) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  if (requireHost && user.role !== "host") return <Navigate to="/" replace />;
  return children;
}
