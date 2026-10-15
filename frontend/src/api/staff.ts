import { get } from "./client";
import type { StaffMember } from "./types";

export const fetchStaff = () => get<StaffMember[]>("/api/public/staff");
