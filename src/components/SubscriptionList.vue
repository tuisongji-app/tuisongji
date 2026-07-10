<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { Button } from "@/components/ui/button";
import { RefreshCw } from "lucide-vue-next";
import SubscriptionItem from "./SubscriptionItem.vue";
import { refreshStatus, refreshSoundManifest, getSoundInfo } from "@/tauri";
import type { SubscriptionStatus, SoundInfo } from "@/types";

const props = defineProps<{
  subscriptions: SubscriptionStatus[];
}>();

const emit = defineEmits<{
  (e: "removed", uid: number, subType: string): void;
  (e: "update:subscriptions", subs: SubscriptionStatus[]): void;
}>();

const leaving = ref(0);
const refreshing = ref(false);

// 音效数据由外层统一获取，key = `${sub_type}:${uid}`
const soundStates = reactive<Record<string, SoundInfo | null>>({});

function soundKey(sub: SubscriptionStatus) {
  return `${sub.sub_type}:${sub.uid}`;
}

async function fetchAllSoundInfos() {
  // 先刷新 manifest 缓存（一次网络请求），再并行取各项音效（命中缓存）
  await refreshSoundManifest().catch(() => {});
  const results = await Promise.allSettled(
    props.subscriptions.map((sub) => getSoundInfo(sub.name))
  );
  props.subscriptions.forEach((sub, i) => {
    const result = results[i];
    soundStates[soundKey(sub)] =
      result.status === "fulfilled" ? result.value : null;
  });
}

onMounted(fetchAllSoundInfos);

async function handleRefresh() {
  refreshing.value = true;
  const minDuration = new Promise((r) => setTimeout(r, 300));

  await fetchAllSoundInfos();

  const results = await Promise.allSettled(
    props.subscriptions.map((sub) => refreshStatus(sub.uid, sub.sub_type))
  );
  const updated = props.subscriptions.map((sub, i) => {
    if (results[i].status === "fulfilled") return results[i].value;
    return sub;
  });
  await minDuration;
  refreshing.value = false;
  emit("update:subscriptions", updated);
}
</script>

<template>
  <div class="mb-6">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-semibold text-muted-foreground">订阅列表</h2>
      <Button
        v-if="subscriptions.length > 0"
        variant="ghost"
        size="icon"
        class="h-7 w-7"
        :disabled="refreshing"
        @click="handleRefresh"
      >
        <RefreshCw :class="['w-3.5 h-3.5', refreshing && 'animate-spin']" />
      </Button>
    </div>

    <div
      v-if="subscriptions.length === 0 && leaving === 0"
      class="text-center py-8 text-muted-foreground"
    >
      <p>还没有订阅任何主播</p>
      <p class="text-xs mt-1">在上方选择平台并输入ID来添加订阅</p>
    </div>

    <TransitionGroup
      name="list"
      tag="div"
      @before-leave="leaving++"
      @after-leave="leaving--"
    >
      <SubscriptionItem
        v-for="sub in subscriptions"
        :key="`${sub.sub_type}:${sub.uid}`"
        :subscription="sub"
        :sound-info="soundStates[`${sub.sub_type}:${sub.uid}`] ?? null"
        @remove="(uid: number, subType: string) => emit('removed', uid, subType)"
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
