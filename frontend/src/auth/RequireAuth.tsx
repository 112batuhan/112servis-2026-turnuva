import type { ReactNode } from "react";
import { Navigate } from "react-router-dom";
import { useAuth } from "./AuthContext";
import { type Access, canAccess } from "./roles";

interface Props {
  children: ReactNode;
  /** A role (hosts always pass) or any staff. */
  access?: Access;
  /** Where under-privileged (but signed-in) users go. Guests always go home. */
  redirect?: string;
}

export function RequireAuth({ children, access, redirect = "/" }: Props) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/" replace />;
  if (access && !canAccess(user, access)) return <Navigate to={redirect} replace />;
  return <>{children}</>;
}
