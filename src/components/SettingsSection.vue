<script setup lang="ts">
import { ref, onMounted } from "vue";
import { NumberField } from "@/components/ui/number-field";
import { Separator } from "@/components/ui/separator";
import {
  updatePollInterval,
  updateBadgeTimeout,
  getConfig,
  setAutostart,
  setShowWindowOnStartup,
  setSoundEnabled,
  setSoundVolume,
} from "@/tauri";
import { enable, disable } from "@tauri-apps/plugin-autostart";
import { Settings } from "lucide-vue-next";

const interval = ref(30);
const badgeTimeout = ref(30);
const autostart = ref(false);
const showWindowOnStartup = ref(true);
const soundEnabled = ref(true);
const soundVolume = ref(0.8);

onMounted(async () => {
  try {
    const config = await getConfig();
    interval.value = config.poll_interval_mins;
    badgeTimeout.value = config.badge_timeout_mins;
    autostart.value = config.autostart;
    showWindowOnStartup.value = config.show_window_on_startup;
    soundEnabled.value = config.sound_enabled;
    soundVolume.value = config.sound_volume;
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

async function onAutostartChange(val: boolean) {
  autostart.value = val;
  if (val) {
    await enable();
  } else {
    await disable();
  }
  await setAutostart(val);
}

async function onShowWindowChange(val: boolean) {
  showWindowOnStartup.value = val;
  await setShowWindowOnStartup(val);
}

async function onSoundEnabledChange(val: boolean) {
  soundEnabled.value = val;
  await setSoundEnabled(val);
}

async function onVolumeChange(e: Event) {
  const val = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  soundVolume.value = val;
  await setSoundVolume(val);
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
      <!-- 开机自启 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">开机自启</span>
          <p class="text-xs text-muted-foreground mt-0.5">登录系统后自动启动应用</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer shrink-0">
          <input
            type="checkbox"
            class="sr-only peer"
            :checked="autostart"
            @change="onAutostartChange(($event.target as HTMLInputElement).checked)"
          />
          <div class="w-9 h-5 bg-input rounded-full peer peer-checked:bg-primary peer-focus:ring-2 peer-focus:ring-ring transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-transform peer-checked:after:translate-x-4" />
        </label>
      </div>

      <!-- 启动时显示窗口 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">启动时显示窗口</span>
          <p class="text-xs text-muted-foreground mt-0.5">应用启动时自动显示主窗口</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer shrink-0">
          <input
            type="checkbox"
            class="sr-only peer"
            :checked="showWindowOnStartup"
            @change="onShowWindowChange(($event.target as HTMLInputElement).checked)"
          />
          <div class="w-9 h-5 bg-input rounded-full peer peer-checked:bg-primary peer-focus:ring-2 peer-focus:ring-ring transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-transform peer-checked:after:translate-x-4" />
        </label>
      </div>

      <!-- 轮询间隔 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">轮询间隔 (分钟)</span>
          <p class="text-xs text-muted-foreground mt-0.5">每隔一段时间请求列表状态</p>
        </div>
        <NumberField
          id="interval"
          v-model="interval"
          class="w-32 shrink-0"
          :min="1"
          @update:model-value="onIntervalChange"
        />
      </div>

      <!-- 通知隐藏时间 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">通知隐藏时间 (分钟)</span>
          <p class="text-xs text-muted-foreground mt-0.5">托盘旁通知超时会自动隐藏</p>
        </div>
        <NumberField
          id="badge"
          v-model="badgeTimeout"
          class="w-32 shrink-0"
          :min="1"
          @update:model-value="onBadgeTimeoutChange"
        />
      </div>

      <!-- 音效通知 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">音效通知</span>
          <p class="text-xs text-muted-foreground mt-0.5">开播/下播时播放提示音效</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer shrink-0">
          <input
            type="checkbox"
            class="sr-only peer"
            :checked="soundEnabled"
            @change="onSoundEnabledChange(($event.target as HTMLInputElement).checked)"
          />
          <div class="w-9 h-5 bg-input rounded-full peer peer-checked:bg-primary peer-focus:ring-2 peer-focus:ring-ring transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-transform peer-checked:after:translate-x-4" />
        </label>
      </div>

      <!-- 音量 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">音效音量</span>
          <p class="text-xs text-muted-foreground mt-0.5">控制音效播放的音量大小</p>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          :value="Math.round(soundVolume * 100)"
          :disabled="!soundEnabled"
          @input="onVolumeChange"
          class="w-32 h-2 bg-input rounded-lg appearance-none cursor-pointer shrink-0 sound-slider"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.sound-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: hsl(var(--primary));
  cursor: pointer;
}
.sound-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: none;
  background: hsl(var(--primary));
  cursor: pointer;
}
.sound-slider:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
