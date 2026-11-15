import { useEffect, useState } from "react";
import { errorMessage } from "../api/client";
import { fetchStaff } from "../api/staff";
import type { Role, StaffMember } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { DiscordMark } from "../components/DiscordMark";
import { DiscordMention } from "../components/DiscordMention";
import { ErrorAlert } from "../components/ErrorAlert";
import "./StaffPage.css";

/** The roles in the order the page lists them. */
const SECTIONS: { role: Role; title: string }[] = [
  { role: "host", title: "Hostlar" },
  { role: "map_pooler", title: "Map Poolerlar" },
  { role: "referee", title: "Hakemler" },
  { role: "graphics_designer", title: "Grafik Tasarımcılar" },
  { role: "custom_mapper", title: "Custom Mapperlar" },
  { role: "streamer", title: "Yayıncılar" },
  { role: "pool_tester", title: "Pool Testerlar" },
  { role: "commentator", title: "Yorumcular" },
];

export function StaffPage() {
  const { user } = useAuth();
  const [staff, setStaff] = useState<StaffMember[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchStaff()
      .then(setStaff)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Ekip</h1>
        {staff && <span className="badge">{staff.length}</span>}
        <div className="page-header-aside staff-hint">
          Discord adına tıklarsan etiketi kopyalanır.
        </div>
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!staff && !error && <p className="status">Yükleniyor…</p>}

      {staff && (
        <div className="staff-roles">
          {SECTIONS.map(({ role, title }) => {
            const members = staff.filter((m) => m.roles.includes(role));
            const headingId = `staff-${role}`;
            return (
              <section key={role} className="staff-role" aria-labelledby={headingId}>
                <header className="staff-role-head">
                  <h2 id={headingId}>{title}</h2>
                  <span className="staff-role-count">{members.length}</span>
                </header>
                {members.length === 0 ? (
                  <p className="staff-empty">Henüz kimse yok.</p>
                ) : (
                  <ul className="staff-list">
                    {members.map((member) => (
                      <StaffRow key={member.id} member={member} isYou={member.id === user?.id} />
                    ))}
                  </ul>
                )}
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}

function StaffRow({ member, isYou }: { member: StaffMember; isYou: boolean }) {
  return (
    <li className={`staff-member ${isYou ? "is-you" : ""}`}>
      <img
        className="staff-avatar"
        src={member.avatar_url}
        alt=""
        width={40}
        height={40}
        loading="lazy"
      />
      <div className="staff-member-text">
        <span className="staff-member-name-line">
          <a
            className="staff-member-name"
            href={`https://osu.ppy.sh/users/${member.osu_id}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {member.username}
          </a>
          {/* Everyone is assumed Turkish; only the exceptions are worth naming. */}
          {member.country_code && member.country_code !== "TR" && (
            <span className="staff-member-country">{member.country_code}</span>
          )}
          {isYou && <span className="staff-member-you">sen</span>}
        </span>
        <span className={`staff-member-discord ${member.discord_username ? "" : "is-missing"}`}>
          <DiscordMark size={14} />
          <DiscordMention player={member} fallback="Discord bağlı değil" />
        </span>
      </div>
    </li>
  );
}
