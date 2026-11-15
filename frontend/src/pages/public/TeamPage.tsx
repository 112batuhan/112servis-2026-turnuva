import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../api/client";
import { useAction } from "../lib/useAction";
import {
  acceptInvite,
  deleteInvite,
  disbandMyTeam,
  fetchTeamStatus,
  removeMyBanner,
  renameMyTeam,
  searchPlayers,
  sendInvite,
  uploadMyBanner,
} from "../api/teams";
import { fetchPublicQualifiers } from "../api/matches";
import type { Invite, PlayerResult, PublicQualifiers, Team, TeamStatus } from "../api/types";
import { useSignedInUser } from "../auth/AuthContext";
import { canPlay } from "../auth/roles";
import { Countdown } from "../components/Countdown";
import { DiscordMark } from "../components/DiscordMark";
import { DiscordMention } from "../components/DiscordMention";
import { ErrorAlert } from "../components/ErrorAlert";
import { Icon } from "../components/Icon";
import { MatchReschedules } from "../components/MatchReschedules";
import { PlayerChip } from "../components/PlayerChip";
import { LobbyStatus } from "../components/LobbyCard";
import { MatchPageLink } from "../components/MatchPageLink";
import { RoomLinks } from "../components/RoomLinks";
import { TeamCard } from "../components/TeamCard";
import { TeamMatches } from "../components/TeamMatches";
import { isLobbyLive } from "../lib/bracket";
import { ELIGIBILITY_LABELS, eligibilityBadgeClass } from "../lib/eligibility";
import {
  BANNER_MAX_BYTES,
  formatMatchTime,
  formatRank,
  formatTeamValue,
  TEAM_NAME_MAX,
} from "../lib/format";
import "./TeamPage.css";

const SEARCH_DELAY_MS = 300;

/** Warns when a player of the team left the tournament's Discord server, where its matches are arranged. */
function LeftDiscordNotice({ team, left, myId }: { team: Team; left: string[]; myId: string }) {
  if (left.length === 0) return null;
  if (left.includes(myId)) {
    return (
      <p className="alert alert-error team-rank-warning" role="status">
        Turnuvanın Discord sunucusundan ayrıldın. Maçlarının kanallarını görebilmek için sunucuya
        geri dön.
      </p>
    );
  }
  const names = team.players.filter((p) => left.includes(p.id)).map((p) => p.username);
  return (
    <p className="alert alert-error team-rank-warning" role="status">
      {names.join(", ")} turnuvanın Discord sunucusundan ayrıldı. Maç kanallarını görebilmesi için
      sunucuya geri dönmesi gerekiyor.
    </p>
  );
}

export function TeamPage() {
  const user = useSignedInUser();
  const isStaff = !canPlay(user);
  const [status, setStatus] = useState<TeamStatus | null>(null);
  // A change, then the team and the search results again either way.
  const { busy, error, setError, run } = useAction(() => Promise.all([reload(), runSearch(query)]));
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<PlayerResult[] | null>(null);
  const [teamName, setTeamName] = useState("");
  // Bumped when a match is rescheduled, so the match list loads its new time.
  const [matchesVersion, setMatchesVersion] = useState(0);
  const frozen = status?.frozen ?? false;
  const banned = user.ban !== null;
  const canForm = !isStaff && !banned && user.discord !== null && !frozen;

  const reload = useCallback(async () => {
    const next = await fetchTeamStatus();
    setStatus(next);
    return next;
  }, []);

  useEffect(() => {
    reload()
      .then((s) => setTeamName(s.sent[0]?.team_name ?? ""))
      .catch((e) => setError(errorMessage(e)));
  }, [reload, setError]);

  const runSearch = useCallback(async (q: string) => {
    setResults(q.trim() ? await searchPlayers(q) : null);
  }, []);

  useEffect(() => {
    const timer = setTimeout(
      () => runSearch(query).catch((e) => setError(errorMessage(e))),
      SEARCH_DELAY_MS,
    );
    return () => clearTimeout(timer);
  }, [query, runSearch, setError]);

  // Staff can't play, but someone promoted after forming a team still sees it.
  const staffOnly = isStaff && status !== null && !status.team;

  return (
    <div className="page page-narrow team-page">
      <div className="page-header">
        <h1>Takımım</h1>
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!status && !error && <p className="status">Yükleniyor…</p>}

      {staffOnly && (
        <section className="panel">
          <p className="status">
            Yetkililer (hostlar, map pooler'lar ve hakemler) turnuvayı yönettiği için takıma
            katılamaz. Yayıncılar oynayabilir.
          </p>
        </section>
      )}

      {status && !staffOnly && (
        <>
          {user.ban && (
            <div className="alert alert-error team-ban-notice" role="status">
              <Icon name="ban" size={20} />
              <div className="team-ban-notice-body">
                <strong className="team-ban-notice-title">Hostlar seni turnuvadan banladı.</strong>
                {user.ban.reason && (
                  <span className="team-ban-notice-reason">Sebep: {user.ban.reason}</span>
                )}
                <span className="team-ban-notice-effects">
                  Takım kuramaz, takımını değiştiremez, eleme lobisine katılamaz ve maç saati
                  öneremezsin.
                </span>
              </div>
            </div>
          )}
          <RankSummary status={status} />
          {frozen && (
            <section className="panel">
              <p className="status">
                Hostlar takımları dondurdu, artık takım kurulamaz veya değiştirilemez.
              </p>
            </section>
          )}
          {!user.discord && !status.team && !frozen && !banned && (
            <section className="panel team-discord-notice">
              <p>Davet göndermek ve kabul etmek için Discord hesabını bağla.</p>
              <Link to="/discord" className="btn btn-discord btn-sm">
                <DiscordMark />
                Discord bağla
              </Link>
            </section>
          )}

          {status.team ? (
            <>
              <section className="panel" aria-label="Takımın">
                {status.team.rank_warning && (
                  <p className="alert alert-error team-rank-warning" role="status">
                    {status.team.rank_warning === "over_limit"
                      ? `Rank güncellemesinden sonra takımın değeri ${formatTeamValue(status.team.team_value)} oldu ve ${formatTeamValue(status.limit)} limitini aşıyor.`
                      : "Takımdan birinin artık rank'ı yok (restrict ya da ban olabilir), bu yüzden takım limite göre kontrol edilemiyor."}{" "}
                    Hostlar bilgilendirildi.
                  </p>
                )}
                <LeftDiscordNotice team={status.team} left={status.left_discord} myId={user.id} />
                <MyTeam
                  team={status.team}
                  readOnly={frozen || banned}
                  busy={busy}
                  run={run}
                  onError={setError}
                />
                <QualifierInfo teamId={status.team.id} canJoin={!banned} onError={setError} />
              </section>
              <section className="panel">
                <h2>Maçlar</h2>
                <TeamMatches key={matchesVersion} teamId={status.team.id} />
              </section>
              <MatchReschedules
                onRescheduled={() => setMatchesVersion((v) => v + 1)}
                onError={setError}
              />
            </>
          ) : (
            <>
              <InviteList
                title="Sana gelen davetler"
                empty="Henüz kimse seni davet etmedi."
                invites={status.received}
                actions={(invite) => (
                  <>
                    <button
                      type="button"
                      className="btn btn-primary btn-sm"
                      disabled={busy || !canForm || invite.eligibility !== "eligible"}
                      title={
                        !user.discord
                          ? "Kabul etmek için Discord hesabını bağla"
                          : invite.eligibility !== "eligible"
                            ? `Bu takım kurulamaz: ${ELIGIBILITY_LABELS[invite.eligibility].toLocaleLowerCase("tr-TR")}`
                            : undefined
                      }
                      onClick={() => run(() => acceptInvite(invite.id))}
                    >
                      Kabul et
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      disabled={busy}
                      onClick={() => run(() => deleteInvite(invite.id))}
                    >
                      Reddet
                    </button>
                  </>
                )}
              />
              <InviteList
                title="Gönderdiğin davetler"
                empty="Henüz kimseyi davet etmedin."
                invites={status.sent}
                actions={(invite) => (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    disabled={busy}
                    onClick={() => run(() => deleteInvite(invite.id))}
                  >
                    İptal et
                  </button>
                )}
              />

              {!frozen && !banned && (
                <section className="panel">
                  <h2>Takım arkadaşı bul</h2>
                  <label className="team-name-field">
                    <span>Takım adı</span>
                    <input
                      className="input"
                      placeholder="Takımına bir ad ver"
                      maxLength={TEAM_NAME_MAX}
                      value={teamName}
                      onChange={(e) => setTeamName(e.target.value)}
                    />
                  </label>
                  <p className="muted team-name-hint">
                    Her davetle birlikte gönderilir. Takım kurulduktan sonra ikiniz de
                    değiştirebilirsiniz.
                  </p>
                  <input
                    className="input team-search"
                    type="search"
                    placeholder="osu! kullanıcı adıyla ara"
                    aria-label="Oyuncu ara"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                  {results?.length === 0 && <p className="status">Oyuncu bulunamadı.</p>}
                  {results && results.length > 0 && (
                    <ul className="team-rows">
                      {results.map((p) => (
                        <li key={p.id} className="team-row">
                          <PlayerChip player={p} />
                          <RowMeta eligibility={p.eligibility} teamValue={p.team_value} />
                          <div className="team-row-actions">
                            {p.invite_received ? (
                              <span className="badge">Seni davet etti</span>
                            ) : p.invite_sent ? (
                              <span className="badge">Davet gönderildi</span>
                            ) : (
                              <button
                                type="button"
                                className="btn btn-primary btn-sm"
                                disabled={
                                  busy ||
                                  !canForm ||
                                  !teamName.trim() ||
                                  p.eligibility !== "eligible"
                                }
                                title={teamName.trim() ? undefined : "Önce takımına bir ad ver"}
                                onClick={() => run(() => sendInvite(p.id, teamName))}
                              >
                                Davet et
                              </button>
                            )}
                          </div>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}

interface MyTeamProps {
  team: Team;
  /** Frozen teams, or a banned player: only shown, nothing to change. */
  readOnly: boolean;
  busy: boolean;
  run: (action: () => Promise<void>) => Promise<unknown>;
  onError: (message: string) => void;
}

function MyTeam({ team, readOnly, busy, run, onError }: MyTeamProps) {
  const fileInput = useRef<HTMLInputElement>(null);
  if (readOnly) return <TeamCard team={team} showMatches={false} showValue={false} />;

  function upload(file: File | undefined) {
    if (!file) return;
    if (file.size > BANNER_MAX_BYTES) {
      onError("Banner en fazla 2 MB olabilir.");
      return;
    }
    run(() => uploadMyBanner(file));
  }

  return (
    <>
      <TeamCard
        team={team}
        showMatches={false}
        showValue={false}
        onRename={(name) => run(() => renameMyTeam(name))}
        actions={
          <>
            <input
              ref={fileInput}
              type="file"
              accept="image/png,image/jpeg,image/gif,image/webp"
              hidden
              onChange={(e) => {
                upload(e.target.files?.[0]);
                e.target.value = "";
              }}
            />
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy}
              onClick={() => fileInput.current?.click()}
            >
              {team.banner_url ? "Banner'ı değiştir" : "Banner ekle"}
            </button>
            {team.banner_url && (
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                disabled={busy}
                onClick={() => run(removeMyBanner)}
              >
                Banner'ı kaldır
              </button>
            )}
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={() => {
                if (
                  window.confirm(
                    "Takımını dağıtmak istiyor musun? Sen ve takım arkadaşın yeniden takımsız kalırsınız.",
                  )
                ) {
                  run(disbandMyTeam);
                }
              }}
            >
              Dağıt
            </button>
          </>
        }
      />
      <p className="muted team-name-hint">
        Banner en fazla 2 MB boyutunda PNG, JPEG, GIF veya WebP olmalı; 3:1 oranında kırpılır.
      </p>
    </>
  );
}

interface QualifierInfoProps {
  teamId: string;
  /** A banned player can't join a lobby, so they aren't pointed at one. */
  canJoin: boolean;
  onError: (message: string) => void;
}

// The team's qualifier lobby, until its match is concluded.
function QualifierInfo({ teamId, canJoin, onError }: QualifierInfoProps) {
  const [qualifiers, setQualifiers] = useState<PublicQualifiers | null>(null);

  const load = useCallback(() => {
    fetchPublicQualifiers()
      .then(setQualifiers)
      .catch((e) => onError(errorMessage(e)));
  }, [onError]);

  useEffect(load, [load, teamId]);

  if (!qualifiers) return null;
  const lobby = qualifiers.lobbies.find((l) => l.teams.some((t) => t.id === teamId));
  if (!lobby) {
    if (!canJoin) return null;
    return (
      <p className="muted team-name-hint">
        Eleme lobinizi <Link to="/qualifiers">Elemeler</Link> sayfasından seçebilirsiniz.
      </p>
    );
  }
  if (lobby.concluded_at) return null;

  return (
    <div className="team-qualifier">
      <div className="team-qualifier-text">
        <span className="team-qualifier-label">Eleme lobisi</span>
        <span>
          <Link to="/qualifiers" className="team-qualifier-lobby">
            Lobi {lobby.name}
          </Link>{" "}
          · {formatMatchTime(lobby.starts_at)}
        </span>
        <span className="muted">
          Hakem:{" "}
          {lobby.referee ? (
            <>
              {lobby.referee.username} <DiscordMention player={lobby.referee} fallback="" />
            </>
          ) : (
            "henüz yok"
          )}
        </span>
      </div>
      <div className="team-qualifier-links">
        <MatchPageLink matchId={lobby.id} live={isLobbyLive(lobby)} variant="text" />
        <RoomLinks rooms={lobby.rooms} />
      </div>
      {lobby.open ? (
        <div className="team-qualifier-countdown">
          <span className="team-qualifier-label">Başlamasına</span>
          <Countdown to={lobby.starts_at} className="team-qualifier-time" onElapsed={load} />
        </div>
      ) : (
        <LobbyStatus lobby={lobby} full={false} />
      )}
    </div>
  );
}

/**
 * Where the player stands against the team value limit: the team's value on a meter once it's
 * formed, otherwise the best rank a teammate may have.
 */
function RankSummary({ status }: { status: TeamStatus }) {
  const value = status.team?.team_value ?? null;
  if (status.team && value !== null) {
    const over = value > status.limit;
    return (
      <section className="team-limit" aria-label="Takım değeri">
        <p className="team-limit-figures">
          <span className="team-limit-label">Takım değeri</span>
          <span className={over ? "team-limit-value num is-over" : "team-limit-value num"}>
            {formatTeamValue(value)}
          </span>
          <span className="team-limit-max num">/ {formatTeamValue(status.limit)}</span>
        </p>
        <div
          className={over ? "team-limit-meter is-over" : "team-limit-meter"}
          role="meter"
          aria-label="Takım değerinin limite oranı"
          aria-valuemin={0}
          aria-valuemax={status.limit}
          aria-valuenow={Math.round(value)}
        >
          <span style={{ width: `${Math.min(100, (value / status.limit) * 100)}%` }} />
        </div>
      </section>
    );
  }

  return (
    <section className="team-limit" aria-label="Takım değeri limiti">
      {status.global_rank && status.min_partner_rank ? (
        <>
          <p className="team-limit-figures">
            <span className="team-limit-label">Takım arkadaşın en iyi</span>
            <span className="team-limit-value num">{formatRank(status.min_partner_rank)}</span>
          </p>
          <p className="muted team-limit-note">
            Rank'ın {formatRank(status.global_rank)}. İkinizin takım değeri{" "}
            {formatTeamValue(status.limit)} limitini aşmamalı, bu yüzden takım arkadaşının rank'ı{" "}
            {formatRank(status.min_partner_rank)} ya da daha geride olmalı.
          </p>
        </>
      ) : (
        <p>Global rank'ın olmadığı için takım kuramazsın.</p>
      )}
    </section>
  );
}

interface RowMetaProps {
  eligibility: Invite["eligibility"];
  teamValue: number | null;
  teamName?: string;
}

function RowMeta({ eligibility, teamValue, teamName }: RowMetaProps) {
  return (
    <div className="team-row-meta">
      {teamName && <span className="team-row-team">{teamName}</span>}
      <span className={eligibilityBadgeClass(eligibility)}>{ELIGIBILITY_LABELS[eligibility]}</span>
      <span className="muted" title="İkinizin takım değeri">
        {formatTeamValue(teamValue)}
      </span>
    </div>
  );
}

interface InviteListProps {
  title: string;
  empty: string;
  invites: Invite[];
  actions: (invite: Invite) => ReactNode;
}

function InviteList({ title, empty, invites, actions }: InviteListProps) {
  return (
    <section className="panel">
      <h2>
        {title} {invites.length > 0 && <span className="badge">{invites.length}</span>}
      </h2>
      {invites.length === 0 ? (
        <p className="status">{empty}</p>
      ) : (
        <ul className="team-rows">
          {invites.map((invite) => (
            <li key={invite.id} className="team-row">
              <PlayerChip player={invite.player} />
              <RowMeta
                eligibility={invite.eligibility}
                teamValue={invite.team_value}
                teamName={invite.team_name}
              />
              <div className="team-row-actions">{actions(invite)}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
