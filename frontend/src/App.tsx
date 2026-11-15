import { lazy, Suspense, type ComponentType } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { useAuth } from "./auth/AuthContext";
import { RequireAuth } from "./auth/RequireAuth";
import { AuthNotice } from "./components/AuthNotice";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { MiniPlayer } from "./components/MiniPlayer";
import { NavBar } from "./components/NavBar";
import { AdminIndexRedirect, AdminLayout } from "./pages/admin/AdminLayout";
import { ArgPage } from "./pages/public/ArgPage";
import { BracketPage } from "./pages/public/BracketPage";
import { LandingPage } from "./pages/public/LandingPage";
import { LinkDiscordPage } from "./pages/public/LinkDiscordPage";
import { ProfilePage } from "./pages/public/ProfilePage";
import { PublicPoolPage } from "./pages/public/PublicPoolPage";
import { QualifiersPage } from "./pages/public/QualifiersPage";
import { RulesPage } from "./pages/public/RulesPage";
import { StaffPage } from "./pages/public/StaffPage";
import { TeamPage } from "./pages/public/TeamPage";
import { TeamsPage } from "./pages/public/TeamsPage";
import { usePlayer } from "./player/PlayerContext";

// Pages most visitors never open (the staff's, and the heavier public ones) load when
// they're first opened.
const page = <K extends string>(load: () => Promise<Record<K, ComponentType>>, name: K) =>
  lazy(() => load().then((m) => ({ default: m[name] })));
const BracketAdminPage = page(
  () => import("./pages/admin/bracket/BracketAdminPage"),
  "BracketAdminPage",
);
const BanchoAdminPage = page(() => import("./pages/admin/BanchoAdminPage"), "BanchoAdminPage");
const DiscordAdminPage = page(() => import("./pages/admin/DiscordAdminPage"), "DiscordAdminPage");
const MapPoolEditorPage = page(
  () => import("./pages/admin/mappool/MapPoolEditorPage"),
  "MapPoolEditorPage",
);
const QualifiersAdminPage = page(
  () => import("./pages/admin/QualifiersAdminPage"),
  "QualifiersAdminPage",
);
const RefereeRoomPage = page(
  () => import("./pages/admin/referee/RefereeRoomPage"),
  "RefereeRoomPage",
);
const SeedingAdminPage = page(() => import("./pages/admin/SeedingAdminPage"), "SeedingAdminPage");
const RulesAdminPage = page(() => import("./pages/admin/RulesAdminPage"), "RulesAdminPage");
const StagesAdminPage = page(() => import("./pages/admin/StagesAdminPage"), "StagesAdminPage");
const TeamsAdminPage = page(() => import("./pages/admin/TeamsAdminPage"), "TeamsAdminPage");
const UsersPage = page(() => import("./pages/admin/UsersPage"), "UsersPage");
const MatchPage = page(() => import("./pages/public/match/MatchPage"), "MatchPage");
const PickemPage = page(() => import("./pages/public/pickem/PickemPage"), "PickemPage");
const StatsPage = page(() => import("./pages/public/stats/StatsPage"), "StatsPage");

// The broadcast overlays and their control dock (pages/overlay), outside the site's shell.
const OverlayApp = lazy(() => import("./pages/overlay/OverlayApp"));

const DevSwitcher = __DEV_LOGIN__
  ? lazy(() => import("./components/DevSwitcher").then((m) => ({ default: m.DevSwitcher })))
  : null;

export function App() {
  const { user, error, refresh } = useAuth();
  const { track } = usePlayer();
  const { pathname } = useLocation();

  // OBS shows these as they are, signed out, without waiting on the session or drawing the site.
  if (pathname.startsWith("/overlay")) {
    return (
      <ErrorBoundary key={pathname} fallback="none" retryAfterMs={5000}>
        <Suspense fallback={null}>
          <Routes>
            <Route path="/overlay/*" element={<OverlayApp />} />
          </Routes>
        </Suspense>
      </ErrorBoundary>
    );
  }

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
      <AuthNotice />
      <main>
        <ErrorBoundary key={pathname}>
          <Suspense fallback={<p className="status page-loading">Yükleniyor…</p>}>
            <Routes>
              <Route path="/" element={<LandingPage />} />
              <Route path="/rules" element={<RulesPage />} />
              <Route path="/mappool" element={<PublicPoolPage />} />
              <Route path="/teams" element={<TeamsPage />} />
              <Route path="/qualifiers" element={<QualifiersPage />} />
              <Route path="/bracket" element={<BracketPage />} />
              <Route path="/match/:matchId" element={<MatchPage />} />
              <Route path="/stats" element={<StatsPage />} />
              <Route path="/pickem" element={<PickemPage />} />
              <Route path="/staff" element={<StaffPage />} />
              <Route path="/kupa" element={<ArgPage />} />

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
                    <LinkDiscordPage />
                  </RequireAuth>
                }
              />
              <Route
                path="/team"
                element={
                  <RequireAuth>
                    <TeamPage />
                  </RequireAuth>
                }
              />

              <Route
                path="/admin"
                element={
                  <RequireAuth access="staff">
                    <AdminLayout />
                  </RequireAuth>
                }
              >
                <Route index element={<AdminIndexRedirect />} />
                <Route
                  path="stages"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <StagesAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="mappool"
                  element={
                    <RequireAuth access="pool" redirect="/admin">
                      <MapPoolEditorPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="qualifiers"
                  element={
                    <RequireAuth access="referee" redirect="/admin">
                      <QualifiersAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="seeding"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <SeedingAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="bracket"
                  element={
                    <RequireAuth access="matches" redirect="/admin">
                      <BracketAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="referee/:matchId"
                  element={
                    <RequireAuth access="referee" redirect="/admin">
                      <ErrorBoundary key={pathname}>
                        <RefereeRoomPage />
                      </ErrorBoundary>
                    </RequireAuth>
                  }
                />
                <Route
                  path="users"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <UsersPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="teams"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <TeamsAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="rules"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <RulesAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="discord"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <DiscordAdminPage />
                    </RequireAuth>
                  }
                />
                <Route
                  path="bancho"
                  element={
                    <RequireAuth access="host" redirect="/admin">
                      <BanchoAdminPage />
                    </RequireAuth>
                  }
                />
              </Route>

              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </ErrorBoundary>
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
