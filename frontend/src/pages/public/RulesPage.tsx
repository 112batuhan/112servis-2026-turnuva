import { useEffect, useMemo, useState } from "react";
import { errorMessage } from "../api/client";
import { fetchRules } from "../api/rules";
import type { Rules } from "../api/types";
import { BBCode } from "../components/BBCode";
import { ErrorAlert } from "../components/ErrorAlert";
import { type BBHeading, headingsOf, parseBBCode } from "../lib/bbcode";
import { formatDate } from "../lib/format";
import "./RulesPage.css";

export function RulesPage() {
  const [rules, setRules] = useState<Rules | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchRules()
      .then(setRules)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const nodes = useMemo(() => (rules ? parseBBCode(rules.bbcode) : []), [rules]);
  const headings = useMemo(() => headingsOf(nodes), [nodes]);
  const current = useCurrentHeading(headings);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Kurallar</h1>
        {rules && (
          <div className="page-header-aside rules-updated">
            Son güncelleme: <span className="num">{formatDate(rules.updated_at)}</span>
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!rules && !error && <p className="status">Yükleniyor…</p>}
      {rules && !rules.bbcode.trim() && <p className="status">Kurallar henüz yazılmadı.</p>}

      {rules && rules.bbcode.trim() && (
        <div className={`rules-layout ${headings.length ? "has-toc" : ""}`}>
          {headings.length > 0 && (
            <nav className="rules-toc" aria-label="Bölümler">
              <ol>
                {headings.map((h) => (
                  <li key={h.id}>
                    <a
                      href={`#${h.id}`}
                      className={h.id === current ? "is-current" : ""}
                      aria-current={h.id === current ? "location" : undefined}
                    >
                      {h.title}
                    </a>
                  </li>
                ))}
              </ol>
            </nav>
          )}
          <BBCode nodes={nodes} headings={headings} />
        </div>
      )}
    </div>
  );
}

/** How far below the top of the window a heading counts as the one being read (past the top bar). */
const READING_LINE = 140;

/** The section being read: the last heading that has scrolled up past the reading line. */
function useCurrentHeading(headings: BBHeading[]): string | null {
  const [current, setCurrent] = useState<string | null>(null);

  useEffect(() => {
    if (!headings.length) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      let found: string | null = headings[0].id;
      for (const h of headings) {
        const el = document.getElementById(h.id);
        if (el && el.getBoundingClientRect().top <= READING_LINE) found = h.id;
      }
      setCurrent(found);
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
    };
  }, [headings]);

  return current;
}
