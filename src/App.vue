<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import AppHeader from "./components/AppHeader.vue";
import SubscriptionForm from "./components/SubscriptionForm.vue";
import SubscriptionList from "./components/SubscriptionList.vue";
import SettingsSection from "./components/SettingsSection.vue";
import TestPanel from "./components/TestPanel.vue";
import { useBilibili } from "./composables/useBilibili";
import type { SubscriptionStatus } from "./types";

const { listSubscriptions, removeSubscription, requestNotificationPermission } =
  useBilibili();
const subscriptions = ref<SubscriptionStatus[]>([]);
const loading = ref(true);

onMounted(async () => {
  try {
    await requestNotificationPermission();
  } catch {
    // notification permission prompt may fail silently
  }

  try {
    subscriptions.value = await listSubscriptions();
  } catch {
    // fetch failed
  } finally {
    loading.value = false;
  }

  // Listen for real-time status changes from backend poller
  await listen<SubscriptionStatus>("status-changed", (event) => {
      const updated = event.payload;
      const idx = subscriptions.value.findIndex((s) => s.uid === updated.uid);
      if (idx >= 0) {
        subscriptions.value[idx] = updated;
      }
    }
  );
});

function onSubscriptionAdded(sub: SubscriptionStatus) {
  subscriptions.value.push(sub);
}

async function onSubscriptionRemoved(uid: number) {
  try {
    await removeSubscription(uid);
    subscriptions.value = subscriptions.value.filter((s) => s.uid !== uid);
  } catch {
    // handle silently
  }
}
</script>

<template>
  <div class="min-h-screen bg-background">
    <div class="max-w-lg mx-auto p-6">
      <AppHeader :count="subscriptions.length" />
      <SubscriptionForm @added="onSubscriptionAdded" />

      <div v-if="loading" class="text-center py-8 text-muted-foreground">
        加载中...
      </div>

      <SubscriptionList
        v-else
        :subscriptions="subscriptions"
        @removed="onSubscriptionRemoved"
      />

      <SettingsSection />
      <TestPanel :subscriptions="subscriptions" />
    </div>
  </div>
</template>
