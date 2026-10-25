# 推送姬

![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)
![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0)

多平台直播状态监控与推送工具，支持 Windows / macOS 系统托盘常驻，开播/下播即时提醒。

### 支持平台

- **B站** (bilibili) — UID 或房间号
- **虎牙** (huya) — UID 或房间号
- **斗鱼** (douyu) — 房间号

## 下载

前往 [Releases](https://github.com/tuisongji-app/tuisongji/releases) 页面下载最新版本。

### 如何选择安装包

**macOS**

| 你的 Mac 芯片 | 下载文件 |
|---|---|
| Apple Silicon (M1/M2/M3/M4) | `tuisongji_*_aarch64.dmg` |
| Intel | `tuisongji_*_x64.dmg` |

> 不确定的话，点击左上角  →「关于本机」→ 查看「芯片」一栏。

**Windows**

| 你的 Windows 架构 | 下载文件 |
|---|---|
| x64 | `tuisongji_*_x64.exe` |
| ARM64 | `tuisongji_*_arm64.exe` |

> 不确定的话，按 `Win + R` 输入 `msinfo32`，查看「系统类型」一栏。显示 x64 则选 x64，显示 ARM64 则选 ARM64。

## 本地音效配置

每个主播都可以使用自定义音效，无需远端音效仓库收录。

1. 在订阅卡片的「开播」或「下播」旁点击「添加本地音频」，可一次选择多个文件。
2. 支持 MP3、WAV、OGG、FLAC，导入后点击文件名即可试听；同类多个音效会随机播放。
3. 音频会复制到应用数据目录，同名文件会自动改名，不会覆盖已有音效。

也可以点击文件夹图标，将音频手动放入 `live`（开播）或 `offline`（下播）子目录。返回应用后自动刷新，也可点击卡片上的刷新按钮。删除对应文件即可移除音效。

删除主播订阅时，会一并清理应用内的音效文件和头像缓存。如果还有同名主播订阅共用音效，则在最后一条同名订阅删除后清理音效。导入前的原始音频文件不受影响。

## 参与贡献

- [音效仓库](https://github.com/tuisongji-app/sound) — 投稿更多主播音效
