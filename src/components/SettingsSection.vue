<script setup lang="ts">
import { ref, onMounted } from "vue";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { useBilibili } from "@/composables/useBilibili";
import { Settings, Bell, BellOff } from "lucide-vue-next";

const { getConfig, updatePollInterval } = useBilibili();
const interval = ref(60);
const saved = ref(false);
const saving = ref(false);
const permissionGranted = ref(false);

onMounted(async () => {
  try {
    const config = await getConfig();
    interval.value = config.poll_interval_secs;
  } catch {
    // use default
  }
  permissionGranted.value = await isPermissionGranted();
});

async function handleRequestPermission() {
  await requestPermission();
  permissionGranted.value = await isPermissionGranted();
}

async function saveInterval() {
  if (interval.value < 10) {
    interval.value = 10;
  }
  saving.value = true;
  try {
    await updatePollInterval(interval.value);
    saved.value = true;
    setTimeout(() => (saved.value = false), 2000);
  } catch {
    // silently fail
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Separator class="my-6" />

  <div>
    <div class="flex items-center gap-2 mb-4">
      <Settings class="w-4 h-4 text-muted-foreground" />
      <h2 class="text-sm font-semibold text-muted-foreground">设置</h2>
    </div>

    <!-- Notification permission -->
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-2">
        <Bell v-if="permissionGranted === true" class="w-4 h-4 text-green-500" />
        <BellOff v-else class="w-4 h-4 text-destructive" />
        <span class="text-sm">通知权限</span>
      </div>
      <div class="flex items-center gap-2">
        <Badge
          v-if="permissionGranted === true"
          variant="success"
          class="text-xs"
        >
          已授权
        </Badge>
        <Badge v-else variant="destructive" class="text-xs">
          未授权
        </Badge>
        <Button
          v-if="!permissionGranted"
          variant="outline"
          size="sm"
          @click="handleRequestPermission"
        >
          授权
        </Button>
      </div>
    </div>

    <!-- Poll interval -->
    <div class="flex items-end gap-2">
      <div class="flex-1">
        <Label for="interval" class="text-xs text-muted-foreground mb-1 block">
          轮询间隔 (秒)
        </Label>
        <Input
          id="interval"
          v-model.number="interval"
          type="number"
          min="10"
          class="w-24"
        />
      </div>
      <Button variant="outline" :disabled="saving" @click="saveInterval">
        {{ saved ? "已保存" : "保存" }}
      </Button>
    </div>
    <p class="text-xs text-muted-foreground mt-1.5">
      最小间隔10秒，默认60秒
    </p>
  </div>
</template>
