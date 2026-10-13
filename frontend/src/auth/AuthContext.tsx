import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { fetchCurrentUser, logout } from "../api/auth";
import type { User } from "../api/types";

interface AuthState {
  /** undefined while loading, null when signed out. */
  user: User | null | undefined;
  error: string | null;
  refresh: () => Promise<void>;
  signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setUser(await fetchCurrentUser());
    } catch {
      setError("Couldn't reach the server. Check that the backend is running.");
    }
  }, []);

  const signOut = useCallback(async () => {
    await logout();
    setUser(null);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const value = useMemo(() => ({ user, error, refresh, signOut }), [user, error, refresh, signOut]);
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
