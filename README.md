# Pulse

**看见系统的节奏。**

一个浅色、轻量的原生资源监控器。以紧凑的指标、时间趋势与进程表格展示设备状态，不使用 WebView。

[下载 Linux 安装包](https://github.com/fzf54122/resource-pulse/releases/latest) · [更新记录](CHANGELOG.md) · [Apache-2.0](LICENSE)

Pulse 使用 Zed 的 **GPUI** 绘制界面，用独立线程采集系统指标。没有 WebView，不依赖本机 Zed 安装，也不需要相邻的源码或缓存目录。

## 一眼了解

| 页面 | 可以做什么 |
| --- | --- |
| 概览 | 查看 CPU、内存、网络和存储空间 |
| 进程 | 按名称、CPU、内存排序，分页浏览，点击查看详情 |
| 资源历史 | 切换指标和时间范围，查看平均值与峰值 |
| 设置 | 选择更新频率、关闭窗口的行为，自动保存选择 |

侧栏、资源卡片、进程行和操作按钮都有对应交互。所有指标只在本机读取和展示，不上传数据，也不结束系统进程。

资源卡片打开对应指标的历史。趋势图按实际时间定位，暂停后的缺口不会连成连续曲线；挂载卷趋势来自真实容量采样，不表示 I/O 速度。空格暂停或继续采样，`Esc` 关闭进程详情。

## 开始使用

准备好 Rust / rustup、系统构建依赖和可用的图形桌面，然后在仓库根目录运行：

```bash
cargo run --release --locked
```

仓库固定使用 **Rust 1.98.1**。GPUI 和窗口后端以 Git 依赖引入，固定到官方 `zed-industries/zed` 的提交 `c4273271092b0287f7d181f616bd5171e0a2f70c`，由 Cargo 自动获取。首次构建需要联网，获取 Zed 源码可能花一些时间；之后使用本地构建缓存。

这里不使用较老的 crates.io GPUI 0.2.2 发布包，因为其渲染后端在当前 Linux 环境的实际启动检查中无法初始化 GPU。

### Linux

以下为 Ubuntu / Debian 依赖示例，其他发行版请安装对应软件包：

```bash
sudo apt-get update
sudo apt-get install -y build-essential clang cmake pkg-config \
  libssl-dev libfontconfig1-dev libfreetype6-dev libasound2-dev \
  libwayland-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev \
  libvulkan1 mesa-vulkan-drivers fonts-noto-cjk
```

需要 X11 或 Wayland 会话，以及兼容的 GPU 驱动。无图形桌面的 SSH 终端不能直接显示窗口。若提示无法初始化图形设备，请先检查驱动和 Vulkan 支持。

### Deb 安装与桌面集成

安装打包工具 `dpkg-dev`、`librsvg2-bin`、`desktop-file-utils` 后，在仓库根目录运行：

```bash
python3 scripts/build_deb.py
# 本机已有最新 release 构建时：
# python3 scripts/build_deb.py --skip-build
sudo apt install ./target/debian/resource-pulse_0.1.1_amd64.deb
```

打包脚本默认先构建 release，不执行安装，不修改自启动或 Dock 固定项。安装包包含二进制、桌面入口、原创雨燕吉祥物 Pip 的 SVG、16–512 逻辑像素的 1×/2× PNG，以及独立托盘图标。安装和卸载会刷新系统图标缓存，不重启桌面或 Plank。运行时也嵌入图标，直接执行二进制时仍能显示窗口与托盘图标。

- **uTools**：通过标准应用入口搜索 `Pulse` 或“系统监控”，不是独立的 uTools 插件。具体是否需要刷新应用索引取决于 uTools 的版本和配置。
- **Plank / Dock**：启动后可在应用图标上选择保留。桌面文件名与窗口 `WM_CLASS` 使用 `io.github.fzf.Pulse`；桌面入口直接指定可缩放 SVG 路径，不依赖旧的主题缓存。Alt+Tab 的 256px 窗口图标保持不变。Plank 的应用识别依赖安装后的桌面入口；只从构建目录运行时可能回退为窗口缩略图，不能替代安装验收。
- **系统托盘**：通过 StatusNotifierItem / D-BusMenu 提供打开、暂停、设置、最小化与退出。独立的小尺寸雨燕图标通过可直接加载的 SVG 路径提供，另有 16、22、24、32、48、64、96px ARGB 位图回退，不再声明可能无法解析的主题图标名。SVG 位于 `$XDG_CACHE_HOME/resource-pulse/icons/`（默认 `~/.cache/resource-pulse/icons/`），缓存目录不可用时仅发送内嵌位图。在支持该协议的桌面显示；GNOME 可能需要 AppIndicator 扩展，不强制安装桌面扩展。
- **dde-top-panel 全局菜单**：X11 下导出 `com.canonical.dbusmenu`，发布窗口上的 `_KDE_NET_WM_APPMENU_SERVICE_NAME` 和 `_KDE_NET_WM_APPMENU_OBJECT_PATH`，并通过 `com.canonical.AppMenu.Registrar.RegisterWindow` 注册。与窗口菜单共用菜单定义和操作；面板可用时隐藏窗口内的重复菜单。监听注册器重启并重新注册，面板不可用时回退到窗口内菜单。Wayland 当前使用窗口内菜单。
- **关闭窗口**：默认在托盘可用时最小化并继续监控，可在设置中改为退出。没有托盘支持时正常退出，不留下无法找回的后台窗口。
- **单实例**：同一用户会话再次启动时激活已有窗口，不重复创建监控线程和 GPU 上下文。D-Bus 不可用时提供窗口模式并记录原因。

更新频率和关闭行为保存到 `$XDG_CONFIG_HOME/resource-pulse/settings.json`，未设置时使用 `~/.config/resource-pulse/settings.json`。配置损坏不会在启动时被自动覆盖；监控历史不写入磁盘。

Deb 使用本机动态库版本计算依赖，需要与构建机兼容的 Debian 系发行版。当前不提供跨发行版 ABI 保证，也不自动下载、上传或发布安装包。

在 X11 和支持 StatusNotifierItem 的桌面安装后，可验证文件、单实例、托盘操作和资源开销：

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify_runtime.py --seconds 10 --require-global-menu --output target/runtime-report.json
```

请先退出正在使用的 Pulse。检测脚本仅控制自己启动的进程，使用临时设置目录，不改动已有偏好。会验证实际窗口的全局菜单注册，通过全局菜单暂停、通过托盘恢复，再关闭到托盘并退出；CPU 按单个逻辑核心的占用率报告。非全局菜单桌面请去掉 `--require-global-menu`。它不验证 uTools 自身的索引或 Plank 的固定项，也不适用于无托盘的桌面。

GPUI 也包含 macOS / Windows 后端；本项目优先验证 Linux。macOS 需要 Xcode / Command Line Tools，Windows 需要 MSVC Build Tools 和 Windows SDK，暂未验证这两个平台。

## GUI 可以单独复用

项目分为两层：

- **`resource-pulse`**：应用入口、采样控制、领域状态和页面分别组织。排序、分页、格式化、指标和趋势计算不依赖视图。
- **`pulse-ui`**：独立 GUI crate，提供主题令牌、按钮、面板、矢量图标、进度条和时间趋势图，不包含监控业务。图表接收归一化坐标和断线标记，不自行生成数据。

`pulse-ui` 重新导出 GPUI 和应用入口。其他项目只需引用这一层，不需要引入 `sysinfo`。

```toml
[dependencies]
# 本地开发
pulse-ui = { path = "../resource-pulse/crates/pulse-ui" }

# Git 依赖，固定实际发布版本：
# pulse-ui = { git = "https://github.com/fzf54122/resource-pulse.git", tag = "v0.1.1" }
```

在视图中组合组件：

```rust
use pulse_ui::{button, caption, card, gpui::prelude::*};

let panel = card()
    .child(caption("你的下一个桌面应用"))
    .child(button("open", "打开", false, |_, _, _| {
        // 接入当前应用自己的操作。
    }));
```

查看不含监控业务的独立示例：

```bash
cargo run --locked -p pulse-ui --example showcase
```

`pulse-ui` 尚未发布到 crates.io，目前支持仓库内引用与 Git 依赖。应用状态和点击行为由使用方管理，组件不绑定资源监控器。

## 目录

```text
src/
├── main.rs                 窗口与应用入口
├── app.rs                  采样生命周期、操作与快捷键
├── brand.rs                应用身份与嵌入图标
├── commands.rs             菜单、托盘共享操作
├── desktop/                Linux 托盘和单实例服务
├── preferences.rs          设置读写与校验
├── model.rs                状态、排序与分页
├── model/
│   ├── metric.rs           指标定义、时间轴与趋势统计
│   └── format.rs           单位与时长格式化
├── collector.rs            有界通道与采样线程生命周期
├── collector/sampler.rs    系统读取与快照组装
├── views/                  应用框架、概览、进程、历史、设置
└── tests.rs                点击、键盘、窗口边界与采样测试
crates/pulse-ui/
├── src/lib.rs              公共组件接口
├── src/theme.rs            颜色令牌
├── src/components.rs       通用布局与控件
├── src/icon.rs             矢量图标
├── src/chart.rs            折线图与柱状图
└── examples/showcase.rs    独立组件示例
```

Cargo workspace 管理这两层。`Cargo.lock` 与 `rust-toolchain.toml` 固定可复现的构建基础；CI 配置位于 `.github/workflows/ci.yml`。

## 当前边界

- GPU、温度和逐磁盘 I/O 尚未实现。挂载卷显示容量，不是读写速度；多个卷可能属于同一物理设备。
- 网络速率是可见接口收发之和，虚拟接口可能重复统计流量；进程 CPU 可跨核心累加超过 100%。
- 历史最多保留 300 个采样点，仅在本次运行中保存。不可用的指标留空，不伪装成 0%；暂停期间不补造数据。
- 后台采样通过容量为 1 的异步通道唤醒界面，不再以 200ms 计时器轮询；暂停时阻塞等待操作，不继续扫描系统。
- `sysinfo` 仅启用 CPU、内存、网络与容量相关功能，关闭额外的进程 I/O 和可执行文件读取，不启用并行扫描线程池。
- 窗口不可见时不扫描进程详情、不请求重绘。进程快照以 `Arc` 共享；恢复窗口后更新详情。
- 设备列表每 30 秒发现一次，容量每 5 秒更新，CPU、内存和网络按用户选择的频率更新。新挂载卷和网卡可能因此延迟出现。
- 系统权限可能限制部分指标。不同平台的指标完整性仍需分别验证。

## 开发与验证

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --locked
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_*.py'
desktop-file-validate packaging/io.github.fzf.Pulse.desktop
```

测试会派发真实的 GPUI 鼠标点击和键盘事件，覆盖导航、指标跳转、暂停、排序、分页、进程详情、历史范围、设置、采样失败状态，以及最小窗口的关键控件边界。领域测试覆盖趋势时间轴、暂停缺口、容量采样、单位换算和分页边界。采样测试会只读访问本机指标。

如已安装 `neo-runner`，可运行统一检查：

```bash
neo-runner run -f .agents/loop.yaml --output json
```

请保留 `Cargo.lock` 并优先使用 `--locked`。GPUI 仍在快速演进，升级版本或依赖后应重新验证，而不是直接替换整个锁文件。

## 许可证

项目原创代码与图标采用 [Apache-2.0](LICENSE)。第三方依赖保留各自许可证，参见 [第三方声明](THIRD_PARTY_NOTICES.md)。Deb 内包含项目许可证及生成的依赖许可证清单。

项目使用 GPUI 组件，不分发 Zed 编辑器。构建产物、本机日志和验证记录不会随源码提交。
