<script setup lang="ts">
import { Badge } from "@/components/ui/badge";
import { Tv } from "lucide-vue-next";

defineProps<{
  count: number;
}>();
</script>

<template>
  <div class="flex items-center justify-between mb-6">
    <div class="flex items-center gap-2">
      <Tv class="w-6 h-6 text-primary" />
      <h1 class="text-xl font-bold">推送姬</h1>
    </div>
    <Badge variant="secondary">
      {{ count }} 个订阅
    </Badge>
  </div>
</template>
