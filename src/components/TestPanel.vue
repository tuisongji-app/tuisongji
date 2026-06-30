<script setup lang="ts">
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { useBilibili } from "@/composables/useBilibili";
import { statusLabels } from "@/types";
import { FlaskConical } from "lucide-vue-next";
import type { SubscriptionStatus } from "@/types";

defineProps<{
  subscriptions: SubscriptionStatus[];
}>();

const { testTriggerStatus } = useBilibili();

const statusActions = [
  { key: "live", label: "开播", variant: "default" as const },
  { key: "offline", label: "下播", variant: "outline" as const },
  { key: "replay", label: "轮播", variant: "secondary" as const },
];

function trigger(uid: number, target: string) {
  testTriggerStatus(uid, target).catch(console.error);
}
</script>

<template>
  <Separator class="my-6" />
  <Card>
    <CardHeader>
      <div class="flex items-center gap-2">
        <FlaskConical class="w-4 h-4 text-muted-foreground" />
        <CardTitle class="text-sm">测试面板</CardTitle>
      </div>
      <CardDescription>手动模拟直播状态变化，测试通知功能</CardDescription>
    </CardHeader>
    <CardContent>
      <div v-if="subscriptions.length === 0" class="text-sm text-muted-foreground">
        暂无订阅，先添加一个再测试
      </div>
      <div v-for="sub in subscriptions" :key="sub.uid" class="mb-3 last:mb-0">
        <div class="text-sm font-medium mb-1.5">
          {{ sub.name }}
          <span class="text-xs text-muted-foreground">(当前: {{ statusLabels[sub.status] }})</span>
        </div>
        <div class="flex gap-2">
          <Button
            v-for="action in statusActions"
            :key="action.key"
            :variant="action.variant"
            size="sm"
            @click="trigger(sub.uid, action.key)"
          >
            模拟 {{ action.label }}
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
