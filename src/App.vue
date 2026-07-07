<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { Toaster } from 'vue-sonner'

import SubscriptionForm from "./components/SubscriptionForm.vue";
import SubscriptionList from "./components/SubscriptionList.vue";
import SettingsSection from "./components/SettingsSection.vue";
// import TestPanel from "./components/TestPanel.vue";
import { listSubscriptions, removeSubscription } from "@/tauri";
import type { SubscriptionStatus } from "./types";

const subscriptions = ref<SubscriptionStatus[]>([]);
const loading = ref(true);

onMounted(async () => {
  try {
    subscriptions.value = await listSubscriptions();
  } catch {
    // fetch failed
  } finally {
    loading.value = false;
  }

  await listen<SubscriptionStatus>("status-changed", (event) => {
    const updated = event.payload;
    const idx = subscriptions.value.findIndex(
      (s) => s.uid === updated.uid && s.sub_type === updated.sub_type
    );
    if (idx >= 0) {
      subscriptions.value[idx] = updated;
    }
  });
});

function onSubscriptionAdded(sub: SubscriptionStatus) {
  subscriptions.value.push(sub);
}

async function onSubscriptionRemoved(uid: number, subType: string) {
  try {
    await removeSubscription(uid, subType);
    subscriptions.value = subscriptions.value.filter(
      (s) => !(s.uid === uid && s.sub_type === subType)
    );
  } catch {
    // handle silently
  }
}
</script>

<template>
  <div class="min-h-screen bg-background">
    <Toaster position="top-center" richColors />
    <div class="max-w-lg mx-auto p-6">
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
      <!-- <TestPanel :subscriptions="subscriptions" /> -->
    </div>
  </div>
</template>
