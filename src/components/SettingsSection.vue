<script setup lang="ts">
import { ref, onMounted } from "vue";
import { NumberField, NumberFieldContent, NumberFieldDecrement, NumberFieldIncrement, NumberFieldInput } from "@/components/ui/number-field";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import { Slider } from "@/components/ui/slider";
import {
  updatePollInterval,
  updateBadgeTimeout,
  getConfig,
  setAutostart,
  setShowWindowOnStartup,
  setSoundEnabled,
  setSoundVolume,
  restartApplication,
} from "@/tauri";
import { enable, disable } from "@tauri-apps/plugin-autostart";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { toast } from "vue-sonner";
import { Settings, RefreshCw, ExternalLink, Info } from "lucide-vue-next";

const interval = ref(30);
const badgeTimeout = ref(30);
const autostart = ref(false);
const showWindowOnStartup = ref(true);
const soundEnabled = ref(true);
const soundVolume = ref(0.8);

// ---- Update state ----
const appVersion = ref("");
const updateChecking = ref(false);
const updateVersion = ref("");
const updateBody = ref("");
const downloading = ref(false);
const installed = ref(false);

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

  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "0.0.0";
  }
});

function onIntervalChange(val: number) {
  if (val >= 1) updatePollInterval(val);
}

function onBadgeTimeoutChange(val: number) {
  if (val >= 1) updateBadgeTimeout(val);
}

async function onAutostartChange(val: boolean | "indeterminate") {
  if (typeof val !== "boolean") return;
  autostart.value = val;
  if (val) {
    await enable();
  } else {
    await disable();
  }
  await setAutostart(val);
}

async function onShowWindowChange(val: boolean | "indeterminate") {
  if (typeof val !== "boolean") return;
  showWindowOnStartup.value = val;
  await setShowWindowOnStartup(val);
}

async function onSoundEnabledChange(val: boolean | "indeterminate") {
  if (typeof val !== "boolean") return;
  soundEnabled.value = val;
  await setSoundEnabled(val);
}

async function onVolumeChange(val: number[] | undefined) {
  if (!val) return;
  const v = val[0] / 100;
  soundVolume.value = v;
  await setSoundVolume(v);
}

async function handleCheckUpdate() {
  updateChecking.value = true;
  updateVersion.value = "";
  updateBody.value = "";
  try {
    const update = await check();
    if (update) {
      updateVersion.value = update.version;
      updateBody.value = update.body ?? "";
    } else {
      toast.success("已是最新版本");
    }
  } catch (e) {
    toast.error("检查更新失败，请稍后重试");
  } finally {
    updateChecking.value = false;
  }
}

async function handleDownloadAndInstall() {
  downloading.value = true;
  try {
    const update = await check();
    if (update) {
      await update.downloadAndInstall();
      installed.value = true;
    }
  } catch (e) {
    toast.error("下载或安装更新失败，请稍后重试");
  } finally {
    downloading.value = false;
  }
}

async function handleRestart() {
  await restartApplication();
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
        <Checkbox
          :model-value="autostart"
          class="shrink-0"
          @update:model-value="onAutostartChange"
        />
      </div>

      <!-- 启动时显示窗口 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">启动时显示窗口</span>
          <p class="text-xs text-muted-foreground mt-0.5">应用启动时自动显示主窗口</p>
        </div>
        <Checkbox
          :model-value="showWindowOnStartup"
          class="shrink-0"
          @update:model-value="onShowWindowChange"
        />
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
        >
          <NumberFieldContent>
            <NumberFieldDecrement />
            <NumberFieldInput />
            <NumberFieldIncrement />
          </NumberFieldContent>
        </NumberField>
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
        >
          <NumberFieldContent>
            <NumberFieldDecrement />
            <NumberFieldInput />
            <NumberFieldIncrement />
          </NumberFieldContent>
        </NumberField>
      </div>

      <!-- 音效通知 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">音效通知</span>
          <p class="text-xs text-muted-foreground mt-0.5">开播/下播时播放提示音效</p>
        </div>
        <Checkbox
          :model-value="soundEnabled"
          class="shrink-0"
          @update:model-value="onSoundEnabledChange"
        />
      </div>

      <!-- 音量 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">音效音量</span>
          <p class="text-xs text-muted-foreground mt-0.5">控制音效播放的音量大小</p>
        </div>
        <Slider
          :model-value="[Math.round(soundVolume * 100)]"
          :min="0"
          :max="100"
          :step="1"
          :disabled="!soundEnabled"
          class="w-32 shrink-0"
          @update:model-value="onVolumeChange"
        />
      </div>

    </div>
  </div>

  <Separator class="my-6" />

  <!-- 关于 -->
  <div>
    <div class="flex items-center gap-2 mb-4">
      <Info class="w-4 h-4 text-muted-foreground" />
      <h2 class="text-sm font-semibold text-muted-foreground">关于</h2>
    </div>

    <div class="space-y-3">
      <!-- 版本更新 -->
      <div class="flex items-center justify-between">
        <div>
          <template v-if="installed">
            <span class="text-sm text-green-600 font-medium">更新已安装，重启后生效</span>
          </template>
          <template v-else>
            <span class="text-sm">{{ updateVersion ? '更新版本' : '当前版本' }}</span>
            <p class="text-xs text-muted-foreground mt-0.5">
              v{{ updateVersion || appVersion }}
            </p>
          </template>
        </div>
        <Button
          v-if="installed"
          variant="default"
          size="sm"
          class="h-8 text-xs shrink-0"
          @click="handleRestart"
        >
          立即重启
        </Button>
        <Button
          v-else-if="updateVersion"
          variant="default"
          size="sm"
          class="h-8 text-xs shrink-0"
          :disabled="downloading"
          @click="handleDownloadAndInstall"
        >
          {{ downloading ? '下载中...' : '下载' }}
        </Button>
        <Button
          v-else
          variant="outline"
          size="sm"
          class="h-8 text-xs shrink-0"
          :disabled="updateChecking"
          @click="handleCheckUpdate"
        >
          <RefreshCw
            :class="['w-3.5 h-3.5 mr-1', updateChecking && 'animate-spin']"
          />
          {{ updateChecking ? '检查中...' : '检查更新' }}
        </Button>
      </div>

      <!-- 项目链接 -->
      <div class="flex items-center justify-between">
        <span class="text-sm">项目仓库</span>
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8 shrink-0"
          @click="openUrl('https://github.com/tuisongji-app/tuisongji')"
        >
          <ExternalLink class="w-4 h-4" />
        </Button>
      </div>

      <!-- 音效仓库 -->
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm">音效仓库</span>
          <p class="text-xs text-muted-foreground mt-0.5">投稿更多音效</p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8 shrink-0"
          @click="openUrl('https://github.com/tuisongji-app/sound')"
        >
          <ExternalLink class="w-4 h-4" />
        </Button>
      </div>
    </div>
  </div>
</template>
