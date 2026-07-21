declare namespace NodeJS {
    interface ProcessEnv {
        readonly TAURI_DEV_HOST: string
        readonly TAURI_ENV_PLATFORM: string
    }
}