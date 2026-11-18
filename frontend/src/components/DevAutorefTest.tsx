import { useEffect, useState } from "react";
import { devSeedAutoref, fetchDevAutorefTest } from "../api/dev";
import type { DevAccount, DevAutorefTest as TestInfo } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import "./DevAutorefTest.css";

const SLOTS = [
  { team: 0, label: "Blue 1" },
  { team: 0, label: "Blue 2" },
  { team: 1, label: "Red 1" },
  { team: 1, label: "Red 2" },
];

// The dev switcher's auto referee test: four accounts in two teams and a bracket match
// starting in 10 minutes, so its room opens at once (with auto rooms on) and the auto
// referee runs it with real players. Dummies can fill in: their invites find nobody. `run` is the switcher's: busy, errors and the
// page change after it.
export function DevAutorefTest({
  accounts,
  busy,
  run,
}: {
  accounts: DevAccount[];
  busy: boolean;
  run: (action: () => Promise<void>, after?: () => void) => void;
}) {
  const { user } = useAuth();
  const [info, setInfo] = useState<TestInfo | null>(null);
  const [stage, setStage] = useState("");
  const [picked, setPicked] = useState<string[]>(["", "", "", ""]);

  useEffect(() => {
    fetchDevAutorefTest()
      .then((i) => {
        setInfo(i);
        setStage((s) => s || (i.stages[0]?.id ?? ""));
      })
      .catch(() => setInfo({ stages: [], auto_rooms: false, test_teams: [] }));
  }, []);

  const real = accounts.filter((a) => !a.dummy);
  // Free dummies first: the ones in a seeded team can't be picked.
  const dummies = accounts
    .filter((a) => a.dummy)
    .sort((a, b) => Number(a.team_name !== null) - Number(b.team_name !== null));
  // In a team other than the last test's, so the seed would refuse them.
  const taken = (a: DevAccount) =>
    a.team_name !== null && !(info?.test_teams.includes(a.team_name) ?? false);
  const ready = stage !== "" && picked.every((p) => p !== "") && new Set(picked).size === 4;

  function pick(slot: number, id: string) {
    setPicked((p) => p.map((v, i) => (i === slot ? id : v)));
  }

  function create() {
    let matchId = "";
    run(
      async () => {
        const res = await devSeedAutoref(stage, picked.slice(0, 2), picked.slice(2));
        matchId = res.match_id;
      },
      () => {
        // A referee goes to the cockpit, anyone else to the public match page.
        window.location.href = hasRole(user, "referee")
          ? `/admin/referee/${matchId}`
          : `/match/${matchId}`;
      },
    );
  }

  if (!info) return <p className="muted dev-autoref-note">Loading…</p>;

  return (
    <div className="dev-autoref">
      <label className="dev-autoref-field">
        <span>Stage</span>
        <select
          className="select select-sm"
          value={stage}
          disabled={busy}
          onChange={(e) => setStage(e.target.value)}
        >
          {info.stages.length === 0 && (
            <option value="">No stage with a pool, best of and order</option>
          )}
          {info.stages.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </label>
      <div className="dev-autoref-teams">
        {SLOTS.map((slot, i) => (
          <label
            key={slot.label}
            className={`dev-autoref-field ${slot.team === 0 ? "is-blue" : "is-red"}`}
          >
            <span>{slot.label}</span>
            <select
              className="select select-sm"
              value={picked[i]}
              disabled={busy}
              onChange={(e) => pick(i, e.target.value)}
            >
              <option value="">Pick an account</option>
              {[
                { label: "Real accounts", list: real },
                { label: "Dummies", list: dummies },
              ].map((group) => (
                <optgroup key={group.label} label={group.label}>
                  {group.list.map((a) => (
                    <option
                      key={a.id}
                      value={a.id}
                      disabled={taken(a) || (picked.includes(a.id) && picked[i] !== a.id)}
                    >
                      {a.username}
                      {a.team_name !== null ? ` · ${a.team_name}` : ""}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </label>
        ))}
      </div>
      <p className="muted dev-autoref-note">
        Real players have to sign in with osu! once to be listed. Dummies only fill a slot: they
        never join, so their team forfeits once the no-show wait is over.
      </p>
      {!info.auto_rooms && (
        <p className="dev-autoref-warn">
          Auto rooms are off, so the room won't open by itself: turn them on at /admin/bancho (with
          an active bot).
        </p>
      )}
      <p className="muted dev-autoref-note">
        Starts in 10 minutes, so the room opens right away and the players are invited. Replaces the
        last test's teams and match; the rest of the data stays.
      </p>
      <button
        type="button"
        className="btn btn-primary btn-sm"
        disabled={busy || !ready}
        onClick={create}
      >
        Create match in 10 min
      </button>
    </div>
  );
}
