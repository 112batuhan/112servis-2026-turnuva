import { Link } from "react-router-dom";
import { useAuth } from "../AuthContext.jsx";
import { loginWithOsu } from "../api.js";
import { hasRole } from "../roles.js";

// Public landing page — visible to everyone, signed in or not. More public pages
// (schedule, bracket, pool showcase) will live alongside this later.
export default function LandingPage() {
  const { user } = useAuth();

  return (
    <div className="content landing">
      <section className="hero">
        <p className="hero-eyebrow">osu! tournament</p>
        <h1 className="hero-title">112 Servis 2026 Turnuva</h1>
        <p className="hero-lead">
          Follow the tournament — stages, mappools, and results. Sign in with osu! to take part, or
          just browse along.
        </p>

        <div className="hero-actions">
          {user ? (
            <>
              <span className="hero-hi">Signed in as {user.username}</span>
              {hasRole(user, "map_pooler") && (
                <Link className="osu-btn hero-btn" to="/admin/mappool">
                  Go to Map Pool
                </Link>
              )}
            </>
          ) : (
            <button className="osu-btn hero-btn" onClick={loginWithOsu}>
              Log in with osu!
            </button>
          )}
        </div>
      </section>

      <section className="landing-cards">
        <div className="landing-card">
          <h3>Stages &amp; mappools</h3>
          <p className="muted">
            Every stage of the tournament has its own pool of maps across mod categories.
          </p>
        </div>
        <div className="landing-card">
          <h3>Public pages coming soon</h3>
          <p className="muted">
            Schedules, brackets, and pool showcases will be viewable here without signing in.
          </p>
        </div>
        <div className="landing-card">
          <h3>Get involved</h3>
          <p className="muted">
            Sign in with osu! and link Discord to participate. Staff manage pools and roles.
          </p>
        </div>
      </section>
    </div>
  );
}
