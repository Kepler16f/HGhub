# HGhub (跨平台桌面与移动端社区移植增强版)

**派生自上游仓库 / Forked from：[Minireel/HGhub](https://github.com/Minireel/HGhub)**

> [!IMPORTANT]
> **本仓库为 [Minireel/HGhub](https://github.com/Minireel/HGhub) 的社区派生与跨平台全端移植增强版本。**
> 
> 由于上游官方遵循 Source-only 策略仅发布源码标签、不分发预编译安装包，本仓库致力于为 **macOS（Apple Silicon & Intel）**、**Windows on ARM (WoA)**、**Linux（x86_64 & ARM64）** 桌面端以及 **iOS (IPA 免签生产包)** 提供开箱即用的预编译安装包与自动化 CI 构建工作流。
> 
> 本项目严格遵循上游 [PolyForm Noncommercial 1.0.0](LICENSE) 开源许可证，**纯技术实践与跨平台学习研究，严禁任何形式的商业用途、广告变现、VIP/卡密收费**。

---

## 🚀 快速下载安装包 (Releases)

### 💻 桌面端安装包
👉 **[前往 GitHub Releases 页面下载最新桌面版 (v1.2.1-desktop)](https://github.com/Kepler16f/HGhub/releases/tag/v1.2.1-desktop)**

| 操作系统 | 目标架构 | 软件包类型 | 下载文件名 | 适用设备 / 说明 |
| :--- | :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon | DMG 镜像 | `HGhub_1.2.1_arm64.dmg` | **Mac 推荐**：M1 / M2 / M3 / M4 系列芯片机型 |
| **macOS** | Intel | DMG 镜像 | `HGhub_1.2.1_x64.dmg` | 传统 Intel 处理器 Mac 机型 |
| **Windows** | ARM64 (WoA) | 安装包 / 便携包 | `HGhub_1.2.1_arm64_setup.exe`<br>`HGhub_1.2.1_arm64_portable.zip` | **Windows on ARM 原生**：骁龙 X Elite / 微软 Surface 等 |
| **Linux** | x86_64 | deb / AppImage | `HGhub_1.2.1_amd64.deb`<br>`HGhub_1.2.1_amd64.AppImage` | Ubuntu / Debian / Deepin / Mint x86_64 桌面端 |
| **Linux** | ARM64 (aarch64) | deb / AppImage | `HGhub_1.2.1_arm64.deb`<br>`HGhub_1.2.1_arm64.AppImage` | 树莓派 4/5、ARM64 架构 Linux 电脑与掌机 |

---

### 📱 iOS 移动端安装包
👉 **[前往 GitHub Releases 页面下载 iOS 安装包 (1.2.1-ios)](https://github.com/Kepler16f/HGhub/releases/tag/v1.2.1-ios)**

| 操作系统 | 软件包类型 | 下载文件名 | 适用平台 / 安装方式 |
| :--- | :--- | :--- | :--- |
| **iOS** (16+) | 未签名生产包 (`.ipa`) | `HGhub_1.2.1_ios.ipa` | 支持 **TrollStore（巨魔）** 永久安装、AltStore / SideStore / 轻松签自签安装 |

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

## 🪟 Windows on ARM (WoA) 运行说明

针对搭载 **高通骁龙 X Elite / 骁龙 8cx / 微软 SQ 系列芯片** 的 Windows on ARM 设备（如 Surface Pro、Surface Laptop 等）：
- **原生架构执行**：针对 `aarch64-pc-windows-msvc` 原生编译，无需经过 Prism / x86 仿真转译层，CPU 占用极低、功耗大幅优化且界面流畅响应。
- **便携版与安装版双选**：
  - `HGhub_1.2.1_arm64_setup.exe`：全自动引导安装程序，创建开始菜单与桌面快捷方式；
  - `HGhub_1.2.1_arm64_portable.zip`：免安装便携绿色版，解压到任意文件夹双击 `HGhub.exe` 即可直接运行。

---

## 🐧 Linux 安装与运行说明

### 1. Debian / Ubuntu 系列 (`.deb`)
在终端中进入安装包所在目录执行：
```bash
# x86_64
sudo dpkg -i HGhub_1.2.1_amd64.deb

# ARM64
sudo dpkg -i HGhub_1.2.1_arm64.deb

# 若提示缺少底层系统库（如 webkit2gtk），执行以下命令自动修复依赖：
sudo apt-get install -f
```

### 2. 通用免安装版 (`.AppImage`)
直接赋予执行权限后双击或终端运行：
```bash
chmod +x HGhub_1.2.1_*.AppImage
./HGhub_1.2.1_*.AppImage
```

---

## 📱 iOS (IPA) 侧载安装与专属特性说明

### 1. 安装方式
- **TrollStore（巨魔商店）**：下载 `HGhub_1.2.1_ios.ipa` 后分享至巨魔商店，一键永久免签安装，无证书过期失效困扰；
- **个人自签工具**：使用 AltStore、SideStore 或 Sideloadly，通过个人 Apple ID 每 7 天续签；
- **企业证书 / 签名软件**：导入轻松签（ESign）、牛蛙助手、Feather、全能签直接签名安装。

### 2. iOS 移动端专属交互增强
- **原生横竖屏智能旋转**：横屏视频支持点击「横屏全屏」直接调用 iOS 系统底层屏幕旋转；支持手机物理横置自动进入沉浸全屏，竖回自动还原舞台；
- **双指自由缩放与视口平移**：竖屏观看横屏短剧时支持 1x~4x 双指缩放，附带「放大铺满 / 完整适应 / 还原缩放」快捷胶囊；
- **高频触控手势**：
  - 单击画面清屏（隐藏所有交互控件与背景压暗）；
  - 双击画面即时暂停 / 恢复播放；
  - 长按画面开启 2.0x 极速倍速，向上轻滑即可锁定倍速；
  - 任意位置左右横滑实时精准预览寻道（Seek）。
- **WKWebView 播放加固**：内置 AES Key 与分片内存高速缓存，规避 iOS WebKit 的跨域与内存释放限制。

---

## 🛠️ 技术栈与全端架构

| 层级 | 技术方案 |
| --- | --- |
| 跨平台核心 | Rust、Tauri 2 |
| 原生系统集成 | macOS AppKit（`objc2`）、Windows MSVC/WoA、WebKitGTK、iOS UIKit |
| 前端界面 | React 19、TypeScript、Vite、Tailwind CSS |
| 播放核心 | hls.js (MSE / Native HLS 自动切换与缓存) |
| 网络与解密 | reqwest、rustls、aes-128-cbc |

---

## 💻 本地开发与云端构建

```bash
# 基础依赖：Node.js 22+、pnpm 10+、Rust 1.90+
corepack enable
pnpm install

# 本地调试热重载
pnpm dev             # 前端热重载
pnpm macos:dev       # macOS 桌面端本地调试热重载

# 本地打包出包
pnpm macos:build:arm64       # macOS Apple Silicon
pnpm macos:build:x64         # macOS Intel
pnpm linux:build             # Linux (deb + AppImage)
```

云端出包：通过 GitHub Actions 提供的专用工作流（`Build macOS Release`、`Build Linux`、`Build Desktop ARM64`、`Build iOS IPA`），一键在云端全自动编译并输出对应平台的安装包。

---

## ⚖️ 非商业立场与免责声明

1. 本项目采用 [PolyForm Noncommercial 1.0.0](LICENSE) 许可证，**严格禁止任何商业用途**，包括但不限于销售、广告推广、卡密变现、收费 VIP 或作为付费服务载体。
2. 本项目为个人与社区的技术探索、跨平台交互实验项目。维护者与开发者不运营任何内容托管服务，不参与外部音视频内容的制作、上传或分发。
3. 软件按现状（AS-IS）提供，不承诺任何外部接口的持续有效性。涉及第三方内容或标识的权利均归原权利人所有。

## 👏 致谢

- 感谢上游项目 [Minireel/HGhub](https://github.com/Minireel/HGhub) 团队的原生实现与架构设计。
- 感谢 [Tauri](https://tauri.app/) 团队提供优秀的跨平台桌面框架。
