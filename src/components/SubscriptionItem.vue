<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Trash2, ExternalLink, Volume2 } from "lucide-vue-next";
import { statusLabels, statusVariants } from "@/types";
import type { SubscriptionStatus, SoundInfo } from "@/types";
import { toast } from "vue-sonner";
import { getSoundInfo, downloadStreamerSounds, playStreamerSound } from "@/tauri";

const props = defineProps<{
  subscription: SubscriptionStatus;
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

const soundState = ref<SoundInfo | null>(null);
const soundLoading = ref(false);

onMounted(async () => {
  try {
    soundState.value = await getSoundInfo(props.subscription.name);
  } catch {
    // no sounds available or network error — leave as null
  }
});

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

function handlePreviewSound(eventType: string) {
  playStreamerSound(props.subscription.name, eventType).catch(() => {
    toast.error("音效播放失败");
  });
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
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-1.5 text-xs text-muted-foreground min-w-0">
            <Volume2 class="w-3.5 h-3.5 flex-shrink-0" />
            <template v-if="soundState.downloaded_live > 0 || soundState.downloaded_offline > 0">
              音效: {{ soundState.downloaded_live + soundState.downloaded_offline }}/{{ soundState.available_live + soundState.available_offline }} 已下载
            </template>
            <template v-else>
              有 {{ soundState.available_live + soundState.available_offline }} 个音效可用
            </template>
          </div>
          <div class="flex gap-1 flex-shrink-0">
            <Button
              v-if="hasUndownloaded"
              variant="outline"
              size="sm"
              class="h-7 text-xs"
              :disabled="soundLoading"
              @click="handleDownloadSounds"
            >
              {{ soundLoading ? '下载中...' : '下载音效' }}
            </Button>
            <Button
              v-if="soundState.downloaded_live > 0"
              variant="ghost"
              size="sm"
              class="h-7 text-xs"
              @click="handlePreviewSound('live')"
            >
              预览开播
            </Button>
            <Button
              v-if="soundState.downloaded_offline > 0"
              variant="ghost"
              size="sm"
              class="h-7 text-xs"
              @click="handlePreviewSound('offline')"
            >
              预览下播
            </Button>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
