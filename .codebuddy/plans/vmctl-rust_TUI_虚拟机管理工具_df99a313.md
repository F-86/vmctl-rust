---
name: vmctl-rust TUI 虚拟机管理工具
overview: 基于 ratatui + crossterm 构建 TUI 界面，管理 VMware Fusion 虚拟机，实现启动、停止、暂停、恢复、状态查看等全部功能，支持从指定目录自动扫描虚拟机列表并实时更新状态。
todos:
  - id: create-vm-models
    content: 定义虚拟机数据结构和状态枚举
    status: completed
  - id: implement-vmrun-wrapper
    content: 封装 vmrun 命令调用（list/start/stop/pause/unpause/getState）
    status: completed
    dependencies:
      - create-vm-models
  - id: build-vm-manager
    content: 实现 VM 管理器（扫描目录 + 状态轮询）
    status: completed
    dependencies:
      - implement-vmrun-wrapper
  - id: design-tui-layout
    content: 设计 TUI 布局（虚拟机列表 + 状态栏 + 提示栏）
    status: completed
  - id: implement-keyboard-nav
    content: 实现键盘导航和操作交互
    status: completed
    dependencies:
      - design-tui-layout
  - id: add-real-time-refresh
    content: 集成状态自动刷新机制
    status: completed
    dependencies:
      - build-vm-manager
---

## 产品概述

基于 VMware Fusion 的命令行工具 vmrun 开发一个 TUI 界面应用，用于管理虚拟机，实现启动、停止、暂停、恢复和状态查看功能。

## 核心功能

1. **虚拟机列表管理**：从指定目录自动扫描 .vmx 文件，实时显示所有发现的虚拟机
2. **虚拟机操作**：支持启动、停止（硬停止/软停止）、暂停、恢复虚拟机
3. **状态监控**：实时轮询获取每个虚拟机的运行状态，并在界面上显示
4. **交互界面**：键盘导航选择虚拟机，操作反馈提示

## 用户界面

- 左侧/主区域：虚拟机列表（名称、状态指示器）
- 底部：操作提示和快捷键说明
- 支持键盘 j/k 上下选择，Enter 执行操作

## 技术栈

- **语言**：Rust（现有项目）
- **TUI 框架**：ratatui 0.30.0（已有）
- **终端事件**：crossterm 0.29.0（已有）
- **外部进程**：std::process::Command（标准库）

## 架构设计

### 模块划分

1. **vmrun 封装层** (`src/vmrun.rs`)

- 封装 vmrun 命令调用
- 实现：list、start、stop、pause、unpause、getState

2. **虚拟机模型** (`src/vm.rs`)

- Vm 结构体：路径、名称、状态
- VmState 枚举：Running、Stopped、Paused、Unknown

3. **VM 管理器** (`src/manager.rs`)

- 扫描指定目录获取 .vmx 文件
- 维护虚拟机列表并更新状态
- 线程安全的状态刷新（定时轮询）

4. **UI 组件** (`src/ui.rs`)

- 虚拟机列表组件
- 状态栏组件
- 操作反馈组件

### 数据流

```
扫描目录 → 获取 .vmx 列表 → 定时调用 vmrun getState → 更新内存状态 → 渲染 UI
                                              ↑                        ↓
                                      用户操作 (start/stop/...) → 执行 vmrun → 刷新状态
```

### 关键实现

- 使用 std::thread 定时刷新状态（每 5 秒）
- 命令执行使用 `std::process::Command`
- 状态解析根据 vmrun 输出判断