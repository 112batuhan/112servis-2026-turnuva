import type { ReactNode } from "react";
import { Navigate } from "react-router-dom";
import type { Role } from "../api/types";
import { useAuth } from "./AuthContext";
import { hasRole } from "./roles";

interface Props {
  children: ReactNode;
  minRole?: Role;
  /** Where under-privileged (but signed-in) users go. Guests always go home. */
  redirect?: string;
}

export function RequireAuth({ children, minRole, redirect = "/" }: Props) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/" replace />;
  if (minRole && !hasRole(user, minRole)) return <Navigate to={redirect} replace />;
  return <>{children}</>;
}
