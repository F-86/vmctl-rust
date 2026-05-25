use std::path::PathBuf;
use ratatui::widgets::TableState;
use crate::vmrun::PortForwarding;
use crate::vmx::{VmxFile, HardwareConfig};
use crate::vm::Vm;

/// 应用模式
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// VM 列表视图
    List,
    /// 配置详情视图
    Detail,
    /// 字段编辑模式
    Editing,
    /// 保存确认对话框
    Confirm,
    /// 快照列表视图
    Snapshot,
    /// 快照名称输入（创建快照）
    SnapshotInput,
    /// 快照操作确认
    SnapshotConfirm,
    /// 克隆 VM 名称输入
    CloneInput,
    /// 端口转发列表视图
    PortForward,
    /// 端口转发添加输入
    PortForwardInput,
    /// 端口转发删除确认
    PortForwardConfirm,
    /// 删除 VM 确认
    DeleteConfirm,
    /// 共享文件夹视图
    SharedFolder,
    /// 共享文件夹添加输入
    SharedFolderInput,
    /// 共享文件夹删除确认
    SharedFolderConfirm,
    /// 客户系统文件浏览器
    GuestFiles,
    /// 客户系统凭据输入
    GuestLogin,
    /// 客户系统文件操作输入（复制/创建目录）
    GuestFileInput,
    /// 客户系统文件删除确认
    GuestFileConfirm,
}

/// 可编辑的字段
#[derive(Debug, Clone, PartialEq)]
pub enum EditField {
    CpuCount,
    MemoryMb,
    DiskSize(usize),
}

/// 详情视图状态
pub struct DetailState {
    /// 原始 VMX 文件（用于修改和保存）
    pub vmx_file: VmxFile,
    /// 解析后的硬件配置
    pub config: HardwareConfig,
    /// 当前选中行索引
    pub selected_row: usize,
    /// 是否有未保存的修改
    pub dirty: bool,
    /// 对应的 VM 名称
    pub vm_name: String,
    /// 对应的 VM vmx 路径
    pub vmx_path: PathBuf,
}

/// 编辑状态
pub struct EditState {
    /// 正在编辑的字段
    pub field: EditField,
    /// 输入缓冲区
    pub buffer: String,
    /// 原始值（用于取消时恢复）
    pub original_value: String,
}

/// 详情视图中可交互的行（用于导航和编辑）
#[derive(Debug, Clone)]
pub struct DetailRow {
    /// 标签
    pub label: String,
    /// 显示值
    pub value: String,
    /// 是否可编辑
    pub editable: bool,
    /// 对应的编辑字段
    pub edit_field: Option<EditField>,
}

/// 快照视图状态
pub struct SnapshotState {
    /// 快照名列表
    pub snapshots: Vec<String>,
    /// 当前选中索引
    pub selected: usize,
    /// VM 名称
    pub vm_name: String,
    /// VM 的 vmx 路径
    pub vmx_path: PathBuf,
}

/// 快照名称输入状态
pub struct SnapshotInputState {
    /// 输入缓冲区
    pub buffer: String,
}

/// 快照操作类型（用于确认对话框）
#[derive(Debug, Clone)]
pub enum SnapshotAction {
    /// 删除快照
    Delete(String),
    /// 恢复到快照
    Revert(String),
}

/// 克隆 VM 输入状态
pub struct CloneInputState {
    /// 新 VM 名称输入缓冲区
    pub buffer: String,
    /// 源 VM 的 vmx 路径
    pub source_vmx: PathBuf,
    /// 源 VM 名称
    pub source_name: String,
    /// 克隆类型: full 或 linked
    pub clone_type: CloneType,
}

/// 克隆类型
#[derive(Debug, Clone, PartialEq)]
pub enum CloneType {
    Full,
    Linked,
}

impl CloneType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloneType::Full => "full",
            CloneType::Linked => "linked",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CloneType::Full => "完整克隆",
            CloneType::Linked => "链接克隆",
        }
    }
}

/// 端口转发视图状态
pub struct PortForwardState {
    /// 网络名称 (如 vmnet8)
    pub network: String,
    /// 转发规则列表
    pub rules: Vec<PortForwarding>,
    /// 当前选中索引
    pub selected: usize,
}

/// 端口转发输入状态（添加新规则）
pub struct PortForwardInputState {
    /// 当前输入字段索引 (0=协议, 1=宿主端口, 2=客户IP, 3=客户端口, 4=描述)
    pub field_index: usize,
    /// 各字段的值
    pub fields: [String; 5],
}

impl PortForwardInputState {
    pub fn new() -> Self {
        Self {
            field_index: 0,
            fields: [
                "tcp".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
        }
    }

    pub fn field_label(&self, index: usize) -> &'static str {
        match index {
            0 => "协议 (tcp/udp)",
            1 => "宿主端口",
            2 => "客户 IP",
            3 => "客户端口",
            4 => "描述",
            _ => "",
        }
    }

    pub fn current_field(&self) -> &str {
        &self.fields[self.field_index]
    }

    pub fn current_field_mut(&mut self) -> &mut String {
        &mut self.fields[self.field_index]
    }
}

/// 共享文件夹视图状态
pub struct SharedFolderState {
    /// VM 名称
    pub vm_name: String,
    /// VM vmx 路径
    pub vmx_path: PathBuf,
    /// 共享文件夹列表
    pub folders: Vec<SharedFolderEntry>,
    /// 当前选中索引
    pub selected: usize,
    /// 共享文件夹功能是否已启用
    pub enabled: bool,
}

/// 共享文件夹条目
#[derive(Debug, Clone)]
pub struct SharedFolderEntry {
    pub name: String,
    pub host_path: String,
    pub writable: bool,
}

/// 共享文件夹添加输入状态
pub struct SharedFolderInputState {
    /// 当前字段索引 (0=名称, 1=宿主路径)
    pub field_index: usize,
    /// 字段值
    pub fields: [String; 2],
}

impl SharedFolderInputState {
    pub fn new() -> Self {
        Self {
            field_index: 0,
            fields: [String::new(), String::new()],
        }
    }

    pub fn field_label(&self, index: usize) -> &'static str {
        match index {
            0 => "共享名称",
            1 => "宿主路径",
            _ => "",
        }
    }
}

/// 客户系统文件浏览器状态
pub struct GuestFilesState {
    /// VM vmx 路径
    pub vmx_path: PathBuf,
    /// VM 名称
    pub vm_name: String,
    /// 客户系统凭据
    pub user: String,
    pub pass: String,
    /// 当前目录
    pub current_dir: String,
    /// 目录内容
    pub entries: Vec<String>,
    /// 当前选中索引
    pub selected: usize,
}

/// 客户系统凭据输入状态
pub struct GuestLoginState {
    /// 当前字段 (0=用户名, 1=密码)
    pub field_index: usize,
    /// 字段值
    pub fields: [String; 2],
    /// 目标 VM
    pub vmx_path: PathBuf,
    pub vm_name: String,
}

/// 客户系统文件操作输入
pub struct GuestFileInputState {
    /// 操作类型
    pub action: GuestFileAction,
    /// 输入缓冲区
    pub buffer: String,
}

/// 客户系统文件操作类型
#[derive(Debug, Clone)]
pub enum GuestFileAction {
    /// 上传：输入宿主文件路径
    Upload,
    /// 下载：输入宿主保存路径
    Download(String),
    /// 创建目录：输入目录名
    Mkdir,
}

/// 虚拟机列表状态
pub struct VmListState {
    pub state: TableState,
    pub selected_vmx: Option<PathBuf>,
}

impl VmListState {
    pub fn new() -> Self {
        VmListState {
            state: TableState::default(),
            selected_vmx: None,
        }
    }

    pub fn with_selection(&mut self, vms: &[Vm], index: Option<usize>) {
        if let Some(i) = index {
            self.state.select(Some(i));
            if i < vms.len() {
                self.selected_vmx = Some(vms[i].vmx_path.clone());
            }
        } else {
            self.state.select(None);
            self.selected_vmx = None;
        }
    }
}
