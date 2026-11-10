import { useEffect, useState } from "react";
import { fetchStaff } from "../api/staff";
import type { Role, StaffMember } from "../api/types";
import { sameName, type RoomSlot } from "./bancho";

// The staff who may be in a match's osu! room without playing in it, shared by the referee
// page (which kicks everyone else) and the public match page (which names them).

/** The roles that may be in a room without playing: the first one held names them there. */
const ROOM_ROLES: Role[] = ["referee", "streamer", "host"];

/** The roles a room's bot can make one of its referees (their tournament client then shows its chat). */
export const REF_ROLES: Role[] = ["referee", "streamer", "commentator"];

/**
 * Everyone holding one of ROOM_ROLES or REF_ROLES, loaded once per page; null until it arrives (or
 * if it fails), so nobody is taken for a stranger before the staff are known.
 */
export function useRoomStaff(): StaffMember[] | null {
  const [staff, setStaff] = useState<StaffMember[] | null>(null);
  useEffect(() => {
    let live = true;
    fetchStaff()
      .then(
        (all) =>
          live && setStaff(all.filter((s) => s.roles.some((r) => ROOM_ROLES.includes(r) || REF_ROLES.includes(r)))),
      )
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);
  return staff;
}

/** The role someone in the room is there as, if they're staff; by osu! id once it's known. */
export function staffRole(staff: StaffMember[] | null, slot: Pick<RoomSlot, "userId" | "username">): Role | null {
  const member = staff?.find((s) => (slot.userId !== null ? s.osu_id === slot.userId : sameName(s.username, slot.username)));
  return member ? (ROOM_ROLES.find((r) => member.roles.includes(r)) ?? null) : null;
}
