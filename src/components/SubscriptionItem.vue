<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Trash2, ExternalLink, Volume2, Play, Download, FolderOpen, Plus, RefreshCw } from "lucide-vue-next";
import { statusLabels, statusVariants } from "@/types";
import type { SubscriptionStatus, SoundInfo } from "@/types";
import { toast } from "vue-sonner";
import { downloadStreamerSounds, getSoundInfo, importSoundFile, playSoundFile, openSoundsDir } from "@/tauri";

const props = defineProps<{
  subscription: SubscriptionStatus;
  soundInfo: SoundInfo | null;
}>();

const emit = defineEmits<{
  (e: "remove", uid: number, subType: string): void;
}>();

const avatarSrc = computed(() => {
  const url = props.subscription.avatar_url;
  if (!url) return null;
  return convertFileSrc(url);
});

// ---- Sound state ----

const soundState = ref<SoundInfo | null>(props.soundInfo);
const soundLoading = ref(false);
const importing = ref(false);
const localRefreshing = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const importEventType = ref<"live" | "offline">("live");

watch(() => props.soundInfo, (val) => {
  soundState.value = val;
});

async function refreshLocalSounds() {
  const name = props.subscription.name;
  localRefreshing.value = true;
  try {
    const info = await getSoundInfo(name);
    if (props.subscription.name === name) soundState.value = info;
  } catch {
    toast.error("读取本地音效失败，请点击刷新重试");
  } finally {
    localRefreshing.value = false;
  }
}

watch(() => props.subscription.name, refreshLocalSounds, { immediate: true });
onMounted(() => window.addEventListener("focus", refreshLocalSounds));
onUnmounted(() => window.removeEventListener("focus", refreshLocalSounds));

const soundGroups = computed(() => [
  { key: "live" as const, label: "开播", files: soundState.value?.live_files ?? [] },
  { key: "offline" as const, label: "下播", files: soundState.value?.offline_files ?? [] },
]);

const localCount = computed(() =>
  (soundState.value?.downloaded_live ?? 0) + (soundState.value?.downloaded_offline ?? 0)
);

function chooseLocalFiles(eventType: "live" | "offline") {
  importEventType.value = eventType;
  fileInput.value?.click();
}

async function handleImportFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  if (files.length === 0) return;

  const name = props.subscription.name;
  const eventType = importEventType.value;
  importing.value = true;
  let imported = 0;
  let failed = 0;
  let firstError = "";
  try {
    for (const file of files) {
      try {
        const data = Array.from(new Uint8Array(await file.arrayBuffer()));
        const info = await importSoundFile(name, eventType, file.name, data);
        if (props.subscription.name === name) soundState.value = info;
        imported++;
      } catch (error) {
        failed++;
        if (!firstError) firstError = typeof error === "string" ? error : `${file.name} 导入失败`;
      }
    }
    if (failed > 0) {
      toast.error(`已导入 ${imported} 个，${failed} 个失败：${firstError}`);
    } else {
      toast.success(`已导入 ${imported} 个${eventType === "live" ? "开播" : "下播"}音效`);
    }
  } finally {
    importing.value = false;
    input.value = "";
  }
}

async function handleOpenSoundsDir() {
  try {
    await openSoundsDir(props.subscription.name);
  } catch {
    toast.error("打开音效文件夹失败");
  }
}

function isDownloaded(eventType: string, filename: string): boolean {
  if (!soundState.value) return false;
  const files = eventType === "live"
    ? soundState.value.downloaded_live_files
    : soundState.value.downloaded_offline_files;
  return files.includes(filename);
}

function handlePlayFile(eventType: string, filename: string) {
  playSoundFile(props.subscription.name, eventType, filename).catch(() => {
    toast.error("音效播放失败");
  });
}

async function handleDownloadSounds() {
  soundLoading.value = true;
  try {
    soundState.value = await downloadStreamerSounds(props.subscription.name);
  } catch (e) {
    toast.error("音效下载失败，请稍后重试");
  } finally {
    soundLoading.value = false;
  }
}

const hasUndownloaded = computed(() => {
  if (!soundState.value) return false;
  return (
    soundState.value.downloaded_live < soundState.value.available_live ||
    soundState.value.downloaded_offline < soundState.value.available_offline
  );
});

function openRoom() {
  if (props.subscription.room_id) {
    const subType = props.subscription.sub_type || "bilibili";
    let url: string;
    if (subType === "huya") {
      url = `https://www.huya.com/${props.subscription.room_id}`;
    } else if (subType === "douyu") {
      url = `https://www.douyu.com/${props.subscription.room_id}`;
    } else {
      url = `https://live.bilibili.com/${props.subscription.room_id}`;
    }
    openUrl(url);
  }
}
</script>

<template>
  <Card class="mb-3">
    <CardContent class="pt-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3 min-w-0">
          <img
            v-if="avatarSrc"
            :src="avatarSrc"
            class="w-10 h-10 rounded-full flex-shrink-0"
          />
          <div class="min-w-0">
            <div class="font-medium truncate">{{ subscription.name }}</div>
          </div>
        </div>

        <div class="flex items-center gap-2 flex-shrink-0">
          <Badge :variant="statusVariants[subscription.status]">
            {{ statusLabels[subscription.status] }}
          </Badge>

          <Button
            v-if="subscription.room_id"
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            @click="openRoom"
          >
            <ExternalLink class="w-4 h-4" />
          </Button>

          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8 text-destructive hover:text-destructive"
            @click="emit('remove', subscription.uid, subscription.sub_type)"
          >
            <Trash2 class="w-4 h-4" />
          </Button>
        </div>
      </div>


      <!-- Sound effects -->
      <div class="mt-3 pt-3 border-t border-border">
        <input
          ref="fileInput"
          type="file"
          accept=".mp3,.wav,.ogg,.flac"
          multiple
          class="hidden"
          @change="handleImportFiles"
        />
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Volume2 class="w-3.5 h-3.5 flex-shrink-0" />
            <span>音效 · 本地 {{ localCount }} 个</span>
          </div>
          <div class="flex gap-1">
            <Button
              v-if="hasUndownloaded"
              variant="outline"
              size="sm"
              class="h-7 text-xs"
              :disabled="soundLoading || importing"
              @click="handleDownloadSounds"
            >
              <Download class="w-3 h-3 mr-1" />
              {{ soundLoading ? "下载中..." : "下载远端音效" }}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7"
              title="刷新本地音效"
              aria-label="刷新本地音效"
              :disabled="localRefreshing || importing || soundLoading"
              @click="refreshLocalSounds"
            >
              <RefreshCw :class="['w-3.5 h-3.5', localRefreshing && 'animate-spin']" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7"
              title="打开音效文件夹"
              aria-label="打开音效文件夹"
              @click="handleOpenSoundsDir"
            >
              <FolderOpen class="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        <!-- File list grouped by event type -->
        <div v-for="group in soundGroups" :key="group.key" class="mb-2">
          <div class="flex items-center justify-between">
            <span class="text-xs text-muted-foreground">{{ group.label }}</span>
            <Button
              variant="ghost"
              size="sm"
              class="h-6 px-1.5 text-xs"
              :disabled="importing || soundLoading"
              @click="chooseLocalFiles(group.key)"
            >
              <Plus class="w-3 h-3 mr-1" />
              {{ importing && importEventType === group.key ? "导入中..." : "添加本地音频" }}
            </Button>
          </div>
          <div v-if="group.files.length > 0" class="flex flex-wrap gap-1 mt-0.5">
            <button
              v-for="file in group.files"
              :key="file"
              :disabled="!isDownloaded(group.key, file)"
              class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs transition-colors"
              :class="isDownloaded(group.key, file)
                ? 'bg-secondary hover:bg-secondary/80 cursor-pointer'
                : 'text-muted-foreground/40 cursor-default'"
              @click="handlePlayFile(group.key, file)"
            >
              <Play class="w-2.5 h-2.5" />
              {{ file }}
            </button>
          </div>
          <p v-else class="text-xs text-muted-foreground/70 mt-0.5">暂未配置{{ group.label }}音效</p>
        </div>
        <p class="text-xs text-muted-foreground/70">
          支持 MP3、WAV、OGG、FLAC；同类多个音效随机播放。
        </p>
        <details class="mt-1 text-xs text-muted-foreground/70">
          <summary class="cursor-pointer">通过文件夹配置</summary>
          <p class="mt-1">打开音效文件夹，将音频放入 live（开播）或 offline（下播）。返回后自动刷新，也可点击刷新按钮。移除音效时删除对应文件即可。</p>
        </details>
      </div>
    </CardContent>
  </Card>
</template>
