# HGhub macOS 适配与移植指南

本项目基于 **Rust + Tauri 2 + React 19**，已完成针对 macOS 桌面系统的完整移植与适配。

---

## 1. macOS 核心适配机制

### 1.1 窗口与系统红绿灯垂直居中
- **原生沉浸式标题栏**：macOS 采用 `titleBarStyle: Overlay` 与 `hiddenTitle: true`（配置见 `src-tauri/tauri.macos.conf.json`）。
- **动态测量对齐**：由于不同 macOS 系统版本（macOS Sonoma / Sequoia 等）以及不同缩放比例下红绿灯按钮的底边偏移量不同，直接写死固定坐标会导致红绿灯偏上或偏下。
  在 `src-tauri/src/macos_titlebar.rs` 中，通过 `objc2` 动态调用 AppKit 的 `NSWindow` 及 `standardWindowButton(CloseButton)` 实时测量按钮几何尺寸，计算出精确的居中偏移量并应用到窗口。
- **全屏与自绘控制区避让**：在全屏状态下，系统红绿灯会自动隐藏；前端 `src/components/DesktopChrome.tsx` 监听全屏事件并动态收回左侧让位边距（84px -> 14px）。

### 1.2 自定义网络协议与流媒体代理
- **URL Scheme 适配**：
  - Windows/Android 使用 `http://hg.localhost`；
  - macOS/iOS/Linux 使用 `hg://localhost`。
  - 前端 `src/api.ts` 与 Rust 侧 `media::LOCAL_ORIGIN` 已对齐该规则，确保 m3u8 切片解密与代理转发在 WebKit 引擎下正常工作。
- **MSE / hls.js 兼容性**：前端在 Safari/WebKit 下开启标准 MSE，hls.js 处理恢复与续播逻辑。

### 1.3 Ad-hoc 签名
- 开源项目默认未绑定付费 Apple 开发者证书。在 `src-tauri/tauri.macos.conf.json` 中配置了 `"signingIdentity": "-"`，Tauri 会在打包时对整个 `.app` 包（包括 Mach-O 可执行文件与 Info.plist）进行 ad-hoc 签名，满足 macOS 系统的基础沙箱完整性检查。

---

## 2. 如何获取与构建 macOS 版本

### 方案 A：通过 GitHub Actions 云端自动打包（无需 Mac 设备，推荐）

由于编译 macOS 原生应用需要 macOS SDK 与 WebKit 原生工具链，如果你当前工作在 Windows/Linux 环境下，可以直接借助 GitHub 提供的免费 macOS Runner 进行云端打包：

1. **推送代码至 GitHub**：
   在 GitHub 上新建私有或公开仓库，并将本项目推送上去。
2. **触发构建 Workflow**：
   - 进入 GitHub 仓库页面的 **Actions** 标签；
   - 在左侧选择 **Build macOS Release**；
   - 点击右侧 **Run workflow** -> 选择 `main` 分支并运行。
3. **下载安装包**：
   构建完成后，在 Workflow 详情底部或 Releases 页面可直接下载以下产物：
   - `HGhub_<版本>_arm64.dmg`（适合 M1 / M2 / M3 / M4 系列 Apple Silicon 芯片）
   - `HGhub_<版本>_x64.dmg`（适合 Intel 处理器机型）
   - `HGhub_<版本>_universal.dmg`（通用双架构版，兼容所有 Mac）

---

### 方案 B：在 Mac 本机开发与构建

如果你的机器为 macOS（或准备在 Mac 设备上运行），请按以下步骤操作：

#### 1. 准备基础环境
```bash
# 1. 安装 Xcode 命令行工具
xcode-select --install

# 2. 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. 安装 Node.js (v20+) 与 pnpm
corepack enable
```

#### 2. 安装依赖并启动本地开发热重载
```bash
pnpm install

# 启动 macOS 桌面端开发环境（热重载）
pnpm macos:dev
```

#### 3. 本地打包出包
你可以直接使用预置的构建脚本，或者运行 npm 命令：

- **使用预置脚本**：
  ```bash
  # 赋予执行权限
  chmod +x scripts/build-macos.sh

  # 打包当前原生架构
  ./scripts/build-macos.sh

  # 或者打包 Universal 通用双架构
  ./scripts/build-macos.sh universal
  ```

- **或者使用 pnpm 命令**：
  ```bash
  # 构建 Apple Silicon (M系列芯片) DMG
  pnpm macos:build:arm64

  # 构建 Intel 芯片 DMG
  pnpm macos:build:x64

  # 构建 Universal 通用双架构 DMG
  pnpm macos:build:universal
  ```

构建生成的 `.dmg` 和 `.app` 产物位于 `src-tauri/target/release/bundle/dmg/` 或对应的 target 子目录下。

---

## 3. macOS 运行避坑说明（绕过 Gatekeeper 未签名拦截）

由于开源个人构建未向 Apple 购买付费开发者证书公证，首次在 macOS 上双击运行 `.dmg` 安装的 `HGhub.app` 时，系统可能弹出：
> **“HGhub” 已损坏，打不开。您应该将它移到废纸篓。**
> 或 **无法验证开发者**

这是 macOS Gatekeeper 对未公证应用添加了隔离属性（`com.apple.quarantine`），**只需在终端执行一次以下命令即可正常运行**：

```bash
sudo xattr -rd com.apple.quarantine /Applications/HGhub.app
```

或者：在「访达」的「应用程序」目录中，**按住 Control 键并单击（右键）HGhub.app**，在弹出的菜单中选择「打开」，并在弹窗提示中点击「打开」即可永久加入信任名单。
