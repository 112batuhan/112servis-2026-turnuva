import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { fetchCurrentUser, logout } from "../api/auth";
import { onUnauthenticated } from "../api/client";
import type { User } from "../api/types";

interface AuthState {
  /** undefined while loading, null when signed out. */
  user: User | null | undefined;
  error: string | null;
  refresh: () => Promise<void>;
  signOut: () => Promise<void>;
  /** A request found the session gone while the page thought it was signed in. */
  sessionExpired: boolean;
  dismissSessionExpired: () => void;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const [sessionExpired, setSessionExpired] = useState(false);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setUser(await fetchCurrentUser());
    } catch {
      setError("Sunucuya ulaşılamadı. Backend'in çalıştığını kontrol et.");
    }
  }, []);

  const signOut = useCallback(async () => {
    await logout();
    setUser(null);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // The 7-day session ran out (or ended in another tab): show the page signed out.
  const signedIn = useRef(false);
  signedIn.current = !!user;
  useEffect(() => {
    onUnauthenticated(() => {
      if (!signedIn.current) return;
      setUser(null);
      setSessionExpired(true);
    });
    return () => onUnauthenticated(null);
  }, []);

  const dismissSessionExpired = useCallback(() => setSessionExpired(false), []);

  const value = useMemo(
    () => ({ user, error, refresh, signOut, sessionExpired, dismissSessionExpired }),
    [user, error, refresh, signOut, sessionExpired, dismissSessionExpired],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside <AuthProvider>");
  return ctx;
}

/** For pages behind <RequireAuth>, where the user is guaranteed to exist. */
export function useSignedInUser(): User {
  const { user } = useAuth();
  if (!user) throw new Error("useSignedInUser used outside <RequireAuth>");
  return user;
}
