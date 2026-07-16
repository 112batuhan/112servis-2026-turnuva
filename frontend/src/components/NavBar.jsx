import { NavLink, useNavigate } from "react-router-dom";
import { useAuth } from "../AuthContext.jsx";
import { hasRole } from "../roles.js";
import { logout as apiLogout } from "../api.js";

export default function NavBar() {
  const { user, setUser } = useAuth();
  const navigate = useNavigate();

  const isHost = hasRole(user, "host");
  const canPool = hasRole(user, "map_pooler");
  const discordLinked = Boolean(user.discord);

  async function handleLogout() {
    await apiLogout();
    setUser(null);
    navigate("/login");
  }

  const linkClass = ({ isActive }) => (isActive ? "nav-link nav-link-active" : "nav-link");

  return (
    <nav className="navbar">
      <div className="nav-left">
        <NavLink to="/" className="nav-brand">
          osu! <span className="nav-brand-dim">+ Discord</span>
        </NavLink>
        <NavLink to="/" end className={linkClass}>
          Home
        </NavLink>
        {canPool && (
          <NavLink to="/mappool" className={linkClass}>
            Map Pool
          </NavLink>
        )}
        {isHost && (
          <NavLink to="/admin" className={linkClass}>
            Admin
          </NavLink>
        )}
        {/* Only shown until the user links Discord. */}
        {!discordLinked && (
          <NavLink to="/discord" className={linkClass}>
            Log in with Discord
          </NavLink>
        )}
      </div>

      <div className="nav-right">
        {user.avatar_url && <img className="nav-avatar" src={user.avatar_url} alt="" />}
        <span className="nav-username">{user.username}</span>
        <button className="nav-logout" onClick={handleLogout}>
          Log out
        </button>
      </div>
    </nav>
  );
}
