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

      <nav className="nav-links" aria-label="Ana menü">
        <NavLink to="/" end className={linkClass}>
          Ana sayfa
        </NavLink>
        <NavLink to="/mappool" className={linkClass}>
          Mappool
        </NavLink>
        <NavLink to="/teams" className={linkClass}>
          Takımlar
        </NavLink>
        {user?.role === "basic" && (
          <NavLink to="/team" className={linkClass}>
            Takımım
          </NavLink>
        )}
        {hasRole(user, "map_pooler") && (
          <NavLink to="/admin" className={linkClass}>
            Yönetim
          </NavLink>
        )}
        {user && !user.discord && (
          <NavLink to="/discord" className={linkClass}>
            Discord bağla
          </NavLink>
        )}
      </nav>

      <div className="nav-account">
        {user ? (
          <>
            <NavLink to="/profile" className="nav-user" aria-label={`Profil: ${user.username}`}>
              {user.avatar_url && <img className="nav-avatar" src={user.avatar_url} alt="" />}
              <span className="nav-username">{user.username}</span>
            </NavLink>
            <button type="button" className="btn btn-secondary btn-sm" onClick={handleSignOut}>
              Çıkış yap
            </button>
          </>
        ) : (
          <button type="button" className="btn btn-primary btn-sm" onClick={loginWithOsu}>
            osu! ile giriş yap
          </button>
        )}
      </div>
    </header>
  );
}
