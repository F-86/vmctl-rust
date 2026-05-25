# VMware 提供但本项目未使用的功能

本文档基于本机 VMware Fusion 自带工具的实际帮助信息和官方文档整理。所有命令均在 `/Applications/VMware Fusion.app/Contents/Library/` 下可找到。

---

## 一、vmrun 未使用的命令

### 1.1 电源管理（Power Commands）

| 命令 | 功能 | 说明 |
|------|------|------|
| `reset` | 重启虚拟机 | 支持 `hard`（强制）/ `soft`（客户端关机再启动） |
| `pause` | 暂停虚拟机 | 冻结 CPU 执行但不释放内存（与 suspend 不同） |
| `unpause` | 恢复暂停 | 解除 pause 状态 |

> **与本项目 suspend/resume 的区别**: `pause`/`unpause` 不写入 .vmss 文件，只冻结 CPU；`suspend` 会将内存保存到磁盘并释放宿主资源。

### 1.2 快照管理（Snapshot Commands）

### ~~1.2 快照管理（Snapshot Commands）~~ ✅ 已实现

> 快照管理已在本项目中实现，详见 `vmware-features-used.md`。

### 1.3 网络适配器管理（Network Adapter Commands）

| 命令 | 功能 | 用法 |
|------|------|------|
| `listNetworkAdapters` | 列出所有网卡 | `vmrun listNetworkAdapters <vmx>` |
| `addNetworkAdapter` | 添加网卡 | `vmrun addNetworkAdapter <vmx> <类型> [网络名]` |
| `setNetworkAdapter` | 修改网卡设置 | `vmrun setNetworkAdapter <vmx> <索引> <类型> [网络名]` |
| `deleteNetworkAdapter` | 删除网卡 | `vmrun deleteNetworkAdapter <vmx> <索引>` |

网卡类型包括：`nat`、`bridged`、`hostonly`、`custom`。

### ~~1.4 宿主网络管理（Host Network Commands）~~ ✅ 已实现

> 端口转发管理已在本项目中实现（按 `f` 键），详见 `vmware-features-used.md`。

适合需要从宿主机访问虚拟机内服务的场景（如 SSH、Web 服务端口映射）。

### 1.5 客户操作系统交互（Guest OS Commands）

需要 VMware Tools 已安装在客户系统中，且需要 `-gu`/`-gp` 参数提供客户系统凭据。

**已实现（按 `g` 键进入文件浏览器）：**

| 命令 | 功能 | 状态 |
|------|------|------|
| ~~`listDirectoryInGuest`~~ | 列出客户系统中目录内容 | ✅ 已实现 |
| ~~`CopyFileFromHostToGuest`~~ | 从宿主复制文件到客户系统 | ✅ 已实现 |
| ~~`CopyFileFromGuestToHost`~~ | 从客户系统复制文件到宿主 | ✅ 已实现 |
| ~~`createDirectoryInGuest`~~ | 在客户系统中创建目录 | ✅ 已实现 |
| ~~`deleteFileInGuest`~~ | 在客户系统中删除文件 | ✅ 已实现 |

**未实现：**

| 命令 | 功能 |
|------|------|
| `runProgramInGuest` | 在客户系统中运行程序 |
| `runScriptInGuest` | 在客户系统中运行脚本 |
| `listProcessesInGuest` | 列出客户系统中的进程 |
| `killProcessInGuest` | 杀死客户系统中的进程 |
| `fileExistsInGuest` | 检查客户系统中文件是否存在 |
| `directoryExistsInGuest` | 检查客户系统中目录是否存在 |
| `deleteDirectoryInGuest` | 在客户系统中删除目录 |
| `renameFileInGuest` | 在客户系统中重命名文件 |
| `CreateTempfileInGuest` | 在客户系统中创建临时文件 |
| `typeKeystrokesInGuest` | 向客户系统发送键盘输入 |
| `captureScreen` | 截取虚拟机屏幕保存到宿主 |

### ~~1.6 共享文件夹（Shared Folders）~~ ✅ 已实现

> 共享文件夹管理已在本项目中实现（按 `h` 键），详见 `vmware-features-used.md`。

### 1.7 设备与变量

| 命令 | 功能 |
|------|------|
| `connectNamedDevice` | 连接指定设备（如 CD-ROM、USB） |
| `disconnectNamedDevice` | 断开指定设备 |
| `readVariable` | 读取 VM 运行时变量（runtimeConfig/guestEnv/guestVar） |
| `writeVariable` | 写入 VM 运行时变量 |

### 1.8 通用命令（General Commands）

| 命令 | 功能 | 用法 |
|------|------|------|
| ~~`getGuestIPAddress`~~ | ~~获取客户系统 IP~~ | ✅ 已实现 |
| `checkToolsState` | 检查 VMware Tools 状态 | `vmrun checkToolsState <vmx>` |
| `installTools` | 安装 VMware Tools | `vmrun installTools <vmx>` |
| `upgradevm` | 升级虚拟硬件版本 | `vmrun upgradevm <vmx>` |
| ~~`deleteVM`~~ | ~~删除虚拟机~~ | ✅ 已实现 |
| ~~`clone`~~ | ~~克隆虚拟机~~ | ✅ 已实现 |
| `downloadPhotonVM` | 下载 Photon OS 模板 | `vmrun downloadPhotonVM <路径>` |

> `getGuestIPAddress` 已集成到本项目中，详见 `vmware-features-used.md`。

---

## 二、vmware-vdiskmanager 未使用的功能

| 选项 | 功能 | 说明 |
|------|------|------|
| `-c` | 创建新磁盘 | 需配合 `-a`（适配器类型）、`-s`（大小）、`-t`（磁盘类型） |
| `-d` | 磁盘碎片整理 | 优化 sparse 磁盘的空间利用率 |
| `-k` | 收缩磁盘 | 回收客户系统中已删除文件占用的空间 |
| `-n` | 重命名磁盘 | 修改 vmdk 文件名及内部引用 |
| `-r` | 转换磁盘类型 | 如将 split 转为 single、sparse 转为 preallocated |
| `-R` | 一致性检查与修复 | 检查 sparse 磁盘完整性 |
| `-e` | 磁盘链一致性检查 | 检查快照链是否完整 |
| `-p` | 准备收缩（挂载点） | 在收缩前擦零已删除块 |
| `-D` | 设为可删除 | 用于从其他产品复制来的磁盘 |
| `-U` | 删除磁盘链接 | 删除单个磁盘链环节 |

磁盘类型 ID：
- `0` — 单文件可增长（growable）
- `1` — 多文件可增长（2GB 分片）
- `2` — 预分配单文件
- `3` — 预分配多文件
- `4` — ESX 预分配
- `5` — 流优化压缩
- `6` — 精简配置（ESX 3.x+）

---

## 三、vmrest — REST API 服务

**工具路径**: `/Applications/VMware Fusion.app/Contents/Library/vmrest`
**版本**: 1.3.1 build-24832108

vmrest 是 VMware Fusion Pro 内置的 HTTP REST API 服务，监听本地端口提供 JSON 接口。

### 配置与启动

```bash
# 首次使用需设置凭据
vmrest -C

# 启动服务（默认监听 http://127.0.0.1:8697）
vmrest -p 8697
```

### 可用 API 端点

| 方法 | 端点 | 功能 |
|------|------|------|
| GET | `/api/vms` | 列出所有已注册的虚拟机 |
| GET | `/api/vms/{id}` | 获取虚拟机详细信息 |
| GET | `/api/vms/{id}/power` | 获取电源状态 |
| PUT | `/api/vms/{id}/power` | 电源操作（on/off/suspend/pause/unpause） |
| GET | `/api/vms/{id}/nic` | 获取网络适配器信息 |
| PUT | `/api/vms/{id}/nic` | 修改网络适配器 |
| GET | `/api/vms/{id}/ip` | 获取 IP 地址 |
| GET/PUT | `/api/vms/{id}/sharedfolders` | 管理共享文件夹 |
| GET | `/api/vmnet` | 列出虚拟网络 |
| PUT | `/api/vmnet/{name}/portforward` | 配置端口转发 |

### 与 vmrun 的区别

| 对比项 | vmrun | vmrest |
|--------|-------|--------|
| 交互方式 | 命令行（每次启动进程） | HTTP 长驻服务 |
| 性能 | 每次 fork 进程，较慢 | 常驻进程，响应快 |
| 远程访问 | 不支持 | 支持（HTTP/HTTPS） |
| 客户操作系统交互 | 丰富（文件/进程/脚本） | 有限 |
| 加密 VM | 需 `-vp` 参数 | 请求体中传入密码 |

---

## 四、VMware OVF Tool

**工具路径**: `/Applications/VMware Fusion.app/Contents/Library/VMware OVF Tool/ovftool`

OVF Tool 用于虚拟机格式转换和部署。

### 主要功能

| 功能 | 说明 |
|------|------|
| VMX → OVA/OVF 导出 | 将本地 VM 导出为便携格式 |
| OVA/OVF → VMX 导入 | 从 OVA 包创建本地 VM |
| 部署到 ESXi/vCenter | 将 VM 直接部署到远程主机 |
| 格式信息查看 | 不指定 target 时显示源 VM 信息 |
| vTPM 设备添加 | 通过 `--addDevice:vtpm` 选项 |

```bash
# 查看 VM 信息
ovftool /path/to/vm.vmx

# 导出为 OVA
ovftool /path/to/vm.vmx /path/to/output.ova

# 从 OVA 导入
ovftool /path/to/input.ova /path/to/output.vmx
```

---

## 五、其他命令行工具

以下工具位于 VMware Fusion Library 目录中：

| 工具 | 功能 |
|------|------|
| `vmware-rawdiskCreator` | 创建 raw disk 映射（物理磁盘直通到 VM） |
| `vmnet-cli` | 管理虚拟网络（启动/停止/状态查询） |
| `vmnet-cfgcli` | 配置虚拟网络参数 |
| `vmnet-dhcpd` | 虚拟网络 DHCP 服务 |
| `vmnet-natd` | 虚拟网络 NAT 服务 |
| `vmnet-sniffer` | 虚拟网络抓包工具 |
| `vmss2core` | 将 .vmss（挂起文件）转换为 core dump 用于调试 |
| `mkisofs` | 创建 ISO 镜像 |
| `vmcli` | VMware 命令行接口（内部使用） |

---

## 六、功能优先级建议（如果未来要扩展本项目）

基于实用性和实现难度，建议按以下优先级接入：

| 优先级 | 功能 | 理由 |
|--------|------|------|
| ~~★★★~~ | ~~快照管理~~ | ✅ 已实现 |
| ~~★★★~~ | ~~`getGuestIPAddress`~~ | ✅ 已实现 |
| ~~★★☆~~ | ~~克隆 VM~~ | ✅ 已实现 |
| ~~★★☆~~ | ~~端口转发管理~~ | ✅ 已实现 |
| ~~★★☆~~ | ~~删除 VM~~ | ✅ 已实现 |
| ~~★☆☆~~ | ~~共享文件夹~~ | ✅ 已实现 |
| ~~★☆☆~~ | ~~客户系统文件操作~~ | ✅ 已实现 |
| ★☆☆ | vmrest 集成 | 大幅改变架构，适合 Web UI 场景 |
