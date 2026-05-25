# VMCTL

一个为 VMware Fusion 设计的终端虚拟机管理界面，灵感来自 [K9s](https://k9scli.io/)。

## 功能特性

- 终端 UI 管理 VMware Fusion 虚拟机
- 支持启动、停止、挂起、恢复虚拟机
- 自动检测虚拟机状态（运行中/已停止/已挂起）
- 自动获取虚拟机 IP 地址（通过 MAC 地址转换）
- 键盘友好的操作方式（类 K9s 风格）

## 技术选型

- **Rust** - 系统编程语言，提供高性能和内存安全
- **ratatui** - Rust 终端 UI 库，支持丰富的文本样式和布局
- **crossterm** - 跨平台终端操作库，处理键盘输入和终端特性
- **vmrun** - VMware Fusion 命令行工具，通过子进程调用

## 快速开始

### 构建

```bash
cargo build --release
```

### 安装

```bash
./install.sh
```

或者手动将可执行文件添加到 PATH：

```bash
cp target/release/vmctl-rust ~/.local/bin/vmctl
chmod +x ~/.local/bin/vmctl
```

确保 `~/.local/bin` 在你的 PATH 中：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## 配置

首次运行前，创建 `~/.config/v9s/config.toml`（注意：代码中读取的是当前目录的 `config.toml`，可复制到项目根目录）：

```toml
vmrun_path = "/Applications/VMware Fusion.app/Contents/Library/vmrun"
vm_dir = "/Users/username/Virtual Machines.localized"
refresh_interval = 5
ascii_file = "ascii.txt"
```

### 配置说明

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `vmrun_path` | vmrun 命令路径 | `/Applications/VMware Fusion.app/Contents/Library/vmrun` |
| `vm_dir` | 虚拟机目录 | `~/Virtual Machines.localized` |
| `refresh_interval` | 状态刷新间隔（秒） | `5` |
| `ascii_file` | ASCII 艺术字文件 | `ascii.txt` |

## 操作

| 按键 | 功能 |
|------|------|
| `w` / `s` 或 `↑` / `↓` | 上下移动选择 |
| `Enter` | 启动虚拟机 |
| `x` | 停止虚拟机 |
| `p` | 挂起虚拟机 |
| `r` | 继续运行（恢复） |
| `q` | 退出 |

## 设计

### 架构

```
┌─────────────────────────────────────────┐
│                 main.rs                 │
│  ┌─────────────┐  ┌──────────────────┐  │
│  │  Terminal   │  │  Event Loop      │  │
│  │  Renderer   │  │  (Input Handler) │  │
│  └──────┬──────┘  └────────┬─────────┘  │
│         │                  │            │
│         ▼                  ▼            │
│  ┌─────────────────────────────────┐    │
│  │         VmManager               │    │
│  │  - 加载配置                      │    │
│  │  - 定时刷新虚拟机状态             │    │
│  │  - 管理 vmrun 命令执行           │    │
│  └─────────────────────────────────┘    │
│         │                  │            │
│         ▼                  ▼            │
│  ┌─────────────┐  ┌──────────────────┐  │
│  │    vm.rs    │  │    vmrun.rs      │  │
│  │  Vm 结构体  │  │  vmrun 封装      │  │
│  │  MAC→IP    │  │  状态检测        │  │
│  └─────────────┘  └──────────────────┘  │
└─────────────────────────────────────────┘
```

### 模块

- **vm.rs** - 虚拟机数据结构，包含路径、名称、状态、IP
- **vmrun.rs** - vmrun 命令封装，提供启动/停止/挂起/恢复等操作
- **manager.rs** - 虚拟机管理器，负责定时刷新状态和操作协调
- **main.rs** - 主程序入口，TUI 渲染和事件处理

### 状态检测

VMware Fusion 不提供直接查询虚拟机状态 API，本项目通过以下方式判断：

1. **运行中** - 通过 `vmrun list` 命令获取正在运行的虚拟机列表
2. **已挂起** - 通过检测 `.vmss` 文件是否存在（挂起时生成）
3. **已停止** - 既不在运行列表中，也没有 `.vmss` 文件

### IP 地址获取

虚拟机通常配置 NAT 网络，IP 地址在 `172.16.x.x` 网段。通过解析 `.vmx` 配置文件中的 MAC 地址，将 MAC 地址的后两字节转换为 IP 地址：

```
MAC: 00:0C:29:50:A2:D3  →  IP: 172.16.50.2
```

## 依赖

- macOS
- VMware Fusion
- Rust 1.70+
