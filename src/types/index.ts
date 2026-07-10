export type LiveStatus = "offline" | "live" | "replay";

export type SubType = "bilibili" | "huya" | "douyu";

export type InputMode = "uid" | "room";

export const inputModeLabels: Record<InputMode, string> = {
  uid: "UID",
  room: "房间号",
};

export const subTypeLabels: Record<SubType, string> = {
  bilibili: "B站",
  huya: "虎牙",
  douyu: "斗鱼",
};

export const subTypePlaceholders: Record<SubType, string> = {
  bilibili: "输入B站UP主的UID...",
  huya: "输入虎牙用户UID...",
  douyu: "输入斗鱼房间号...",
};

export const subTypeRoomPlaceholders: Record<SubType, string> = {
  bilibili: "输入B站直播间房间号...",
  huya: "输入虎牙房间号...",
  douyu: "输入斗鱼房间号...",
};

export interface SubscriptionStatus {
  uid: number;
  sub_type: string;
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
  live_files: string[];
  offline_files: string[];
  downloaded_live_files: string[];
  downloaded_offline_files: string[];
}

export interface AppConfig {
  poll_interval_mins: number;
  badge_timeout_mins: number;
  autostart: boolean;
  show_window_on_startup: boolean;
  sound_enabled: boolean;
  sound_volume: number;
  auto_check_update: boolean;
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
