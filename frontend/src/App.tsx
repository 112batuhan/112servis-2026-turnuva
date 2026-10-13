import { Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./auth/AuthContext";
import { RequireAuth } from "./auth/RequireAuth";
import { MiniPlayer } from "./components/MiniPlayer";
import { NavBar } from "./components/NavBar";
import { AdminIndexRedirect, AdminLayout } from "./pages/admin/AdminLayout";
import { MapPoolEditorPage } from "./pages/admin/mappool/MapPoolEditorPage";
import { TeamsAdminPage } from "./pages/admin/TeamsAdminPage";
import { UsersPage } from "./pages/admin/UsersPage";
import { LandingPage } from "./pages/LandingPage";
import { LinkDiscordPage } from "./pages/LinkDiscordPage";
import { ProfilePage } from "./pages/ProfilePage";
import { PublicPoolPage } from "./pages/PublicPoolPage";
import { TeamPage } from "./pages/TeamPage";
import { TeamsPage } from "./pages/TeamsPage";
import { usePlayer } from "./player/PlayerContext";

export function App() {
  const { user, error, refresh } = useAuth();
  const { track } = usePlayer();

  if (error) {
    return (
      <main className="fullscreen-status">
        <p className="status" role="alert">
          {error}
        </p>
        <button type="button" className="btn btn-secondary" onClick={refresh}>
          Try again
        </button>
      </main>
    );
  }

  if (user === undefined) {
    return (
      <main className="fullscreen-status">
        <p className="status">Checking session…</p>
      </main>
    );
  }

  return (
    <div className={`app-shell ${track ? "has-player" : ""}`}>
      <NavBar />
      <main>
        <Routes>
          <Route path="/" element={<LandingPage />} />
          <Route path="/mappool" element={<PublicPoolPage />} />
          <Route path="/teams" element={<TeamsPage />} />

          <Route path="/profile" element={<RequireAuth><ProfilePage /></RequireAuth>} />
          <Route path="/discord" element={<RequireAuth><LinkDiscordPage /></RequireAuth>} />
          <Route path="/team" element={<RequireAuth><TeamPage /></RequireAuth>} />

          <Route path="/admin" element={<RequireAuth minRole="map_pooler"><AdminLayout /></RequireAuth>}>
            <Route index element={<AdminIndexRedirect />} />
            <Route path="mappool" element={<MapPoolEditorPage />} />
            <Route
              path="users"
              element={<RequireAuth minRole="host" redirect="/admin/mappool"><UsersPage /></RequireAuth>}
            />
            <Route
              path="teams"
              element={<RequireAuth minRole="host" redirect="/admin/mappool"><TeamsAdminPage /></RequireAuth>}
            />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
      <MiniPlayer />
    </div>
  );
}
