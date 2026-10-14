import type { Role } from "../../api/types";

/** The admin tabs, in order, each with who can open it. The top bar shows them on /admin pages. */
export const ADMIN_TABS: { path: string; label: string; minRole: Role }[] = [
  { path: "/admin/stages", label: "Stage'ler", minRole: "host" },
  { path: "/admin/mappool", label: "Mappool", minRole: "map_pooler" },
  { path: "/admin/qualifiers", label: "Elemeler", minRole: "referee" },
  { path: "/admin/bracket", label: "Maçlar", minRole: "streamer" },
  { path: "/admin/users", label: "Kullanıcılar", minRole: "host" },
  { path: "/admin/teams", label: "Takımlar", minRole: "host" },
  { path: "/admin/discord", label: "Discord", minRole: "host" },
];

export const isAdminPath = (pathname: string) => pathname === "/admin" || pathname.startsWith("/admin/");
