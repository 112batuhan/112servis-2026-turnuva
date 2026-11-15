import { useEffect, useRef, useState } from "react";
import { Link, NavLink, useLocation, useNavigate } from "react-router-dom";
import { loginWithOsu } from "../api/auth";
import { useAuth } from "../auth/AuthContext";
import { canAccess, canPlay, isStaff } from "../auth/roles";
import { ADMIN_TABS, isAdminPath } from "../lib/adminTabs";
import { Icon } from "./Icon";
import { NotificationBell } from "./NotificationBell";
import "./NavBar.css";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  isActive ? "nav-link is-active" : "nav-link";

/**
 * The top bar. On the public pages it links to them, with a button on the right into the admin
 * pages for staff. On /admin it switches to the admin tabs, a tinted background and a label in
 * the middle, and the button goes back to the public page the user came from.
 */
export function NavBar() {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();
  const { pathname, search, hash } = useLocation();
  const inAdmin = isAdminPath(pathname);
  // Where "Siteye dön" leads: the last public page, or home after opening /admin directly.
  const lastPublic = useRef("/");

  useEffect(() => {
    if (!inAdmin) lastPublic.current = pathname + search + hash;
  }, [inAdmin, pathname, search, hash]);

  // The bar's height as --nav-height on the root, for anything that sticks under it (it wraps to
  // two rows on a phone).
  const barRef = useRef<HTMLElement>(null);
  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    const root = document.documentElement;
    const observer = new ResizeObserver(() =>
      root.style.setProperty("--nav-height", `${bar.offsetHeight}px`),
    );
    observer.observe(bar);
    return () => {
      observer.disconnect();
      root.style.removeProperty("--nav-height");
    };
  }, []);

  // The links never wrap: when they don't fit they scroll sideways in one row, so the bar stays
  // one line tall. Keep the current page's link in view.
  const linksRef = useRef<HTMLElement>(null);
  useEffect(() => {
    const links = linksRef.current;
    const active = links?.querySelector<HTMLElement>(".nav-link.is-active");
    if (!links || !active || links.scrollWidth <= links.clientWidth) return;
    links.scrollLeft = active.offsetLeft - (links.clientWidth - active.offsetWidth) / 2;
  }, [pathname, inAdmin]);

  // Which ends of the row have links scrolled out of view, so those edges fade out.
  const [overflow, setOverflow] = useState({ start: false, end: false });
  useEffect(() => {
    const links = linksRef.current;
    if (!links) return;
    const update = () => {
      const max = links.scrollWidth - links.clientWidth;
      const start = links.scrollLeft > 1;
      const end = links.scrollLeft < max - 1;
      setOverflow((o) => (o.start === start && o.end === end ? o : { start, end }));
    };
    update();
    links.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(links);
    return () => {
      links.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [inAdmin, user]);
  const fade =
    [overflow.start && "start", overflow.end && "end"].filter(Boolean).join(" ") || undefined;

  async function handleSignOut() {
    await signOut();
    navigate("/");
  }

  return (
    <header ref={barRef} className={inAdmin ? "navbar is-admin" : "navbar"}>
      <div className="nav-main">
        <NavLink to="/" className="nav-brand">
          <span className="brand-mark" aria-hidden="true" />
          <span className="nav-brand-text">
            112servis <span className="nav-brand-year">2026</span>
          </span>
        </NavLink>

        {inAdmin ? (
          <nav ref={linksRef} className="nav-links" data-fade={fade} aria-label="Yönetim bölümleri">
            {ADMIN_TABS.filter((tab) => canAccess(user, tab.access)).map((tab) => (
              <NavLink key={tab.path} to={tab.path} className={linkClass}>
                {tab.label}
              </NavLink>
            ))}
          </nav>
        ) : (
          <nav ref={linksRef} className="nav-links" data-fade={fade} aria-label="Ana menü">
            <NavLink to="/" end className={linkClass}>
              Ana sayfa
            </NavLink>
            <NavLink to="/rules" className={linkClass}>
              Kurallar
            </NavLink>
            <NavLink to="/mappool" className={linkClass}>
              Mappool
            </NavLink>
            <NavLink to="/teams" className={linkClass}>
              Takımlar
            </NavLink>
            <NavLink to="/qualifiers" className={linkClass}>
              Elemeler
            </NavLink>
            <NavLink to="/bracket" className={linkClass}>
              Maçlar
            </NavLink>
            <NavLink to="/stats" className={linkClass}>
              İstatistik
            </NavLink>
            <NavLink to="/pickem" className={linkClass}>
              Pick'em
            </NavLink>
            <NavLink to="/staff" className={linkClass}>
              Ekip
            </NavLink>
            {canPlay(user) && (
              <NavLink to="/team" className={linkClass}>
                Takımım
              </NavLink>
            )}
            {user && !user.discord && (
              <NavLink to="/discord" className={linkClass}>
                Discord bağla
              </NavLink>
            )}
          </nav>
        )}
      </div>

      {inAdmin && (
        <div className="nav-mode">
          <span className="nav-mode-label">
            <Icon name="settings" size={14} />
            Yönetim panelindesin
          </span>
        </div>
      )}

      <div className="nav-account">
        {inAdmin ? (
          <Link to={lastPublic.current} className="btn btn-secondary btn-sm">
            <Icon name="arrowLeft" size={14} />
            Siteye dön
          </Link>
        ) : (
          isStaff(user) && (
            <Link to="/admin" className="btn btn-secondary btn-sm nav-admin-link" title="Yönetim">
              <Icon name="settings" size={14} />
              <span className="nav-admin-text">Yönetim</span>
            </Link>
          )
        )}
        {user && <NotificationBell key={user.id} />}
        {user ? (
          <>
            <NavLink to="/profile" className="nav-user" aria-label={`Profil: ${user.username}`}>
              {user.avatar_url && <img className="nav-avatar" src={user.avatar_url} alt="" />}
              <span className="nav-username">{user.username}</span>
            </NavLink>
            <button
              type="button"
              className="btn btn-secondary btn-sm nav-signout"
              title="Çıkış yap"
              onClick={handleSignOut}
            >
              <Icon name="logOut" size={14} />
              <span className="nav-signout-text">Çıkış yap</span>
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
