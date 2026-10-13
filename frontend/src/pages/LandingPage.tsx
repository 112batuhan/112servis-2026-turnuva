import { Link } from "react-router-dom";
import { loginWithOsu } from "../api/auth";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import "./LandingPage.css";

export function LandingPage() {
  const { user } = useAuth();

  return (
    <div className="page page-narrow">
      <section className="hero">
        <p className="hero-eyebrow">osu! tournament</p>
        <h1 className="hero-title">112 Servis 2026 Turnuva</h1>
        <p className="hero-lead">
          Follow the tournament: stages, mappools and results. Sign in with osu! to take part, or just
          browse along.
        </p>

        <div className="hero-actions">
          <Link className="btn btn-secondary btn-lg" to="/mappool">
            View mappool
          </Link>
          {!user && (
            <button type="button" className="btn btn-primary btn-lg" onClick={loginWithOsu}>
              Log in with osu!
            </button>
          )}
          {hasRole(user, "map_pooler") && (
            <Link className="btn btn-primary btn-lg" to="/admin/mappool">
              Open pool editor
            </Link>
          )}
        </div>
        {user && <p className="hero-signed-in">Signed in as {user.username}</p>}
      </section>

      <section className="landing-cards" aria-label="About the tournament">
        <Link className="landing-card is-link" to="/mappool">
          <h2>Stages &amp; mappools</h2>
          <p>Every stage has its own pool of maps across mod categories. Listen to previews and grab downloads.</p>
        </Link>
        <div className="landing-card">
          <h2>Schedule &amp; bracket</h2>
          <p>Match times and the bracket will be published here as the tournament gets going.</p>
        </div>
        <div className="landing-card">
          <h2>Get involved</h2>
          <p>Sign in with osu! and link your Discord to participate. Staff manage pools and roles.</p>
        </div>
      </section>
    </div>
  );
}
