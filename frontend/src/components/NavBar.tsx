import { NavLink, useNavigate } from "react-router-dom";
import { loginWithOsu } from "../api/auth";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import "./NavBar.css";

const linkClass = ({ isActive }: { isActive: boolean }) => (isActive ? "nav-link is-active" : "nav-link");

export function NavBar() {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();

  async function handleSignOut() {
    await signOut();
    navigate("/");
  }

  return (
    <header className="navbar">
      <NavLink to="/" className="nav-brand">
        112 Servis <span className="nav-brand-year">2026</span>
      </NavLink>

      <nav className="nav-links" aria-label="Main">
        <NavLink to="/" end className={linkClass}>
          Home
        </NavLink>
        <NavLink to="/mappool" className={linkClass}>
          Mappool
        </NavLink>
        <NavLink to="/teams" className={linkClass}>
          Teams
        </NavLink>
        {user && (
          <NavLink to="/team" className={linkClass}>
            My team
          </NavLink>
        )}
        {hasRole(user, "map_pooler") && (
          <NavLink to="/admin" className={linkClass}>
            Admin
          </NavLink>
        )}
        {user && !user.discord && (
          <NavLink to="/discord" className={linkClass}>
            Link Discord
          </NavLink>
        )}
      </nav>

      <div className="nav-account">
        {user ? (
          <>
            <NavLink to="/profile" className="nav-user" aria-label={`Profile: ${user.username}`}>
              {user.avatar_url && <img className="nav-avatar" src={user.avatar_url} alt="" />}
              <span className="nav-username">{user.username}</span>
            </NavLink>
            <button type="button" className="btn btn-secondary btn-sm" onClick={handleSignOut}>
              Log out
            </button>
          </>
        ) : (
          <button type="button" className="btn btn-primary btn-sm" onClick={loginWithOsu}>
            Log in with osu!
          </button>
        )}
      </div>
    </header>
  );
}
