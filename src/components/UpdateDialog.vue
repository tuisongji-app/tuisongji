<script setup lang="ts">
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

defineProps<{
  open: boolean;
  version: string;
  body: string | null;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "navigate"): void;
}>();
</script>

<template>
  <Dialog :open="open" @update:open="(v) => !v && emit('close')">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>发现新版本 v{{ version }}</DialogTitle>
        <DialogDescription v-if="body" class="max-h-40 overflow-y-auto whitespace-pre-wrap text-sm">
          {{ body }}
        </DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button variant="outline" @click="emit('close')">稍后更新</Button>
        <Button @click="emit('navigate')">前往下载</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
