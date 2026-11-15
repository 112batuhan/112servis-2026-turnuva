import type { Access } from "../auth/roles";

/** The admin tabs, in order, each with who can open it. The top bar shows them on /admin pages. */
export const ADMIN_TABS: { path: string; label: string; access: Access }[] = [
  { path: "/admin/stages", label: "Stage'ler", access: "host" },
  { path: "/admin/mappool", label: "Mappool", access: "pool" },
  { path: "/admin/qualifiers", label: "Elemeler", access: "referee" },
  { path: "/admin/seeding", label: "Sıralama sonuçları", access: "host" },
  { path: "/admin/bracket", label: "Maçlar", access: "matches" },
  { path: "/admin/users", label: "Kullanıcılar", access: "host" },
  { path: "/admin/teams", label: "Takımlar", access: "host" },
  { path: "/admin/rules", label: "Kurallar", access: "host" },
  { path: "/admin/discord", label: "Discord", access: "host" },
  { path: "/admin/bancho", label: "Bancho", access: "host" },
];

export const isAdminPath = (pathname: string) => pathname === "/admin" || pathname.startsWith("/admin/");
