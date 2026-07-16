import { createContext, useContext, useEffect, useState } from "react";
import { fetchCurrentUser } from "./api.js";

// Holds the current user (from /api/me) once, so the nav bar and pages share it.
// user: undefined = still loading, null = signed out, object = signed in.
const AuthContext = createContext(null);

export function AuthProvider({ children }) {
  const [user, setUser] = useState(undefined);
  const [error, setError] = useState(null);

  async function refresh() {
    try {
      setUser(await fetchCurrentUser());
    } catch {
      setError("Couldn't reach the backend. Is it running on port 8080?");
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return <AuthContext.Provider value={{ user, error, refresh, setUser }}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  return useContext(AuthContext);
}
