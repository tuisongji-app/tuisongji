import { invoke } from "@tauri-apps/api/core";
import type { SubscriptionStatus, AppConfig } from "@/types";

export function useBilibili() {
  async function addSubscription(uid: number): Promise<SubscriptionStatus> {
    return invoke("add_subscription", { uid });
  }

  async function removeSubscription(uid: number): Promise<void> {
    return invoke("remove_subscription", { uid });
  }

  async function listSubscriptions(): Promise<SubscriptionStatus[]> {
    return invoke("list_subscriptions");
  }

  async function refreshStatus(uid: number): Promise<SubscriptionStatus> {
    return invoke("refresh_status", { uid });
  }

  async function updatePollInterval(intervalMins: number): Promise<void> {
    return invoke("update_poll_interval", { intervalMins });
  }

  async function getConfig(): Promise<AppConfig> {
    return invoke("get_config");
  }

  async function updateBadgeTimeout(timeoutMins: number): Promise<void> {
    return invoke("update_badge_timeout", { timeoutMins });
  }

  async function testTriggerStatus(
    uid: number,
    targetStatus: string
  ): Promise<void> {
    return invoke("test_trigger_status", { uid, targetStatus });
  }

  return {
    addSubscription,
    removeSubscription,
    listSubscriptions,
    refreshStatus,
    updatePollInterval,
    getConfig,
    updateBadgeTimeout,
    testTriggerStatus,
  };
}
