import { describe, expect, it } from "vitest";
import type { Role } from "../api/types";
import { canAccess, canPlay, canViewPool, hasRole, holdsRole, isStaff } from "./roles";

// The same cases as backend/src/role.rs, which this file mirrors: the server decides, and the
// pages should offer exactly what it allows.

const user = (...roles: Role[]) => ({ roles });

describe("roles", () => {
  it("lets a host do everything while holding only Host", () => {
    const host = user("host");
    expect(hasRole(host, "referee") && hasRole(host, "map_pooler")).toBe(true);
    expect(holdsRole(host, "referee")).toBe(false);
    expect(canPlay(host)).toBe(false);
  });

  it("doesn't let one role stand for another", () => {
    const pooler = user("map_pooler");
    expect(hasRole(pooler, "map_pooler")).toBe(true);
    expect(hasRole(pooler, "referee") || hasRole(pooler, "streamer")).toBe(false);
  });

  it("lets only players, streamers, designers and commentators play", () => {
    expect(canPlay(user())).toBe(true);
    expect(canPlay(user("streamer"))).toBe(true);
    expect(canPlay(user("streamer", "graphics_designer"))).toBe(true);
    expect(canPlay(user("commentator"))).toBe(true);
    expect(canPlay(user("streamer", "referee"))).toBe(false);
    expect(canPlay(user("pool_tester"))).toBe(false);
    expect(canPlay(user("custom_mapper"))).toBe(false);
  });

  it("lets pool testers and custom mappers see the pool without editing it", () => {
    for (const role of ["pool_tester", "custom_mapper"] as const) {
      expect(canViewPool(user(role))).toBe(true);
      expect(hasRole(user(role), "map_pooler")).toBe(false);
    }
    expect(canViewPool(user("host"))).toBe(true);
    expect(canViewPool(user("map_pooler"))).toBe(true);
    expect(canViewPool(user("graphics_designer", "streamer"))).toBe(false);
    expect(canViewPool(user("commentator"))).toBe(false);
  });

  it("treats a signed-out visitor as nobody", () => {
    expect(hasRole(null, "host")).toBe(false);
    expect(isStaff(undefined)).toBe(false);
    expect(canPlay(null)).toBe(false);
  });
});

describe("admin pages", () => {
  it("open for staff, but not for those only listed on the staff page", () => {
    expect(canAccess(user("referee"), "staff")).toBe(true);
    expect(canAccess(user("graphics_designer", "commentator"), "staff")).toBe(false);
    expect(canAccess(user(), "staff")).toBe(false);
  });

  it("show the matches tab to hosts, map poolers, referees and streamers only", () => {
    for (const role of ["host", "map_pooler", "referee", "streamer"] as const) {
      expect(canAccess(user(role), "matches")).toBe(true);
    }
    expect(canAccess(user("pool_tester"), "matches")).toBe(false);
    expect(canAccess(user("pool_tester"), "pool")).toBe(true);
  });

  it("need the role itself, or Host", () => {
    expect(canAccess(user("referee"), "referee")).toBe(true);
    expect(canAccess(user("host"), "referee")).toBe(true);
    expect(canAccess(user("streamer"), "referee")).toBe(false);
  });
});
