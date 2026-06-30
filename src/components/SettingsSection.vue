<script setup lang="ts">
import { ref, onMounted } from "vue";
import { NumberField } from "@/components/ui/number-field";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { useBilibili } from "@/composables/useBilibili";
import { Settings } from "lucide-vue-next";

const { getConfig, updatePollInterval, updateBadgeTimeout } = useBilibili();
const interval = ref(30);
const badgeTimeout = ref(30);

onMounted(async () => {
  try {
    const config = await getConfig();
    interval.value = config.poll_interval_mins;
    badgeTimeout.value = config.badge_timeout_mins;
  } catch {
    // use default
  }
});

function onIntervalChange(val: number) {
  if (val >= 1) updatePollInterval(val);
}

function onBadgeTimeoutChange(val: number) {
  if (val >= 1) updateBadgeTimeout(val);
}
</script>

<template>
  <Separator class="my-6" />

  <div>
    <div class="flex items-center gap-2 mb-4">
      <Settings class="w-4 h-4 text-muted-foreground" />
      <h2 class="text-sm font-semibold text-muted-foreground">设置</h2>
    </div>

    <div class="space-y-4">
      <div>
        <Label for="interval" class="text-xs text-muted-foreground mb-1 block">
          轮询间隔 (分钟)
        </Label>
        <NumberField
          id="interval"
          v-model="interval"
          :min="1"
          @update:model-value="onIntervalChange"
        />
        <p class="text-xs text-muted-foreground mt-1">每隔一段时间请求列表状态</p>
      </div>
      <div>
        <Label for="badge" class="text-xs text-muted-foreground mb-1 block">
          通知隐藏时间 (分钟)
        </Label>
        <NumberField
          id="badge"
          v-model="badgeTimeout"
          :min="1"
          @update:model-value="onBadgeTimeoutChange"
        />
        <p class="text-xs text-muted-foreground mt-1">托盘旁通知超时会自动隐藏</p>
      </div>
    </div>
  </div>
</template>
