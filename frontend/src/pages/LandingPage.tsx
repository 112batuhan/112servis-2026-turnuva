import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { loginWithOsu } from "../api/auth";
import { fetchPublicBracket } from "../api/bracket";
import { errorMessage } from "../api/client";
import { fetchPublicStages } from "../api/mappool";
import { fetchPublicQualifiers } from "../api/matches";
import type { BracketMatch, PublicBracket, PublicQualifiers, Stage } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { canPlay } from "../auth/roles";
import { Countdown } from "../components/Countdown";
import { FixtureRow } from "../components/FixtureRow";
import { compareByTime, fixturePath, incomingSources, withPotentials, withScore } from "../lib/bracket";
import { formatMatchTime } from "../lib/format";
import { stageTrack } from "../lib/stageTrack";
import { useLiveRefresh } from "../lib/useLiveRefresh";
import { useLiveBracket } from "../live/LiveContext";
import bannerLarge from "../assets/banner-1316.webp";
import bannerSmall from "../assets/banner-760.webp";
import crest from "../assets/crest.webp";
import "./LandingPage.css";

const UPCOMING_LIMIT = 6;
const RESULTS_LIMIT = 4;

interface Data {
  bracket: PublicBracket | null;
  qualifiers: PublicQualifiers | null;
  stages: Stage[] | null;
}

/**
 * The front page as a "now" board: what stage the tournament is in, the visitor's own team and
 * next match, the coming matches and the latest results, all from the public data.
 */
export function LandingPage() {
  const { user } = useAuth();
  const [data, setData] = useState<Data | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Each part degrades on its own: a failing request only empties its section.
    Promise.allSettled([fetchPublicBracket(), fetchPublicQualifiers(), fetchPublicStages()]).then(
      ([bracket, qualifiers, stages]) => {
        const value = <T,>(r: PromiseSettledResult<T>) => (r.status === "fulfilled" ? r.value : null);
        if (bracket.status === "rejected" && qualifiers.status === "rejected") setError(errorMessage(bracket.reason));
        setData({ bracket: value(bracket), qualifiers: value(qualifiers), stages: value(stages) });
      },
    );
  }, []);

  const matches = useMemo(() => data?.bracket?.matches ?? [], [data]);
  // While a match is on, the bracket is fetched again so live scores and results come in on their
  // own. A failed refresh keeps what's shown.
  const refreshBracket = useCallback(
    () =>
      fetchPublicBracket()
        .then((bracket) => setData((d) => (d ? { ...d, bracket } : d)))
        .catch(() => {}),
    [],
  );
  useLiveRefresh(matches, refreshBracket);
  // Saved scores arrive the moment the referee saves them; other changes reload the bracket.
  useLiveBracket(
    (e) =>
      setData((d) =>
        d?.bracket
          ? {
              ...d,
              bracket: { ...d.bracket, matches: withScore(d.bracket.matches, e.match_id, e.team1_score, e.team2_score) },
            }
          : d,
      ),
    refreshBracket,
  );
  const teamsById = useMemo(() => new Map(data?.bracket?.teams.map((t) => [t.id, t])), [data]);
  const sources = useMemo(() => incomingSources(matches), [matches]);
  const track = useMemo(
    () => (data ? stageTrack(data.bracket?.matches ?? [], data.qualifiers?.lobbies ?? [], data.stages ?? []) : []),
    [data],
  );

  // Potential matches are played too, if their teams get there: they're listed in their bracket match's place.
  const upcoming = withPotentials(matches)
    .filter((m) => !m.concluded_at && m.starts_at)
    .sort(compareByTime)
    .slice(0, UPCOMING_LIMIT);
  const results = matches
    .filter((m) => m.concluded_at)
    .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""))
    .slice(0, RESULTS_LIMIT);
  const lobbies = data?.qualifiers?.lobbies ?? [];
  // Lobbies matter until the bracket starts; one a referee forgot to close shouldn't linger after.
  const openLobbies = matches.some((m) => m.concluded_at) ? [] : lobbies.filter((lobby) => !lobby.concluded_at);

  const myTeamId = data?.bracket?.my_team_id ?? null;
  const played = matches.filter((m) => m.concluded_at).length;
  const teamCount = data?.bracket?.teams.length;

  const row = (m: BracketMatch) => (
    <li key={m.id}>
      <FixtureRow
        match={m}
        to={fixturePath(m)}
        team1={m.team1_id ? teamsById.get(m.team1_id) : undefined}
        team2={m.team2_id ? teamsById.get(m.team2_id) : undefined}
        sources={sources.get(m.id)}
        ownTeamId={myTeamId}
      />
    </li>
  );

  return (
    <div className="page landing">
      <section className="landing-head" aria-labelledby="landing-title">
        <figure className="landing-hero">
          {/* Narrow screens get just the trophy and the crystal, so the matches stay in view. */}
          <picture>
            <source media="(max-width: 1199px)" srcSet={crest} width={640} height={580} />
            <img
              src={bannerLarge}
              srcSet={`${bannerSmall} 760w, ${bannerLarge} 1316w`}
              sizes="680px"
              width={1316}
              height={590}
              alt="Buzlu bir kupanın önünde 1, 1 ve 2 rakamlarından çizilmiş buz kristali"
              fetchPriority="high"
            />
          </picture>
          {/* A small spot in the banner's centre is the hidden ARG page's door, with nothing to
              give it away. */}
          <Link to="/kupa" className="landing-hero-link" aria-label="Kupa" />
        </figure>
        <div className="landing-title-block">
          <h1 id="landing-title" className="landing-title">
            112servis <span className="landing-year">2026</span>
          </h1>
          <p className="landing-tagline">112'nin Son Kış Turnuvası</p>
          <p className="landing-lead">
            osu! 2v2 turnuvası
            {teamCount !== undefined && <> · {teamCount} takım</>}
            {matches.length > 0 && (
              <>
                {" "}
                · {played}/{matches.length} maç oynandı
              </>
            )}
          </p>
          <div className="landing-actions">
            {!user ? (
              <>
                <button type="button" className="btn btn-primary btn-lg" onClick={loginWithOsu}>
                  osu! ile giriş yap
                </button>
                <Link className="btn btn-secondary btn-lg" to="/mappool">
                  Mappool
                </Link>
              </>
            ) : (
              <Link className="btn btn-secondary btn-lg" to="/bracket">
                Maçlar
              </Link>
            )}
          </div>
        </div>
      </section>

      {track.length > 0 && (
        <nav className="stage-track" aria-label="Turnuva aşamaları">
          <ol>
            {track.map((s) => (
              <li key={s.id} className={`stage-track-item is-${s.state}`} aria-current={s.state === "current" ? "step" : undefined}>
                <span className="stage-track-name">{s.name}</span>
                {s.state === "current" && <span className="stage-track-now">Şu an</span>}
              </li>
            ))}
          </ol>
        </nav>
      )}

      {user && canPlay(user) && data?.bracket && (
        <MyTeamSection data={data.bracket} myTeamId={myTeamId} />
      )}

      {error && <p className="status">{error}</p>}
      {!data && <p className="status">Yükleniyor…</p>}

      {data && (
        <div className="landing-board">
          {openLobbies.length > 0 && (
            <section className="landing-section" aria-labelledby="landing-lobbies">
              <header className="landing-section-head">
                <h2 id="landing-lobbies">Eleme lobileri</h2>
                <Link to="/qualifiers" className="landing-more">
                  Tümü
                </Link>
              </header>
              <ul className="lobby-lines">
                {openLobbies.map((lobby) => (
                  <li key={lobby.id}>
                    <Link to="/qualifiers" className="lobby-line">
                      <span className="lobby-line-name">Lobi {lobby.name}</span>
                      <span className="lobby-line-time">{formatMatchTime(lobby.starts_at)}</span>
                      <span className="lobby-line-fill num">
                        {lobby.teams.length}/{data.qualifiers?.capacity}
                      </span>
                      <span className="lobby-line-left">
                        {lobby.open ? <Countdown to={lobby.starts_at} suffix="kaldı" /> : "Başladı"}
                      </span>
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <section className="landing-section" aria-labelledby="landing-upcoming">
            <header className="landing-section-head">
              <h2 id="landing-upcoming">Sıradaki maçlar</h2>
              <Link to="/bracket" className="landing-more">
                Tüm maçlar
              </Link>
            </header>
            {upcoming.length > 0 ? (
              <ul className="fixture-list">{upcoming.map(row)}</ul>
            ) : (
              <p className="landing-empty">
                {matches.length === 0 ? "Bracket maçları henüz açıklanmadı." : "Planlanmış maç kalmadı."}
              </p>
            )}
          </section>

          {results.length > 0 && (
            <section className="landing-section" aria-labelledby="landing-results">
              <header className="landing-section-head">
                <h2 id="landing-results">Son sonuçlar</h2>
              </header>
              <ul className="fixture-list">{results.map(row)}</ul>
            </section>
          )}
        </div>
      )}
    </div>
  );
}

/** The signed-in player's team and its next match as a fixture row, or a nudge to form a team. */
function MyTeamSection({ data, myTeamId }: { data: PublicBracket; myTeamId: string | null }) {
  const team = myTeamId ? data.teams.find((t) => t.id === myTeamId) : undefined;
  const sources = useMemo(() => incomingSources(data.matches), [data]);
  if (!team) {
    return (
      <Link to="/team" className="my-team-strip">
        <span className="my-team-label">Takımın yok</span>
        <span className="my-team-main">Bir takım arkadaşı bul ve takımını kur</span>
        <span className="my-team-go">Takım kur</span>
      </Link>
    );
  }

  const next = withPotentials(data.matches)
    .filter((m) => !m.concluded_at && (m.team1_id === team.id || m.team2_id === team.id))
    .sort(compareByTime)[0];
  const teamOf = (id: string | null) => (id ? data.teams.find((t) => t.id === id) : undefined);

  return (
    <section className="landing-section my-team" aria-labelledby="landing-my-team">
      <header className="landing-section-head">
        <h2 id="landing-my-team">
          <span className="my-team-label">Takımın</span> {team.name}
        </h2>
        <Link to="/team" className="landing-more">
          Takımım
        </Link>
      </header>
      {next ? (
        <ul className="fixture-list">
          <li>
            <FixtureRow
              match={next}
              to={fixturePath(next)}
              team1={teamOf(next.team1_id)}
              team2={teamOf(next.team2_id)}
              sources={sources.get(next.id)}
              ownTeamId={team.id}
            />
          </li>
        </ul>
      ) : (
        <p className="landing-empty">Planlanmış maçın yok.</p>
      )}
    </section>
  );
}

