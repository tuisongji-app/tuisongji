<script setup lang="ts">
import SubscriptionItem from "./SubscriptionItem.vue";
import type { SubscriptionStatus } from "@/types";

defineProps<{
  subscriptions: SubscriptionStatus[];
}>();

const emit = defineEmits<{
  (e: "removed", uid: number): void;
}>();
</script>

<template>
  <div class="mb-6">
    <h2 class="text-sm font-semibold text-muted-foreground mb-3">订阅列表</h2>

    <div
      v-if="subscriptions.length === 0"
      class="text-center py-8 text-muted-foreground"
    >
      <p>还没有订阅任何UP主</p>
      <p class="text-xs mt-1">在上方输入B站UID来添加订阅</p>
    </div>

    <TransitionGroup name="list" tag="div">
      <SubscriptionItem
        v-for="sub in subscriptions"
        :key="sub.uid"
        :subscription="sub"
        @remove="emit('removed', $event)"
      />
    </TransitionGroup>
  </div>
</template>

<style scoped>
.list-enter-active,
.list-leave-active {
  transition: all 0.3s ease;
}
.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateX(20px);
}
</style>
