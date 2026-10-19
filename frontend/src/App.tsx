import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./auth/AuthContext";
import { RequireAuth } from "./auth/RequireAuth";
import { MiniPlayer } from "./components/MiniPlayer";
import { NavBar } from "./components/NavBar";
import { AdminIndexRedirect, AdminLayout } from "./pages/admin/AdminLayout";
import { BracketAdminPage } from "./pages/admin/bracket/BracketAdminPage";
import { BanchoAdminPage } from "./pages/admin/BanchoAdminPage";
import { DiscordAdminPage } from "./pages/admin/DiscordAdminPage";
import { MapPoolEditorPage } from "./pages/admin/mappool/MapPoolEditorPage";
import { QualifiersAdminPage } from "./pages/admin/QualifiersAdminPage";
import { RefereeRoomPage } from "./pages/admin/referee/RefereeRoomPage";
import { RulesAdminPage } from "./pages/admin/RulesAdminPage";
import { StagesAdminPage } from "./pages/admin/StagesAdminPage";
import { TeamsAdminPage } from "./pages/admin/TeamsAdminPage";
import { UsersPage } from "./pages/admin/UsersPage";
import { ArgPage } from "./pages/ArgPage";
import { BracketPage } from "./pages/BracketPage";
import { LandingPage } from "./pages/LandingPage";
import { LinkDiscordPage } from "./pages/LinkDiscordPage";
import { MatchPage } from "./pages/match/MatchPage";
import { PickemPage } from "./pages/pickem/PickemPage";
import { ProfilePage } from "./pages/ProfilePage";
import { PublicPoolPage } from "./pages/PublicPoolPage";
import { QualifiersPage } from "./pages/QualifiersPage";
import { RulesPage } from "./pages/RulesPage";
import { StaffPage } from "./pages/StaffPage";
import { TeamPage } from "./pages/TeamPage";
import { TeamsPage } from "./pages/TeamsPage";
import { usePlayer } from "./player/PlayerContext";

const DevSwitcher = __DEV_LOGIN__
  ? lazy(() => import("./components/DevSwitcher").then((m) => ({ default: m.DevSwitcher })))
  : null;

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
          Tekrar dene
        </button>
      </main>
    );
  }

  if (user === undefined) {
    return (
      <main className="fullscreen-status">
        <p className="status">Oturum kontrol ediliyor…</p>
      </main>
    );
  }

  return (
    <div className={`app-shell ${track ? "has-player" : ""}`}>
      <NavBar />
      <main>
        <Routes>
          <Route path="/" element={<LandingPage />} />
          <Route path="/rules" element={<RulesPage />} />
          <Route path="/mappool" element={<PublicPoolPage />} />
          <Route path="/teams" element={<TeamsPage />} />
          <Route path="/qualifiers" element={<QualifiersPage />} />
          <Route path="/bracket" element={<BracketPage />} />
          <Route path="/match/:matchId" element={<MatchPage />} />
          <Route path="/pickem" element={<PickemPage />} />
          <Route path="/staff" element={<StaffPage />} />
          <Route path="/kupa" element={<ArgPage />} />

          <Route path="/profile" element={<RequireAuth><ProfilePage /></RequireAuth>} />
          <Route path="/discord" element={<RequireAuth><LinkDiscordPage /></RequireAuth>} />
          <Route path="/team" element={<RequireAuth><TeamPage /></RequireAuth>} />

          <Route path="/admin" element={<RequireAuth access="staff"><AdminLayout /></RequireAuth>}>
            <Route index element={<AdminIndexRedirect />} />
            <Route
              path="stages"
              element={<RequireAuth access="host" redirect="/admin"><StagesAdminPage /></RequireAuth>}
            />
            <Route
              path="mappool"
              element={<RequireAuth access="pool" redirect="/admin"><MapPoolEditorPage /></RequireAuth>}
            />
            <Route
              path="qualifiers"
              element={<RequireAuth access="referee" redirect="/admin"><QualifiersAdminPage /></RequireAuth>}
            />
            <Route
              path="bracket"
              element={<RequireAuth access="matches" redirect="/admin"><BracketAdminPage /></RequireAuth>}
            />
            <Route
              path="referee/:matchId"
              element={<RequireAuth access="referee" redirect="/admin"><RefereeRoomPage /></RequireAuth>}
            />
            <Route
              path="users"
              element={<RequireAuth access="host" redirect="/admin"><UsersPage /></RequireAuth>}
            />
            <Route
              path="teams"
              element={<RequireAuth access="host" redirect="/admin"><TeamsAdminPage /></RequireAuth>}
            />
            <Route
              path="rules"
              element={<RequireAuth access="host" redirect="/admin"><RulesAdminPage /></RequireAuth>}
            />
            <Route
              path="discord"
              element={<RequireAuth access="host" redirect="/admin"><DiscordAdminPage /></RequireAuth>}
            />
            <Route
              path="bancho"
              element={<RequireAuth access="host" redirect="/admin"><BanchoAdminPage /></RequireAuth>}
            />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
      <MiniPlayer />
      {DevSwitcher && (
        <Suspense fallback={null}>
          <DevSwitcher />
        </Suspense>
      )}
    </div>
  );
}
