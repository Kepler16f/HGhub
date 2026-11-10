# HGhub macOS 适配与构建指南

本项目基于 **Rust + Tauri 2 + React 19**，已实现针对 macOS 桌面系统的深度适配。

---

## 1. macOS 核心适配机制

### 1.1 窗口与系统红绿灯垂直居中
- **原生沉浸式标题栏**：macOS 采用 `titleBarStyle: Overlay` 与 `hiddenTitle: true`（配置见 `src-tauri/tauri.macos.conf.json`）。
- **动态测量对齐**：不同 macOS 系统版本（Sonoma / Sequoia 等）以及不同屏幕缩放比例下，红绿灯按钮的底边偏移量存在差异。
  在 `src-tauri/src/macos_titlebar.rs` 中，通过 `objc2` 动态调用 AppKit 的 `NSWindow` 及 `standardWindowButton(CloseButton)` 实时测量按钮几何尺寸，计算出精确的垂直居中偏移量并应用到窗口。
- **全屏与自绘控制区避让**：在全屏状态下，系统红绿灯会自动隐藏；前端 `src/components/DesktopChrome.tsx` 监听全屏事件并动态收回左侧让位边距（84px -> 14px）。

### 1.2 自定义网络协议与流媒体代理
- **URL Scheme 适配**：
  - Windows/Android 使用 `http://hg.localhost`；
  - macOS/iOS/Linux 使用 `hg://localhost`。
  - 前端 `src/api.ts` 与 Rust 侧 `media::LOCAL_ORIGIN` 均已按平台生成基址，确保 WebKit 引擎下 m3u8 切片解密与代理转发正常工作。
- **MSE / hls.js 兼容性**：前端在 Safari/WebKit 下开启标准 MSE，由 hls.js 处理流媒体缓冲与续播逻辑。

### 1.3 Ad-hoc 签名
- 在 `src-tauri/tauri.macos.conf.json` 中配置了 `"signingIdentity": "-"`，Tauri 打包时会自动对 Mach-O 可执行文件与 Info.plist 完成 ad-hoc 签名，满足 macOS 基础沙箱完整性检查。

---

## 2. 构建指南

### 方案 A：通过 GitHub Actions 云端自动打包

通过仓库的 **Build macOS Release** 流水线，可自动在 GitHub 托管的 macOS runner 上编译并产出 DMG 安装包：
- Apple Silicon (`arm64`)：适用于 M1 / M2 / M3 / M4 系列芯片
- Intel (`x64`)：适用于 Intel 芯片
- Universal (`universal`)：通用双架构版

### 方案 B：在 Mac 本机开发与构建

```bash
# 1. 安装基础工具链
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
corepack enable

# 2. 安装依赖并启动本地开发热重载
pnpm install
pnpm macos:dev

# 3. 打包出 DMG 安装包
# 方式一：使用内置向导脚本
./scripts/build-macos.sh universal

# 方式二：使用 pnpm 命令
pnpm macos:build:arm64       # Apple Silicon DMG
pnpm macos:build:x64         # Intel DMG
pnpm macos:build:universal   # Universal DMG
```

---

## 3. macOS 运行提示（绕过 Gatekeeper 未签名拦截）

由于个人开源构建未购买 Apple 开发者企业证书签名与公证，首次打开若提示“已损坏”或“无法打开”，请在终端执行一次以下命令清除隔离属性：

```bash
sudo xattr -rd com.apple.quarantine /Applications/HGhub.app
```

或在「访达」中按住 Control 键并右键点击 `HGhub.app`，选择「打开」加入系统信任。
