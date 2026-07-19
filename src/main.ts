import { createApp } from "vue";
import 'vue-sonner/style.css'
import "./assets/main.css";

async function mount() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const isOverlay = getCurrentWindow().label === "toast-overlay";

  if (isOverlay) {
    const { default: OverlayApp } = await import("./OverlayApp.vue");
    const app = createApp(OverlayApp);
    app.mount("#app");
  } else {
    const { default: App } = await import("./App.vue");
    const app = createApp(App);
    app.mount("#app");
  }
}

mount();
