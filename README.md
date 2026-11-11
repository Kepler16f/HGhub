# HGhub (跨平台桌面社区移植版 · macOS / Linux / Windows ARM64)

> [!IMPORTANT]
> **本仓库为 [Minireel/HGhub](https://github.com/Minireel/HGhub) 的社区复刻与全架构桌面端移植维护版本。**
> 
> 由于上游官方遵循 Source-only 策略仅发布源码标签、不分发预编译安装包，本仓库致力于为 **macOS（Apple Silicon & Intel）**、**Linux（x86_64 & ARM64）** 以及 **Windows on ARM64** 桌面端提供开箱即用的预编译安装包与自动化 CI 构建工作流。
> 
> 本项目严格遵循上游 [PolyForm Noncommercial 1.0.0](LICENSE) 开源许可证，**仅供个人技术探索与跨平台实践，严禁任何形式的商业用途、广告变现、VIP/卡密收费**。

---

## 🚀 快速下载安装包 (Latest Release)

👉 **[前往 GitHub Releases 页面下载最新安装包 (v1.2.1-desktop)](https://github.com/Kepler16f/HGhub/releases/tag/v1.2.1-desktop)**

| 操作系统 | 目标架构 | 软件包类型 | 下载文件名 | 适用设备 / 说明 |
| :--- | :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon | DMG 镜像 | `HGhub_1.2.1_arm64.dmg` | **macOS 推荐**：M1 / M2 / M3 / M4 系列芯片 Mac |
| **macOS** | Intel | DMG 镜像 | `HGhub_1.2.1_x64.dmg` | 传统 Intel 处理器 Mac 机型 |
| **Windows** | ARM64 (WoA) | 安装包 / 便携包 | `HGhub_1.2.1_arm64_setup.exe`<br>`HGhub_1.2.1_arm64_portable.zip` | 骁龙 X Elite / 微软 Surface 等 Windows on ARM 设备 |
| **Linux** | x86_64 | deb / AppImage | `HGhub_1.2.1_amd64.deb`<br>`HGhub_1.2.1_amd64.AppImage` | Ubuntu / Debian / Deepin / Mint x86_64 桌面端 |
| **Linux** | ARM64 (aarch64) | deb / AppImage | `HGhub_1.2.1_arm64.deb`<br>`HGhub_1.2.1_arm64.AppImage` | 树莓派 4/5、ARM64 架构 Linux 电脑与掌机 |

---

## 🍎 macOS 安装与运行说明

### 1. 安装方式
下载对应的 `.dmg` 镜像后双击挂载，将 `HGhub.app` 拖入系统的「应用程序 (Applications)」目录即可。

### 2. 绕过 Gatekeeper 拦截（“已损坏，打不开”提示）
由于开源个人构建未向 Apple 购买每年 99 美元的开发者证书与公证服务，首次双击运行安装的 `HGhub.app` 时，系统 Gatekeeper 可能会提示：
> **“HGhub” 已损坏，打不开。您应该将它移到废纸篓。**  
> 或 **“无法验证开发者，macOS 无法验证此 App 是否包含恶意软件”**

只需在终端中执行一次以下命令清除系统的隔离属性（quarantine）：
```bash
sudo xattr -rd com.apple.quarantine /Applications/HGhub.app
```
*或者：在「访达」的「应用程序」目录中，**按住 Control 键并右键点击 HGhub.app**，在弹出的菜单中选择「打开」，并在确认弹窗中再次点击「打开」即可永久加入信任。*

---

## 🐧 Linux 安装与运行说明

### 1. Debian / Ubuntu 系列 (`.deb`)
在终端中进入安装包所在目录执行：
```bash
sudo dpkg -i HGhub_1.2.1_amd64.deb

# 若提示缺少底层系统库（如 webkit2gtk），执行以下命令自动修复依赖：
sudo apt-get install -f
```

### 2. 通用免安装版 (`.AppImage`)
直接赋予执行权限后双击或终端运行：
```bash
chmod +x HGhub_1.2.1_amd64.AppImage
./HGhub_1.2.1_amd64.AppImage
```

---

## 🛠️ 移植特性与核心机制

1. **原生沉浸标题栏与系统红绿灯垂直居中**：
   - 启用 macOS 原生 `titleBarStyle: Overlay` 隐藏标题栏模式；
   - 在 `src-tauri/src/macos_titlebar.rs` 中通过 `objc2` 动态调用 AppKit 的 `NSWindow` 实时测量红绿灯按钮尺寸，计算像素级垂直居中偏移量；
   - 原生全屏状态下自动避让并收回标题栏侧边内边距。
2. **WebKit 平台流媒体协议对齐**：
   - macOS（WKWebView）与 Linux（WebKitGTK）强制要求 URI Scheme 格式；
   - 前端与 Rust 后端自动切换为 `hg://localhost` 协议基址，完整保障封面 AES 解密输出与 HLS 流媒体切片代理正常工作。
3. **Ad-hoc 完整代码签名**：
   - 在 `src-tauri/tauri.macos.conf.json` 中配置了 `signingIdentity: "-"`，对 Mach-O 二进制文件与资源包完成完整绑定签名。
4. **CI 云端免解压直下**：
   - 流水线升级至 `actions/upload-artifact@v7` 并配置 `archive: false`，CI 产物直接产出原生 DMG，下载免去解压 zip 的繁琐步骤。

---

## 💻 本地开发与云端构建

### 本地编译
```bash
# 1. 基础环境
# Node.js 22+、pnpm 10+、Rust 1.90+、目标平台 SDK（Xcode CLI / WebKitGTK-4.1）
corepack enable
pnpm install

# 2. 本地开发热重载
pnpm dev             # 前端热重载
pnpm macos:dev       # macOS 桌面端本地调试热重载

# 3. 打包出包
pnpm macos:build:arm64       # 打包 Apple Silicon DMG
pnpm macos:build:x64         # 打包 Intel DMG
pnpm linux:build             # 打包 Linux deb + AppImage
```

### 云端 CI 自动化出包
直接在 GitHub 仓库页面的 **Actions** 标签下选择对应的 Workflow（如 `Build macOS Release` 或 `Build Linux`），点击 **Run workflow** 即可由 GitHub 官方云端虚拟机自动编译打包。

---

## ⚖️ 非商业立场与免责声明

1. 本项目采用 [PolyForm Noncommercial 1.0.0](LICENSE) 许可证，**严格禁止任何商业用途**，包括但不限于销售、广告推广、卡密变现、收费 VIP 或作为付费服务载体。
2. 本项目为个人与社区的技术探索、跨平台交互实验项目。维护者与开发者不运营任何内容托管服务，不参与外部音视频内容的制作、上传或分发。
3. 软件按现状（AS-IS）提供，不承诺任何外部接口的持续有效性。涉及第三方内容或标识的权利均归原权利人所有。

## 👏 致谢

- 感谢上游项目 [Minireel/HGhub](https://github.com/Minireel/HGhub) 团队的原生实现与架构设计。
- 感谢 [Tauri](https://tauri.app/) 团队提供优秀的跨平台桌面框架。
