<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Trash2, ExternalLink, Volume2, Play, Download, FolderOpen } from "lucide-vue-next";
import { statusLabels, statusVariants } from "@/types";
import type { SubscriptionStatus, SoundInfo } from "@/types";
import { toast } from "vue-sonner";
import { downloadStreamerSounds, playSoundFile, openSoundsDir } from "@/tauri";

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

// ---- Sound state (初始化自 prop，下载后本地更新) ----

const soundState = ref<SoundInfo | null>(props.soundInfo);
const soundLoading = ref(false);

watch(() => props.soundInfo, (val) => {
  soundState.value = val;
});

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
      <div
        v-if="soundState !== null && (soundState.available_live > 0 || soundState.available_offline > 0)"
        class="mt-3 pt-3 border-t border-border"
      >
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Volume2 class="w-3.5 h-3.5 flex-shrink-0" />
            <span v-if="soundState.downloaded_live + soundState.downloaded_offline > 0">
              音效: {{ soundState.downloaded_live + soundState.downloaded_offline }}/{{ soundState.available_live + soundState.available_offline }} 已下载
            </span>
            <span v-else>
              有 {{ soundState.available_live + soundState.available_offline }} 个音效可用
            </span>
          </div>
          <div class="flex gap-1">
            <Button
              v-if="hasUndownloaded"
              variant="outline"
              size="sm"
              class="h-7 text-xs"
              :disabled="soundLoading"
              @click="handleDownloadSounds"
            >
              <Download class="w-3 h-3 mr-1" />
              {{ soundLoading ? "下载中..." : "下载全部" }}
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7"
              title="打开音效文件夹"
              @click="openSoundsDir(subscription.name)"
            >
              <FolderOpen class="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        <!-- File list grouped by event type -->
        <div
          v-for="(files, eventType) in { 开播: soundState.live_files, 下播: soundState.offline_files }"
          :key="eventType"
        >
          <div v-if="files.length > 0" class="mb-1">
            <span class="text-xs text-muted-foreground/70">{{ eventType }}</span>
            <div class="flex flex-wrap gap-1 mt-0.5">
              <button
                v-for="file in files"
                :key="file"
                :disabled="!isDownloaded(eventType === '开播' ? 'live' : 'offline', file)"
                class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs transition-colors"
                :class="isDownloaded(eventType === '开播' ? 'live' : 'offline', file)
                  ? 'bg-secondary hover:bg-secondary/80 cursor-pointer'
                  : 'text-muted-foreground/40 cursor-default'"
                @click="handlePlayFile(eventType === '开播' ? 'live' : 'offline', file)"
              >
                <Play class="w-2.5 h-2.5" />
                {{ file }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
