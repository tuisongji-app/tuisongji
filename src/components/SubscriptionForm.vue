<script setup lang="ts">
import { ref } from "vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useBilibili } from "@/composables/useBilibili";
import { Plus } from "lucide-vue-next";
import type { SubscriptionStatus } from "@/types";

const emit = defineEmits<{
  (e: "added", subscription: SubscriptionStatus): void;
}>();

const uid = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const { addSubscription } = useBilibili();

async function handleAdd() {
  error.value = null;
  const parsed = parseInt(uid.value.trim(), 10);
  if (isNaN(parsed) || parsed <= 0) {
    error.value = "请输入有效的B站UID";
    return;
  }
  loading.value = true;
  try {
    const result = await addSubscription(parsed);
    uid.value = "";
    emit("added", result);
  } catch (e: unknown) {
    error.value = typeof e === "string" ? e : "添加失败，请检查UID是否正确";
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="mb-6">
    <div class="flex gap-2">
      <Input
        v-model="uid"
        placeholder="输入B站UP主的UID..."
        class="flex-1"
        @keyup.enter="handleAdd"
      />
      <Button :disabled="loading" @click="handleAdd">
        <Plus v-if="!loading" class="w-4 h-4" />
        {{ loading ? "添加中..." : "添加" }}
      </Button>
    </div>
    <p v-if="error" class="text-sm text-destructive mt-1.5">{{ error }}</p>
  </div>
</template>
