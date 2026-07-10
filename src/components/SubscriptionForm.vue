<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { addSubscription } from "@/tauri";
import { Plus } from "lucide-vue-next";
import { toast } from "vue-sonner";
import { subTypeLabels, subTypePlaceholders, subTypeRoomPlaceholders, inputModeLabels } from "@/types";
import type { SubscriptionStatus, SubType, InputMode } from "@/types";

const emit = defineEmits<{
  (e: "added", subscription: SubscriptionStatus): void;
}>();

const uid = ref("");
const subType = ref<SubType>("bilibili");
const inputMode = ref<InputMode>("uid");
const loading = ref(false);

const platforms: SubType[] = ["bilibili", "huya", "douyu"];
const inputModes: InputMode[] = ["uid", "room"];

const isDouyu = computed(() => subType.value === "douyu");

const placeholder = computed(() =>
  inputMode.value === "room" || isDouyu.value
    ? subTypeRoomPlaceholders[subType.value]
    : subTypePlaceholders[subType.value],
);

// 斗鱼仅支持房间号，选择斗鱼时自动切换
watch(subType, (newType) => {
  if (newType === "douyu") {
    inputMode.value = "room";
  }
});

async function handleAdd() {
  const parsed = parseInt(uid.value.trim(), 10);
  if (isNaN(parsed) || parsed <= 0) {
    toast.error("请输入有效的ID");
    return;
  }
  loading.value = true;
  try {
    const result = await addSubscription(parsed, subType.value, inputMode.value);
    uid.value = "";
    emit("added", result);
  } catch (e: unknown) {
    toast.error(typeof e === "string" ? e : "添加失败，请检查ID是否正确");
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="mb-6">
    <div class="flex gap-2 mb-4">
      <div class="flex rounded-md border border-input bg-transparent">
        <button
          v-for="p in platforms"
          :key="p"
          type="button"
          class="px-3 py-1.5 text-sm transition-colors cursor-pointer"
          :class="[
            subType === p
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:text-foreground',
            {
              'rounded-l-md': p === platforms[0],
              'rounded-r-md': p === platforms[platforms.length - 1],
            },
          ]"
          @click="subType = p"
        >
          {{ subTypeLabels[p] }}
        </button>
      </div>
      <div v-if="!isDouyu" class="flex rounded-md border border-input bg-transparent">
        <button
          v-for="m in inputModes"
          :key="m"
          type="button"
          class="px-3 py-1.5 text-sm transition-colors cursor-pointer"
          :class="[
            inputMode === m
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:text-foreground',
            {
              'rounded-l-md': m === inputModes[0],
              'rounded-r-md': m === inputModes[inputModes.length - 1],
            },
          ]"
          @click="inputMode = m"
        >
          {{ inputModeLabels[m] }}
        </button>
      </div>
    </div>
    <div class="flex gap-2">
      <Input
        v-model="uid"
        :placeholder="placeholder"
        class="flex-1"
        @keyup.enter="handleAdd"
      />
      <Button :disabled="loading" @click="handleAdd">
        <Plus v-if="!loading" class="w-4 h-4" />
        {{ loading ? "添加中..." : "添加" }}
      </Button>
    </div>
  </div>
</template>
