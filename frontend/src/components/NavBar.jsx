import { NavLink, useNavigate } from "react-router-dom";
import { useAuth } from "../AuthContext.jsx";
import { hasRole } from "../roles.js";
import "./NavBar.css";
import { logout as apiLogout, loginWithOsu } from "../api.js";

export default function NavBar() {
  const { user, setUser } = useAuth();
  const navigate = useNavigate();

  const canPool = hasRole(user, "map_pooler");
  const discordLinked = Boolean(user?.discord);

  async function handleLogout() {
    await apiLogout();
    setUser(null);
    navigate("/");
  }

  const linkClass = ({ isActive }) => (isActive ? "nav-link nav-link-active" : "nav-link");

  return (
    <nav className="navbar">
      <div className="nav-left">
        <NavLink to="/" className="nav-brand">
          112 Servis <span className="nav-brand-dim">2026</span>
        </NavLink>
        <NavLink to="/" end className={linkClass}>
          Home
        </NavLink>
        {/* Public read-only pool viewer (published stages). */}
        <NavLink to="/mappool" className={linkClass}>
          Mappool
        </NavLink>
        {/* Admin console — Users tab (host) + Map Pool tab (map_pooler+). */}
        {canPool && (
          <NavLink to="/admin" className={linkClass}>
            Admin
          </NavLink>
        )}
        {/* Shown to signed-in users who haven't linked Discord yet. */}
        {user && !discordLinked && (
          <NavLink to="/discord" className={linkClass}>
            Log in with Discord
          </NavLink>
        )}
      </div>

      <div className="nav-right">
        {user ? (
          <>
            <NavLink to="/profile" className="nav-user">
              {user.avatar_url && <img className="nav-avatar" src={user.avatar_url} alt="" />}
              <span className="nav-username">{user.username}</span>
            </NavLink>
            <button className="nav-logout" onClick={handleLogout}>
              Log out
            </button>
          </>
        ) : (
          <button className="nav-login" onClick={loginWithOsu}>
            Log in with osu!
          </button>
        )}
      </div>
    </nav>
  );
}
