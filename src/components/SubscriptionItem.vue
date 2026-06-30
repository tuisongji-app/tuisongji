<script setup lang="ts">
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Trash2, ExternalLink } from "lucide-vue-next";
import { statusLabels, statusVariants } from "@/types";
import type { SubscriptionStatus } from "@/types";

const props = defineProps<{
  subscription: SubscriptionStatus;
}>();

const emit = defineEmits<{
  (e: "remove", uid: number): void;
}>();

const avatarSrc = computed(() => {
  const url = props.subscription.avatar_url;
  if (!url) return null;
  return convertFileSrc(url);
});

function openRoom() {
  if (props.subscription.room_id) {
    openUrl(`https://live.bilibili.com/${props.subscription.room_id}`);
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
            @click="emit('remove', subscription.uid)"
          >
            <Trash2 class="w-4 h-4" />
          </Button>
        </div>
      </div>

    </CardContent>
  </Card>
</template>
