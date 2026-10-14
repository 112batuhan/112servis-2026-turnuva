import { get, post } from "./client";
import type { NotificationList } from "./types";

export const fetchNotifications = () => get<NotificationList>("/api/notifications");
/** Just the bell's number, for polling. */
export const fetchNotificationCount = () => get<{ attention: number }>("/api/notifications/count");
export const markNotificationsRead = () => post<void>("/api/notifications/read");
/** Answers a confirm_time: the referee or streamer can make the new time. */
export const confirmNotification = (id: string) => post<void>(`/api/notifications/${id}/confirm`);
/** Answers a confirm_time: they can't, so they're taken off the match and the other staff are told. */
export const declineNotification = (id: string) => post<void>(`/api/notifications/${id}/decline`);
