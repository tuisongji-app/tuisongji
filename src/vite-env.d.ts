/// <reference types="vite/client" />

declare const __IS_MACOS__: boolean;
declare const __IS_WINDOWS__: boolean;

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
