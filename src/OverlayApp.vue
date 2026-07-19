<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-vue-next";

// ---- types ----

interface ToastItem {
  room_id: number;
  name: string;
  action: string; // "开播" | "下播"
  avatar_path: string | null;
  sub_type: string; // "bilibili" | "huya" | "douyu"
}

// ---- state ----

const items = ref<ToastItem[]>([]);
const collapsed = ref(false);
const iconUrl = ref("");

// ---- actions ----

function dismiss(roomId: number) {
  invoke("dismiss_notif", { roomId });
}

function clearAll() {
  invoke("clear_all_notifs");
}

function openRoom(item: ToastItem) {
  invoke("open_notif_url", {
    roomId: item.room_id,
    subType: item.sub_type,
  });
}

async function expand() {
  collapsed.value = false;
  await getCurrentWindow().emit("toast-expand");
}

function avatarSrc(path: string | null): string | null {
  if (!path) return null;
  try {
    return convertFileSrc(path);
  } catch {
    return null;
  }
}

// ---- lifecycle ----

onMounted(async () => {
  // Load app icon for collapsed indicator.
  try {
    const bytes = await invoke<number[]>("get_app_icon");
    const blob = new Blob([new Uint8Array(bytes)], { type: "image/png" });
    iconUrl.value = URL.createObjectURL(blob);
  } catch { /* ignore */ }

  await listen<ToastItem[]>("toast-state", (event) => {
    items.value = event.payload;
    collapsed.value = false; // new notification → expand
  });
  await listen("toast-collapse", () => {
    collapsed.value = true;
  });
  await getCurrentWindow().emit("overlay-ready");
});
</script>

<template>
  <div class="overlay-root">
    <!-- Collapsed indicator -->
    <div
      v-if="collapsed && items.length > 1"
      class="collapse-bar"
      @click="expand"
    >
      <img v-if="iconUrl" :src="iconUrl" class="app-icon" />
      <span class="collapse-text">{{ items.length }} 条通知</span>
    </div>

    <!-- Normal card stack (not collapsed) -->
    <template v-if="!collapsed || items.length <= 1">
      <div v-if="items.length > 0" class="header-row">
        <button class="clear-all-btn" @click="clearAll">
          清空全部 ({{ items.length }})
        </button>
      </div>
      <TransitionGroup name="toast" tag="div" class="card-stack">
        <div
          v-for="(item, index) in items"
          :key="item.room_id"
          class="toast-card"
          :style="{ zIndex: items.length - index }"
          @click="openRoom(item)"
        >
          <img
            v-if="item.avatar_path"
            :src="avatarSrc(item.avatar_path)!"
            class="avatar-img"
            @error="($event.target as HTMLImageElement).style.display = 'none'"
          />
          <span class="name">{{ item.name }}</span>
          <span
            class="action-badge"
            :class="item.action === '开播' ? 'live' : 'offline'"
          >{{ item.action }}</span>
          <button
            class="dismiss-btn"
            @click.stop="dismiss(item.room_id)"
            :title="'关闭 ' + item.name"
          >
            <X class="h-3 w-3" />
          </button>
        </div>
      </TransitionGroup>
    </template>
  </div>
</template>

<style>
/* Global — transparent body so the window's empty pixels
   pass clicks through to windows underneath.
   Must override main.css Tailwind `body { @apply bg-background }`. */
html,
body {
  background: transparent !important;
  background-color: transparent !important;
  margin: 0;
  padding: 0;
  overflow: hidden;
  user-select: none;
}

#app {
  background: transparent !important;
}
</style>

<style scoped>
/* ---- outer container: transparent to clicks ---- */
.overlay-root {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  justify-content: flex-end;
  min-height: 100vh;
  width: 100%;
  padding: 8px;
  box-sizing: border-box;
  pointer-events: none;
}

/* ---- header ---- */
.header-row {
  pointer-events: auto;
  margin-bottom: 4px;
  flex-shrink: 0;
}

.clear-all-btn {
  background: rgba(30, 30, 30, 0.85);
  backdrop-filter: blur(8px);
  color: #9ca3af;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.clear-all-btn:hover {
  background: rgba(50, 50, 50, 0.9);
  color: #fff;
}

/* ---- card stack ---- */
.card-stack {
  display: flex;
  flex-direction: column;
  width: 100%;
  pointer-events: auto;
}

/* ---- collapsed indicator bar ---- */
.collapse-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 12px;
  background: rgba(20, 20, 24, 0.92);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  cursor: pointer;
  pointer-events: auto;
  transition: transform 0.15s, box-shadow 0.15s;
}
.collapse-bar:hover {
  transform: translateX(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  background: rgba(30, 30, 36, 0.95);
}

.app-icon {
  width: 24px;
  height: 24px;
  border-radius: 4px;
  flex-shrink: 0;
}

.collapse-text {
  color: #ccc;
  font-size: 13px;
  white-space: nowrap;
}

/* ---- individual card ---- */
.toast-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 64px;
  padding: 0 8px;
  background: rgb(34, 34, 34);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  cursor: pointer;
  flex-shrink: 0;
  transition: transform 0.2s ease, box-shadow 0.2s ease, background 0.2s ease,
    margin-bottom 0.2s ease;
}

/* Stacking: each subsequent card overlaps the previous by 28px,
   leaving only the top 36px visible. */
.toast-card + .toast-card {
  margin-top: -28px;
}

/* Hover: bring to front above all other cards. */
.toast-card:hover {
  z-index: 1000 !important;
  transform: translateX(-4px);
  margin-bottom: 28px;
}

/* ---- card internals ---- */
.avatar-img {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
}

.name {
  flex: 1;
  min-width: 0;
  color: #eee;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.action-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}
.action-badge.live {
  background: rgba(34, 197, 94, 0.2);
  color: #4ade80;
}
.action-badge.offline {
  background: rgba(234, 179, 8, 0.2);
  color: #fbbf24;
}

.expand-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.08);
  color: #999;
  cursor: pointer;
}

.dismiss-btn {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #666;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.dismiss-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

/* ---- Vue TransitionGroup animations ---- */
.toast-enter-active {
  animation: toast-in 0.3s ease-out;
}
.toast-leave-active {
  animation: toast-out 0.2s ease-in;
}
.toast-move {
  transition: transform 0.3s ease, margin-top 0.3s ease;
}

@keyframes toast-in {
  from {
    transform: translateX(120%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
@keyframes toast-out {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(120%);
    opacity: 0;
  }
}
</style>
