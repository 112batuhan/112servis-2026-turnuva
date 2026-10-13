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
        <p className="hero-eyebrow">osu! turnuvası</p>
        <h1 className="hero-title">112 Servis 2026 Turnuva</h1>
        <p className="hero-lead">
          Turnuvayı takip et: stage'ler, mappool'lar ve sonuçlar. Katılmak için osu! ile giriş yap ya da sadece
          göz at.
        </p>

        <div className="hero-actions">
          <Link className="btn btn-secondary btn-lg" to="/mappool">
            Mappool'u gör
          </Link>
          {!user && (
            <button type="button" className="btn btn-primary btn-lg" onClick={loginWithOsu}>
              osu! ile giriş yap
            </button>
          )}
          {hasRole(user, "map_pooler") && (
            <Link className="btn btn-primary btn-lg" to="/admin/mappool">
              Mappool editörünü aç
            </Link>
          )}
        </div>
        {user && <p className="hero-signed-in">{user.username} olarak giriş yapıldı</p>}
      </section>

      <section className="landing-cards" aria-label="Turnuva hakkında">
        <Link className="landing-card is-link" to="/mappool">
          <h2>Stage'ler ve mappool'lar</h2>
          <p>Her stage'in mod kategorilerine ayrılmış kendi mappool'u var. Önizlemeleri dinle, map'leri indir.</p>
        </Link>
        <div className="landing-card">
          <h2>Program ve bracket</h2>
          <p>Turnuva ilerledikçe maç saatleri ve bracket burada yayınlanacak.</p>
        </div>
        <div className="landing-card">
          <h2>Katıl</h2>
          <p>Katılmak için osu! ile giriş yap ve Discord hesabını bağla. Mappool'ları ve rolleri yetkililer yönetir.</p>
        </div>
      </section>
    </div>
  );
}
