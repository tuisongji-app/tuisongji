import { invoke } from "@tauri-apps/api/core";
import type { SubscriptionStatus, AppConfig, SoundInfo } from "@/types";

// ---- Subscriptions ----

export function addSubscription(uid: number): Promise<SubscriptionStatus> {
  return invoke("add_subscription", { uid });
}

export function removeSubscription(uid: number): Promise<void> {
  return invoke("remove_subscription", { uid });
}

export function listSubscriptions(): Promise<SubscriptionStatus[]> {
  return invoke("list_subscriptions");
}

export function refreshStatus(uid: number): Promise<SubscriptionStatus> {
  return invoke("refresh_status", { uid });
}

// ---- Poll / badge config ----

export function updatePollInterval(intervalMins: number): Promise<void> {
  return invoke("update_poll_interval", { intervalMins });
}

export function updateBadgeTimeout(timeoutMins: number): Promise<void> {
  return invoke("update_badge_timeout", { timeoutMins });
}

// ---- App config ----

export function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export function setAutostart(enabled: boolean): Promise<void> {
  return invoke("set_autostart", { enabled });
}

export function setShowWindowOnStartup(enabled: boolean): Promise<void> {
  return invoke("set_show_window_on_startup", { enabled });
}

// ---- Test ----

export function testTriggerStatus(
  uid: number,
  targetStatus: string,
): Promise<void> {
  return invoke("test_trigger_status", { uid, targetStatus });
}

// ---- Sound effects ----

export function downloadStreamerSounds(name: string): Promise<SoundInfo> {
  return invoke("download_streamer_sounds", { name });
}

export function getSoundInfo(name: string): Promise<SoundInfo> {
  return invoke("get_sound_info", { name });
}

export function playStreamerSound(
  name: string,
  eventType: string,
): Promise<void> {
  return invoke("play_streamer_sound", { name, eventType });
}

export function setSoundEnabled(enabled: boolean): Promise<void> {
  return invoke("set_sound_enabled", { enabled });
}

export function setSoundVolume(volume: number): Promise<void> {
  return invoke("set_sound_volume", { volume });
}
