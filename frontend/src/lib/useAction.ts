import { useCallback, useRef, useState } from "react";
import { errorMessage } from "../api/client";

/**
 * A page's changes, one at a time: `busy` while one runs, and `error` with what went wrong (the
 * server's message). `after` runs once the change has, whether or not it went through: most pages
 * reload there, so they show what the server has either way.
 *
 * `run` resolves to whether the change itself went through, so forms know to clear.
 */
export function useAction(after?: () => Promise<unknown>) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const afterRef = useRef(after);
  afterRef.current = after;

  const run = useCallback(async (action: () => Promise<unknown>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    let done = true;
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
      done = false;
    }
    try {
      await afterRef.current?.();
    } catch (e) {
      setError(errorMessage(e));
    }
    setBusy(false);
    return done;
  }, []);

  return { busy, error, setError, run };
}
