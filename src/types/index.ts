export type LiveStatus = "offline" | "live" | "replay";

export interface SubscriptionStatus {
  uid: number;
  name: string;
  status: LiveStatus;
  title: string | null;
  room_id: number | null;
  avatar_url: string | null;
}

export interface SoundInfo {
  name: string;
  available_live: number;
  available_offline: number;
  downloaded_live: number;
  downloaded_offline: number;
}

export interface AppConfig {
  poll_interval_mins: number;
  badge_timeout_mins: number;
  autostart: boolean;
  show_window_on_startup: boolean;
  sound_enabled: boolean;
  sound_volume: number;
}

export const statusLabels: Record<LiveStatus, string> = {
  offline: "未开播",
  live: "直播中",
  replay: "轮播中",
};

export const statusVariants: Record<
  LiveStatus,
  "outline" | "success" | "warning"
> = {
  offline: "outline",
  live: "success",
  replay: "warning",
};
