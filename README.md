# HGhub (macOS 社区复刻移植版)

> [!IMPORTANT]
> **本仓库为 [Minireel/HGhub](https://github.com/Minireel/HGhub) 的社区复刻与 macOS 深度移植版本。**
> 
> 本分支/复刻版本核心专注于为 **macOS 平台（Apple Silicon M系列芯片与 Intel 芯片）** 提供完整、开箱即用的原生桌面体验，补齐了 macOS 专用构建脚本、CI 云端构建流水线与系统交互细节。
> 
> 本项目严格遵循上游 [PolyForm Noncommercial 1.0.0](LICENSE) 开源许可证，**纯技术实践与学习研究，严禁任何形式的商业用途、广告变现、VIP/卡密收费**。

---

## 🍎 macOS 复刻移植特性

相比官方原版主要聚焦于 Windows/Android，本移植版针对 macOS 进行了以下深度适配：

1. **原生沉浸标题栏与系统红绿灯垂直居中**：
   - 启用 macOS 原生 `titleBarStyle: Overlay` 隐藏标题栏模式。
   - 针对不同 macOS 版本（Sonoma / Sequoia 等）系统红绿灯几何尺寸差异，在 `src-tauri/src/macos_titlebar.rs` 中通过 `objc2` 动态探测 AppKit 的 `NSWindow` 按钮尺寸，实时计算精确居中偏移量，杜绝标题栏歪斜。
   - 原生全屏状态下自动避让并收回标题栏侧边内边距。

2. **WebKit 平台协议与流媒体代理对齐**：
   - macOS WebKit 强制要求 URI Scheme 格式，本项目在前端与 Rust 后端自动切换为 `hg://localhost` 协议基址，完整保障封面 AES 解密输出与 HLS 流媒体切片代理正常工作。

3. **三架构全覆盖**：
   - **Apple Silicon (`arm64`)**：针对 M1 / M2 / M3 / M4 系列芯片原生编译，极低能耗与流畅动画。
   - **Intel (`x64`)**：兼容传统 Intel 处理器的 Mac 机型。
   - **Universal (`universal`)**：通用双架构 DMG，全机型免甄别运行。

4. **Ad-hoc 完整代码签名**：
   - 在 `src-tauri/tauri.macos.conf.json` 中配置了 `signingIdentity: "-"`，对 Mach-O 二进制文件与资源包完成完整绑定签名，避免被系统直接判定为损坏 bundle。

5. **GitHub Actions 自动化云端出包（方案 A）**：
   - 无需本地拥有 Mac 电脑，只需在 GitHub 仓库一键触发工作流，即可由 GitHub 官方 macOS 虚拟机自动编译并打包出完整的 `.dmg` 与 `.app.tar.gz`。

---

## 📦 如何获取与构建 macOS 版本

### 方案 A：通过 GitHub Actions 云端构建（推荐，无需 Mac 设备）

如果你目前使用 Windows 或 Linux，无法在本地编译 macOS 原生应用，可以直接使用本仓库配置的 CI 流水线：

1. **推送到自己的 GitHub 仓库**：
   将本仓库推送（Push）到你的 GitHub 个人账号下。
2. **触发云端编译**：
   - 打开 GitHub 仓库页面的 **Actions** 标签。
   - 在左侧选择 **Build macOS Release** 工作流。
   - 点击 **Run workflow**（分支选择 `main`），确认启动。
3. **下载 DMG 产物**：
   GitHub 官方 `macos-latest` 虚拟机将在约 5~8 分钟内完成编译，并在构建记录底部的 **Artifacts** 区域提供下载：
   - `HGhub_<版本>_arm64.dmg`（推荐 M 系列芯片用户）
   - `HGhub_<版本>_x64.dmg`（推荐 Intel 芯片用户）
   - `HGhub_<版本>_universal.dmg`（通用双架构版）

---

### 方案 B：在 Mac 本地开发与编译

如果你手头有 Mac 电脑，也可以在本地直接运行或打包：

```bash
# 1. 确保已安装基础工具链
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
corepack enable

# 2. 安装前端依赖
pnpm install

# 3. 启动 macOS 本地开发热重载
pnpm macos:dev

# 4. 一键打包 macOS DMG 安装包
# 方式一：使用内置向导脚本（支持 native / arm64 / x64 / universal）
./scripts/build-macos.sh universal

# 方式二：使用 pnpm 命令
pnpm macos:build:arm64       # 打包 Apple Silicon M系列芯片 DMG
pnpm macos:build:x64         # 打包 Intel 芯片 DMG
pnpm macos:build:universal   # 打包 Universal 通用版 DMG
```

---

## ⚠️ macOS 首次打开“应用已损坏”解决方法

由于开源个人构建未向 Apple 购买每年 99 美元的开发者证书与公证服务，首次在 macOS 上双击运行安装的 `HGhub.app` 时，系统 Gatekeeper 安全机制可能会弹出：

> **“HGhub” 已损坏，打不开。您应该将它移到废纸篓。**  
> 或 **“无法验证开发者，macOS 无法验证此 App 是否包含恶意软件”**

这是 macOS 针对未公证应用施加的隔离属性（`com.apple.quarantine`），**只需在终端执行一次以下命令即可解除拦截**：

```bash
sudo xattr -rd com.apple.quarantine /Applications/HGhub.app
```

*或者：在「访达」的「应用程序」目录中，**按住 Control 键并单击（右键）HGhub.app**，在弹出的菜单中选择「打开」，并在弹窗中再次点击「打开」，即可永久加入受信任列表。*

---

## 🛠️ 技术栈与目录结构

| 层级 | 技术方案 |
| --- | --- |
| 应用框架 | Rust、Tauri 2 |
| macOS 原生集成 | AppKit（`objc2`）、WebKit |
| 前端界面 | React 19、TypeScript、Vite、Tailwind CSS |
| 播放核心 | hls.js |
| 网络与解密 | reqwest、rustls、aes-128-cbc |

```text
src/                    界面、交互、状态管理与组件
src-tauri/              原生能力、macOS 标题栏控制与 Rust 路由
scripts/                macOS 打包向导 (build-macos.sh) 与开发验证脚本
docs/                   macOS 移植技术指南 (macos-port.md) 与架构文档
.github/workflows/      GitHub Actions 云端构建流水线 (build-macos.yml)
```

---

## ⚖️ 非商业立场与免责声明

1. 本项目采用 [PolyForm Noncommercial 1.0.0](LICENSE) 许可证，**严格禁止任何商业用途**，包括但不限于销售、广告推广、卡密变现、收费 VIP 或作为付费服务载体。
2. 本项目为个人与社区的技术探索、跨平台交互实验项目。维护者与开发者不运营任何内容托管服务，不参与外部音视频内容的制作、上传或分发。
3. 软件按现状（AS-IS）提供，不承诺任何外部接口的持续有效性。涉及第三方内容或标识的权利均归原权利人所有。

## 👏 致谢

- 感谢上游项目 [Minireel/HGhub](https://github.com/Minireel/HGhub) 团队的原生实现与架构设计。
- 感谢 [Tauri](https://tauri.app/) 团队提供优秀的跨平台桌面框架。
