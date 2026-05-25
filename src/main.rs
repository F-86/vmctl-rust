use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use sysinfo::System;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, TableState, Paragraph, Table, Row, Cell},
};

mod vm;
mod vmrun;
mod manager;
mod vmx;

use vm::{Vm, VmState};
use manager::VmManager;
use vmrun::{set_vmrun_path, Vmrun};
use vmx::{VmxFile, HardwareConfig};

/// 应用模式
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
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
enum EditField {
    CpuCount,
    MemoryMb,
    DiskSize(usize),  // 索引对应 HardwareConfig.disks 中的位置
}

/// 详情视图状态
struct DetailState {
    /// 原始 VMX 文件（用于修改和保存）
    vmx_file: VmxFile,
    /// 解析后的硬件配置
    config: HardwareConfig,
    /// 当前选中行索引
    selected_row: usize,
    /// 是否有未保存的修改
    dirty: bool,
    /// 对应的 VM 名称
    vm_name: String,
    /// 对应的 VM vmx 路径
    vmx_path: PathBuf,
}

/// 编辑状态
struct EditState {
    /// 正在编辑的字段
    field: EditField,
    /// 输入缓冲区
    buffer: String,
    /// 原始值（用于取消时恢复）
    original_value: String,
}

/// 详情视图中可交互的行（用于导航和编辑）
#[derive(Debug, Clone)]
struct DetailRow {
    /// 标签
    label: String,
    /// 显示值
    value: String,
    /// 是否可编辑
    editable: bool,
    /// 对应的编辑字段
    edit_field: Option<EditField>,
}

/// 快照视图状态
struct SnapshotState {
    /// 快照名列表
    snapshots: Vec<String>,
    /// 当前选中索引
    selected: usize,
    /// VM 名称
    vm_name: String,
    /// VM 的 vmx 路径
    vmx_path: PathBuf,
}

/// 快照名称输入状态
struct SnapshotInputState {
    /// 输入缓冲区
    buffer: String,
}

/// 快照操作类型（用于确认对话框）
#[derive(Debug, Clone)]
enum SnapshotAction {
    /// 删除快照
    Delete(String),
    /// 恢复到快照
    Revert(String),
}

/// 克隆 VM 输入状态
struct CloneInputState {
    /// 新 VM 名称输入缓冲区
    buffer: String,
    /// 源 VM 的 vmx 路径
    source_vmx: PathBuf,
    /// 源 VM 名称
    source_name: String,
    /// 克隆类型: full 或 linked
    clone_type: CloneType,
}

/// 克隆类型
#[derive(Debug, Clone, PartialEq)]
enum CloneType {
    Full,
    Linked,
}

impl CloneType {
    fn as_str(&self) -> &'static str {
        match self {
            CloneType::Full => "full",
            CloneType::Linked => "linked",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            CloneType::Full => "完整克隆",
            CloneType::Linked => "链接克隆",
        }
    }
}

/// 端口转发视图状态
struct PortForwardState {
    /// 网络名称 (如 vmnet8)
    network: String,
    /// 转发规则列表
    rules: Vec<vmrun::PortForwarding>,
    /// 当前选中索引
    selected: usize,
}

/// 端口转发输入状态（添加新规则）
struct PortForwardInputState {
    /// 当前输入字段索引 (0=协议, 1=宿主端口, 2=客户IP, 3=客户端口, 4=描述)
    field_index: usize,
    /// 各字段的值
    fields: [String; 5],
}

impl PortForwardInputState {
    fn new() -> Self {
        Self {
            field_index: 0,
            fields: [
                "tcp".to_string(),     // 协议
                String::new(),         // 宿主端口
                String::new(),         // 客户 IP
                String::new(),         // 客户端口
                String::new(),         // 描述
            ],
        }
    }

    fn field_label(&self, index: usize) -> &'static str {
        match index {
            0 => "协议 (tcp/udp)",
            1 => "宿主端口",
            2 => "客户 IP",
            3 => "客户端口",
            4 => "描述",
            _ => "",
        }
    }

    fn current_field(&self) -> &str {
        &self.fields[self.field_index]
    }

    fn current_field_mut(&mut self) -> &mut String {
        &mut self.fields[self.field_index]
    }
}

/// 共享文件夹视图状态
struct SharedFolderState {
    /// VM 名称
    vm_name: String,
    /// VM vmx 路径
    vmx_path: PathBuf,
    /// 共享文件夹列表 (名称, 宿主路径, 是否可写)
    folders: Vec<SharedFolderEntry>,
    /// 当前选中索引
    selected: usize,
    /// 共享文件夹功能是否已启用
    enabled: bool,
}

/// 共享文件夹条目
#[derive(Debug, Clone)]
struct SharedFolderEntry {
    name: String,
    host_path: String,
    writable: bool,
}

/// 共享文件夹添加输入状态
struct SharedFolderInputState {
    /// 当前字段索引 (0=名称, 1=宿主路径)
    field_index: usize,
    /// 字段值
    fields: [String; 2],
}

impl SharedFolderInputState {
    fn new() -> Self {
        Self {
            field_index: 0,
            fields: [String::new(), String::new()],
        }
    }

    fn field_label(&self, index: usize) -> &'static str {
        match index {
            0 => "共享名称",
            1 => "宿主路径",
            _ => "",
        }
    }
}

/// 客户系统文件浏览器状态
struct GuestFilesState {
    /// VM vmx 路径
    vmx_path: PathBuf,
    /// VM 名称
    vm_name: String,
    /// 客户系统凭据
    user: String,
    pass: String,
    /// 当前目录
    current_dir: String,
    /// 目录内容
    entries: Vec<String>,
    /// 当前选中索引
    selected: usize,
}

/// 客户系统凭据输入状态
struct GuestLoginState {
    /// 当前字段 (0=用户名, 1=密码)
    field_index: usize,
    /// 字段值
    fields: [String; 2],
    /// 目标 VM
    vmx_path: PathBuf,
    vm_name: String,
}

/// 客户系统文件操作输入
struct GuestFileInputState {
    /// 操作类型
    action: GuestFileAction,
    /// 输入缓冲区
    buffer: String,
}

/// 客户系统文件操作类型
#[derive(Debug, Clone)]
enum GuestFileAction {
    /// 上传：输入宿主文件路径
    Upload,
    /// 下载：输入宿主保存路径
    Download(String),
    /// 创建目录：输入目录名
    Mkdir,
}

/// 配置文件结构
struct Config {
    vmrun_path: String,
    vm_dir: PathBuf,
    refresh_interval: u64,
}

impl Config {
    fn load() -> Self {
        // 默认值
        let mut vmrun_path = "/Applications/VMware Fusion.app/Contents/Library/vmrun".to_string();
        let mut vm_dir = PathBuf::from("/Users/jane/Virtual Machines.localized");
        let mut refresh_interval = 5u64;

        // 读取 config.toml
        if let Ok(content) = std::fs::read_to_string("config.toml") {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim();
                    let value = line[eq_pos + 1..].trim();
                    match key {
                        "vmrun_path" => vmrun_path = value.to_string(),
                        "vm_dir" => vm_dir = PathBuf::from(value),
                        "refresh_interval" => {
                            if let Ok(interval) = value.parse() {
                                refresh_interval = interval;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // 将 vmrun_path 写入 vmrun 模块
        vmrun::set_vmrun_path(vmrun_path.clone());

        Config {
            vmrun_path,
            vm_dir,
            refresh_interval,
        }
    }
}

/// 读取 ASCII 艺术字
fn load_ascii() -> String {
    if let Ok(content) = std::fs::read_to_string("ascii.txt") {
        content
    } else {
        // 默认艺术字
        "__     ____  __  ____ _____ _     
\\ \\   / /  \\/  |/ ___|_   _| |    
 \\ \\ / /| |\\/| | |     | | | |    
  \\ V / | |  | | |___  | | | |___ 
   \\_/  |_|  |_|\\____| |_| |_____|
".to_string()
    }
}

/// 虚拟机列表状态
struct VmListState {
    state: TableState,
    selected_vmx: Option<PathBuf>,
}

impl VmListState {
    fn new() -> Self {
        VmListState {
            state: TableState::default(),
            selected_vmx: None,
        }
    }

    fn with_selection(&mut self, vms: &[Vm], index: Option<usize>) {
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

/// 渲染 UI
fn ui(frame: &mut Frame, vms: &[Vm], list_state: &mut VmListState, message: &Option<String>, vm_count: usize, ascii_art: &str, cpu_usage: f32, mem_usage: f32) {
    let area = frame.area();

    // 主布局：标题栏 + 表格区域
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // 标题栏（包含 ASCII art）
            Constraint::Min(1),     // 表格区域
        ])
        .split(area);

    // 顶部标题栏（ASCII art + 操作提示 + 系统资源）
    render_header(frame, chunks[0], vm_count, ascii_art, cpu_usage, mem_usage);

    // 表格区域
    render_table(frame, chunks[1], vms, list_state);

    // 如果有消息，在表格下方显示
    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray))
            .alignment(Alignment::Left);
        frame.render_widget(msg_para, Rect::new(chunks[1].x, chunks[1].y + chunks[1].height.saturating_sub(3), chunks[1].width, 3));
    }
}

/// 渲染顶部标题栏（ASCII 艺术字 + 操作提示 + 系统资源）
fn render_header(frame: &mut Frame, area: Rect, vm_count: usize, ascii_art: &str, cpu_usage: f32, mem_usage: f32) {
    let block = Block::default()
        .style(Style::new().bg(Color::Black))
        .borders(Borders::BOTTOM);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 右侧：ASCII art (logo)
    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter()
        .map(|line| format!("{}\n", line))
        .collect::<String>();

    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan))
        .alignment(Alignment::Right);

    // 限制 ASCII art 的显示区域
    let ascii_width = 55.min(inner.width / 2);
    let ascii_area = Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height);
    frame.render_widget(ascii_para, ascii_area);

    // 左侧：操作提示（K9S 风格，三列竖排）
    let col1 = vec![
        Line::from(vec![Span::styled("<w/s>", Style::new().fg(Color::Yellow)), Span::raw(" navigate")]),
        Line::from(vec![Span::styled("<enter>", Style::new().fg(Color::Green)), Span::raw(" start")]),
        Line::from(vec![Span::styled("<x>", Style::new().fg(Color::Yellow)), Span::raw(" stop")]),
        Line::from(vec![Span::styled("<p>", Style::new().fg(Color::Yellow)), Span::raw(" pause")]),
        Line::from(vec![Span::styled("<r>", Style::new().fg(Color::Yellow)), Span::raw(" resume")]),
    ];
    let col2 = vec![
        Line::from(vec![Span::styled("<i>", Style::new().fg(Color::Yellow)), Span::raw(" info")]),
        Line::from(vec![Span::styled("<n>", Style::new().fg(Color::Yellow)), Span::raw(" snapshot")]),
        Line::from(vec![Span::styled("<c>", Style::new().fg(Color::Yellow)), Span::raw(" clone")]),
        Line::from(vec![Span::styled("<f>", Style::new().fg(Color::Yellow)), Span::raw(" forward")]),
        Line::from(vec![Span::styled("<h>", Style::new().fg(Color::Yellow)), Span::raw(" share")]),
    ];
    let col3 = vec![
        Line::from(vec![Span::styled("<g>", Style::new().fg(Color::Yellow)), Span::raw(" guest")]),
        Line::from(vec![Span::styled("<D>", Style::new().fg(Color::Red)), Span::raw(" delete")]),
        Line::from(vec![Span::styled("<q>", Style::new().fg(Color::Yellow)), Span::raw(" quit")]),
    ];

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 3;
    frame.render_widget(
        Paragraph::new(Text::from(col1)).alignment(Alignment::Left),
        Rect::new(inner.x, inner.y, col_width, 5),
    );
    frame.render_widget(
        Paragraph::new(Text::from(col2)).alignment(Alignment::Left),
        Rect::new(inner.x + col_width, inner.y, col_width, 5),
    );
    frame.render_widget(
        Paragraph::new(Text::from(col3)).alignment(Alignment::Left),
        Rect::new(inner.x + col_width * 2, inner.y, col_width, 5),
    );

    // 左下角：系统资源 + VM 数量
    let sys_text = format!("CPU: {:.0}%  MEM: {:.0}%   VMs: {}", cpu_usage, mem_usage, vm_count);
    let sys_para = Paragraph::new(Text::from(sys_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(sys_para, Rect::new(inner.x, inner.y + 5, 35, 1));
}

/// 渲染表格
fn render_table(frame: &mut Frame, area: Rect, vms: &[Vm], list_state: &mut VmListState) {
    // 表格列定义（去掉 VMX PATH）
    let col_widths = &[
        Constraint::Length(30),  // 名称
        Constraint::Length(15), // 状态
        Constraint::Length(18), // IP
    ];

    // 表头
    let header = Row::new(vec![
        Cell::from(Span::raw(" NAME")),
        Cell::from(Span::raw(" STATE")),
        Cell::from(Span::raw(" IP")),
    ])
    .style(Style::new().fg(Color::White).bg(Color::DarkGray));

    // 数据行
    let rows: Vec<Row> = vms.iter().enumerate().map(|(i, vm)| {
        let is_selected = list_state.state.selected() == Some(i);
        
        let name_prefix = if is_selected { "▶" } else { " " };
        let name_cell = Cell::from(Span::raw(format!("{} {}", name_prefix, vm.name)));

        let state_color = match vm.state {
            VmState::Running => Color::Green,
            VmState::Stopped => Color::Red,
            VmState::Paused => Color::Yellow,
            VmState::Unknown => Color::DarkGray,
        };
        let state_cell = Cell::from(Span::styled(
            format!("● {}", vm.state.display_name()),
            Style::new().fg(state_color),
        ));

        let ip_text = vm.ip.as_deref().unwrap_or("-");
        let ip_cell = Cell::from(Span::styled(ip_text, Style::new().fg(Color::Cyan)));

        let row = Row::new(vec![name_cell, state_cell, ip_cell]);
        
        if is_selected {
            row.style(Style::new().bg(Color::Blue).fg(Color::White))
        } else if i % 2 == 0 {
            row.style(Style::new().bg(Color::Black))
        } else {
            row.style(Style::new().bg(Color::Rgb(30, 30, 30)))
        }
    }).collect();

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
        .column_spacing(1);

    frame.render_stateful_widget(table, area, &mut list_state.state);
}

/// 获取详情视图的行数据
fn get_detail_rows(ds: &DetailState) -> Vec<DetailRow> {
    let mut rows = Vec::new();

    // 硬件信息
    rows.push(DetailRow {
        label: "CPU".to_string(),
        value: format!("{}", ds.config.cpu_count),
        editable: true,
        edit_field: Some(EditField::CpuCount),
    });
    rows.push(DetailRow {
        label: "内存".to_string(),
        value: format!("{}", ds.config.memory_mb),
        editable: true,
        edit_field: Some(EditField::MemoryMb),
    });
    rows.push(DetailRow {
        label: "系统".to_string(),
        value: ds.config.guest_os.clone(),
        editable: false,
        edit_field: None,
    });
    rows.push(DetailRow {
        label: "固件".to_string(),
        value: ds.config.firmware.clone(),
        editable: false,
        edit_field: None,
    });

    // 网络适配器
    for net in &ds.config.networks {
        let mac_str = net.mac_address.as_deref().unwrap_or("-");
        rows.push(DetailRow {
            label: format!("网络 eth{}", net.index),
            value: format!("{} ({})", net.connection_type, mac_str),
            editable: false,
            edit_field: None,
        });
    }

    // 磁盘
    for (idx, disk) in ds.config.disks.iter().enumerate() {
        let size_str = disk.size_gb
            .map(|gb| format!("{}GB", gb))
            .unwrap_or_else(|| "未知".to_string());
        rows.push(DetailRow {
            label: format!("磁盘 {}:{}", disk.controller, disk.unit),
            value: format!("{} ({})", disk.file_name, size_str),
            editable: disk.size_gb.is_some(),
            edit_field: Some(EditField::DiskSize(idx)),
        });
    }

    rows
}

/// 渲染配置详情视图
fn render_detail_view(frame: &mut Frame, ds: &DetailState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();

    // 主布局：header + 表格区域（和主界面一致）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),   // 顶部 header（logo + 快捷键）
            Constraint::Min(1),      // 配置表格区域
        ])
        .split(area);

    // 顶部 header（和主界面格式一致：左侧快捷键 + 右侧 logo）
    render_detail_header(frame, chunks[0], ds, ascii_art);

    // 配置表格区域
    render_detail_table(frame, chunks[1], ds);

    // 如果有消息，在表格下方显示
    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray))
            .alignment(Alignment::Left);
        frame.render_widget(msg_para, Rect::new(
            chunks[1].x,
            chunks[1].y + chunks[1].height.saturating_sub(3),
            chunks[1].width,
            3,
        ));
    }
}

/// 渲染详情视图的顶部 header（logo + 快捷键提示）
fn render_detail_header(frame: &mut Frame, area: Rect, ds: &DetailState, ascii_art: &str) {
    let block = Block::default()
        .style(Style::new().bg(Color::Black))
        .borders(Borders::BOTTOM);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 右侧：ASCII art (logo)
    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter()
        .map(|line| format!("{}\n", line))
        .collect::<String>();

    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan))
        .alignment(Alignment::Right);

    let ascii_width = 55.min(inner.width / 2);
    let ascii_area = Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height);
    frame.render_widget(ascii_para, ascii_area);

    // 左侧：操作提示（K9S 风格，两列竖排）
    let left_col = vec![
        Line::from(vec![
            Span::styled("<w/s>", Style::new().fg(Color::Yellow)),
            Span::raw(" navigate"),
        ]),
        Line::from(vec![
            Span::styled("<e>", Style::new().fg(Color::Green)),
            Span::raw(" edit"),
        ]),
        Line::from(vec![
            Span::styled("<W>", Style::new().fg(Color::Yellow)),
            Span::raw(" save"),
        ]),
    ];

    let right_col = vec![
        Line::from(vec![
            Span::styled("<u>", Style::new().fg(Color::Yellow)),
            Span::raw(" undo"),
        ]),
        Line::from(vec![
            Span::styled("<esc>", Style::new().fg(Color::Yellow)),
            Span::raw(" back"),
        ]),
        Line::from(vec![
            Span::styled("<i>", Style::new().fg(Color::Yellow)),
            Span::raw(" back"),
        ]),
    ];

    let col1_para = Paragraph::new(Text::from(left_col))
        .style(Style::new().fg(Color::White))
        .alignment(Alignment::Left);

    let col2_para = Paragraph::new(Text::from(right_col))
        .style(Style::new().fg(Color::White))
        .alignment(Alignment::Left);

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 2;
    frame.render_widget(col1_para, Rect::new(inner.x, inner.y + 1, col_width, 4));
    frame.render_widget(col2_para, Rect::new(inner.x + col_width, inner.y + 1, col_width, 4));

    // 左下角：VM 名称 + 状态
    let dirty_mark = if ds.dirty { " [已修改]" } else { "" };
    let info_text = format!("VM: {}{}", ds.vm_name, dirty_mark);
    let info_para = Paragraph::new(Text::from(info_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(info_para, Rect::new(inner.x, inner.y + 5, 50, 1));
}

/// 渲染详情视图的配置表格
fn render_detail_table(frame: &mut Frame, area: Rect, ds: &DetailState) {
    let rows = get_detail_rows(ds);

    // 表格列定义
    let col_widths = &[
        Constraint::Length(3),   // 编辑标记
        Constraint::Length(16),  // 配置项名称
        Constraint::Min(20),    // 值
    ];

    // 表头
    let header = Row::new(vec![
        Cell::from(Span::raw(" ")),
        Cell::from(Span::raw(" PROPERTY")),
        Cell::from(Span::raw(" VALUE")),
    ])
    .style(Style::new().fg(Color::White).bg(Color::DarkGray));

    // 数据行
    let table_rows: Vec<Row> = rows.iter().enumerate().map(|(i, row)| {
        let is_selected = i == ds.selected_row;

        let marker = if row.editable { "✎" } else { " " };
        let marker_cell = Cell::from(Span::styled(
            format!(" {}", marker),
            Style::new().fg(Color::DarkGray),
        ));

        let label_cell = Cell::from(Span::styled(
            format!(" {}", row.label),
            Style::new().fg(Color::Yellow),
        ));

        let unit = match row.edit_field {
            Some(EditField::CpuCount) => " 核",
            Some(EditField::MemoryMb) => " MB",
            Some(EditField::DiskSize(_)) => "",
            None => "",
        };
        let value_cell = Cell::from(Span::styled(
            format!(" {}{}", row.value, unit),
            if row.editable && ds.dirty {
                Style::new().fg(Color::Green)
            } else {
                Style::new().fg(Color::White)
            },
        ));

        let row_widget = Row::new(vec![marker_cell, label_cell, value_cell]);

        if is_selected {
            row_widget.style(Style::new().bg(Color::Blue).fg(Color::White))
        } else if i % 2 == 0 {
            row_widget.style(Style::new().bg(Color::Black))
        } else {
            row_widget.style(Style::new().bg(Color::Rgb(30, 30, 30)))
        }
    }).collect();

    let table = Table::new(table_rows, col_widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
        .column_spacing(1);

    let mut table_state = TableState::default();
    table_state.select(Some(ds.selected_row));
    frame.render_stateful_widget(table, area, &mut table_state);
}

/// 渲染编辑覆盖层（输入框）
fn render_edit_overlay(frame: &mut Frame, es: &EditState) {
    let area = frame.area();

    // 在屏幕中央显示输入框
    let popup_width = 40u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // 清除背景
    let clear_block = Block::default()
        .style(Style::new().bg(Color::Black));
    frame.render_widget(clear_block, popup_area);

    let field_name = match es.field {
        EditField::CpuCount => "CPU 核心数".to_string(),
        EditField::MemoryMb => "内存 (MB)".to_string(),
        EditField::DiskSize(_) => "磁盘大小 (GB) - 仅支持扩容".to_string(),
    };

    let title = format!(" 编辑: {} ", field_name);
    let input_text = format!(" > {}_ ", es.buffer);

    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let input_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(input_text, Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [Enter] 确认  [Esc] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(input_para, inner);
}

/// 渲染保存确认对话框
fn render_confirm_dialog(frame: &mut Frame) {
    let area = frame.area();

    let popup_width = 36u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 确认保存 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let confirm_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(" 保存修改到 .vmx 文件？", Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [y] 确认  [n] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(confirm_para, inner);
}

/// 渲染快照列表视图
fn render_snapshot_view(frame: &mut Frame, ss: &SnapshotState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();

    // 主布局：header + 表格区域（和主界面一致）
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),   // 顶部 header
            Constraint::Min(1),      // 快照表格
        ])
        .split(area);

    // 顶部 header
    render_snapshot_header(frame, chunks[0], ss, ascii_art);

    // 快照表格
    render_snapshot_table(frame, chunks[1], ss);

    // 消息提示
    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray))
            .alignment(Alignment::Left);
        frame.render_widget(msg_para, Rect::new(
            chunks[1].x,
            chunks[1].y + chunks[1].height.saturating_sub(3),
            chunks[1].width,
            3,
        ));
    }
}

/// 渲染快照视图的 header
fn render_snapshot_header(frame: &mut Frame, area: Rect, ss: &SnapshotState, ascii_art: &str) {
    let block = Block::default()
        .style(Style::new().bg(Color::Black))
        .borders(Borders::BOTTOM);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 右侧：ASCII art (logo)
    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter()
        .map(|line| format!("{}\n", line))
        .collect::<String>();

    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan))
        .alignment(Alignment::Right);

    let ascii_width = 55.min(inner.width / 2);
    let ascii_area = Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height);
    frame.render_widget(ascii_para, ascii_area);

    // 左侧：操作提示
    let left_col = vec![
        Line::from(vec![
            Span::styled("<w/s>", Style::new().fg(Color::Yellow)),
            Span::raw(" navigate"),
        ]),
        Line::from(vec![
            Span::styled("<c>", Style::new().fg(Color::Green)),
            Span::raw(" create"),
        ]),
        Line::from(vec![
            Span::styled("<d>", Style::new().fg(Color::Red)),
            Span::raw(" delete"),
        ]),
    ];

    let right_col = vec![
        Line::from(vec![
            Span::styled("<r>", Style::new().fg(Color::Yellow)),
            Span::raw(" revert"),
        ]),
        Line::from(vec![
            Span::styled("<esc>", Style::new().fg(Color::Yellow)),
            Span::raw(" back"),
        ]),
        Line::from(vec![
            Span::styled("<n>", Style::new().fg(Color::Yellow)),
            Span::raw(" back"),
        ]),
    ];

    let col1_para = Paragraph::new(Text::from(left_col))
        .style(Style::new().fg(Color::White))
        .alignment(Alignment::Left);

    let col2_para = Paragraph::new(Text::from(right_col))
        .style(Style::new().fg(Color::White))
        .alignment(Alignment::Left);

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 2;
    frame.render_widget(col1_para, Rect::new(inner.x, inner.y + 1, col_width, 4));
    frame.render_widget(col2_para, Rect::new(inner.x + col_width, inner.y + 1, col_width, 4));

    // 左下角：VM 名称 + 快照数
    let info_text = format!("Snapshots: {} ({})", ss.vm_name, ss.snapshots.len());
    let info_para = Paragraph::new(Text::from(info_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(info_para, Rect::new(inner.x, inner.y + 5, 50, 1));
}

/// 渲染快照表格
fn render_snapshot_table(frame: &mut Frame, area: Rect, ss: &SnapshotState) {
    if ss.snapshots.is_empty() {
        // 无快照提示
        let empty_para = Paragraph::new(Text::from("\n  (无快照)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty_para, area);
        return;
    }

    let col_widths = &[
        Constraint::Length(3),   // 选中标记
        Constraint::Min(30),    // 快照名称
    ];

    let header = Row::new(vec![
        Cell::from(Span::raw(" ")),
        Cell::from(Span::raw(" SNAPSHOT NAME")),
    ])
    .style(Style::new().fg(Color::White).bg(Color::DarkGray));

    let rows: Vec<Row> = ss.snapshots.iter().enumerate().map(|(i, name)| {
        let is_selected = i == ss.selected;
        let marker = if is_selected { " ▶" } else { "  " };

        let row = Row::new(vec![
            Cell::from(Span::raw(marker)),
            Cell::from(Span::styled(
                format!(" {}", name),
                Style::new().fg(Color::White),
            )),
        ]);

        if is_selected {
            row.style(Style::new().bg(Color::Blue).fg(Color::White))
        } else if i % 2 == 0 {
            row.style(Style::new().bg(Color::Black))
        } else {
            row.style(Style::new().bg(Color::Rgb(30, 30, 30)))
        }
    }).collect();

    let table = Table::new(rows, col_widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
        .column_spacing(1);

    let mut table_state = TableState::default();
    table_state.select(Some(ss.selected));
    frame.render_stateful_widget(table, area, &mut table_state);
}

/// 渲染快照名称输入框
fn render_snapshot_input(frame: &mut Frame, sis: &SnapshotInputState) {
    let area = frame.area();

    let popup_width = 44u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 创建快照 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Green).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let input_text = format!(" > {}_ ", sis.buffer);
    let input_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(input_text, Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [Enter] 确认  [Esc] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(input_para, inner);
}

/// 渲染快照操作确认对话框
fn render_snapshot_confirm(frame: &mut Frame, action: &SnapshotAction) {
    let area = frame.area();

    let popup_width = 44u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let (title, prompt) = match action {
        SnapshotAction::Delete(name) => (
            " 确认删除 ",
            format!(" 删除快照 \"{}\"？", name),
        ),
        SnapshotAction::Revert(name) => (
            " 确认恢复 ",
            format!(" 恢复到快照 \"{}\"？", name),
        ),
    };

    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let confirm_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(prompt, Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [y] 确认  [n] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(confirm_para, inner);
}

/// 渲染克隆 VM 输入框
fn render_clone_input(frame: &mut Frame, ci: &CloneInputState) {
    let area = frame.area();

    let popup_width = 52u16.min(area.width - 4);
    let popup_height = 7u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = format!(" 克隆: {} ", ci.source_name);
    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Cyan).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let type_label = format!(" 类型: {} [Tab 切换]", ci.clone_type.display_name());
    let input_text = format!(" 名称: {}_ ", ci.buffer);

    let input_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(type_label, Style::new().fg(Color::Yellow))),
        Line::from(Span::styled(input_text, Style::new().fg(Color::White))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            " [Enter] 克隆  [Tab] 切换类型  [Esc] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(input_para, inner);
}

/// 渲染端口转发列表视图
fn render_portfwd_view(frame: &mut Frame, pfs: &PortForwardState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    // Header
    render_portfwd_header(frame, chunks[0], pfs, ascii_art);

    // 表格
    if pfs.rules.is_empty() {
        let empty = Paragraph::new(Text::from("\n  (无端口转发规则)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let col_widths = &[
            Constraint::Length(8),   // 协议
            Constraint::Length(12),  // 宿主端口
            Constraint::Length(22),  // 客户 IP:端口
            Constraint::Min(15),    // 描述
        ];

        let header = Row::new(vec![
            Cell::from(Span::raw(" PROTO")),
            Cell::from(Span::raw(" HOST PORT")),
            Cell::from(Span::raw(" GUEST")),
            Cell::from(Span::raw(" DESCRIPTION")),
        ]).style(Style::new().fg(Color::White).bg(Color::DarkGray));

        let rows: Vec<Row> = pfs.rules.iter().enumerate().map(|(i, rule)| {
            let is_selected = i == pfs.selected;
            let row = Row::new(vec![
                Cell::from(Span::raw(format!(" {}", rule.protocol))),
                Cell::from(Span::raw(format!(" {}", rule.host_port))),
                Cell::from(Span::raw(format!(" {}:{}", rule.guest_ip, rule.guest_port))),
                Cell::from(Span::raw(format!(" {}", rule.description))),
            ]);
            if is_selected {
                row.style(Style::new().bg(Color::Blue).fg(Color::White))
            } else if i % 2 == 0 {
                row.style(Style::new().bg(Color::Black))
            } else {
                row.style(Style::new().bg(Color::Rgb(30, 30, 30)))
            }
        }).collect();

        let table = Table::new(rows, col_widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
            .column_spacing(1);

        let mut table_state = TableState::default();
        table_state.select(Some(pfs.selected));
        frame.render_stateful_widget(table, chunks[1], &mut table_state);
    }

    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray));
        frame.render_widget(msg_para, Rect::new(
            chunks[1].x, chunks[1].y + chunks[1].height.saturating_sub(3), chunks[1].width, 3,
        ));
    }
}

/// 渲染端口转发 header
fn render_portfwd_header(frame: &mut Frame, area: Rect, pfs: &PortForwardState, ascii_art: &str) {
    let block = Block::default().style(Style::new().bg(Color::Black)).borders(Borders::BOTTOM);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter().map(|l| format!("{}\n", l)).collect();
    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan)).alignment(Alignment::Right);
    let ascii_width = 55.min(inner.width / 2);
    let ascii_area = Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height);
    frame.render_widget(ascii_para, ascii_area);

    let left_col = vec![
        Line::from(vec![Span::styled("<w/s>", Style::new().fg(Color::Yellow)), Span::raw(" navigate")]),
        Line::from(vec![Span::styled("<a>", Style::new().fg(Color::Green)), Span::raw(" add")]),
        Line::from(vec![Span::styled("<d>", Style::new().fg(Color::Red)), Span::raw(" delete")]),
    ];
    let right_col = vec![
        Line::from(vec![Span::styled("<esc>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
        Line::from(vec![Span::styled("<f>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
    ];

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 2;
    frame.render_widget(Paragraph::new(Text::from(left_col)).alignment(Alignment::Left),
        Rect::new(inner.x, inner.y + 1, col_width, 4));
    frame.render_widget(Paragraph::new(Text::from(right_col)).alignment(Alignment::Left),
        Rect::new(inner.x + col_width, inner.y + 1, col_width, 4));

    let info = format!("Port Forwarding: {} ({})", pfs.network, pfs.rules.len());
    frame.render_widget(
        Paragraph::new(Text::from(info)).style(Style::new().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 5, 50, 1),
    );
}

/// 渲染端口转发添加输入框
fn render_portfwd_input(frame: &mut Frame, pfi: &PortForwardInputState) {
    let area = frame.area();
    let popup_width = 50u16.min(area.width - 4);
    let popup_height = 10u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 添加端口转发 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Green).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let mut lines = Vec::new();
    for i in 0..5 {
        let marker = if i == pfi.field_index { "▶" } else { " " };
        let value_display = if i == pfi.field_index {
            format!("{}_ ", pfi.fields[i])
        } else {
            format!("{} ", pfi.fields[i])
        };
        let style = if i == pfi.field_index {
            Style::new().fg(Color::White)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} {}: ", marker, pfi.field_label(i)), style),
            Span::styled(value_display, Style::new().fg(Color::Cyan)),
        ]));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " [Tab] 下一字段  [Enter] 确认  [Esc] 取消",
        Style::new().fg(Color::DarkGray),
    )));

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// 渲染端口转发删除确认
fn render_portfwd_delete_confirm(frame: &mut Frame, pfs: &PortForwardState) {
    let area = frame.area();
    let popup_width = 44u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 确认删除 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let prompt = if let Some(rule) = pfs.rules.get(pfs.selected) {
        format!(" 删除 {} :{} ?", rule.protocol, rule.host_port)
    } else {
        " 删除此规则？".to_string()
    };

    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(prompt, Style::new().fg(Color::White))),
        Line::from(Span::styled(" [y] 确认  [n] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}

/// 渲染删除 VM 确认对话框
fn render_delete_vm_confirm(frame: &mut Frame, vms: &[Vm], list_state: &VmListState) {
    let area = frame.area();
    let popup_width = 48u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" ⚠ 确认删除 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Red).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let vm_name = list_state.selected_vmx.as_ref()
        .and_then(|path| vms.iter().find(|vm| &vm.vmx_path == path))
        .map(|vm| vm.name.as_str())
        .unwrap_or("?");

    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(format!(" 永久删除 \"{}\"？此操作不可恢复！", vm_name), Style::new().fg(Color::White))),
        Line::from(Span::styled(" [y] 确认删除  [n] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}

/// 从 .vmx 文件解析共享文件夹配置
fn parse_shared_folders_from_vmx(vmx_path: &PathBuf) -> (Vec<SharedFolderEntry>, bool) {
    let mut folders = Vec::new();
    let mut enabled = false;

    if let Ok(content) = std::fs::read_to_string(vmx_path) {
        // 检测是否启用: sharedFolder.maxNum 或 isolation.tools.hgfs.disable = "FALSE"
        for line in content.lines() {
            let line = line.trim().to_lowercase();
            if line.starts_with("sharedfolder.maxnum") {
                if let Some(val) = line.split('=').nth(1) {
                    let val = val.trim().trim_matches('"');
                    if let Ok(n) = val.parse::<u32>() {
                        if n > 0 {
                            enabled = true;
                        }
                    }
                }
            }
        }

        // 解析共享文件夹条目: sharedFolder0.present, sharedFolder0.hostPath, sharedFolder0.guestName, sharedFolder0.readAccess/writeAccess
        for i in 0..16 {
            let present_key = format!("sharedfolder{}.present", i);
            let host_key = format!("sharedfolder{}.hostpath", i);
            let name_key = format!("sharedfolder{}.guestname", i);
            let write_key = format!("sharedfolder{}.writeaccess", i);

            let mut present = false;
            let mut host_path = String::new();
            let mut name = String::new();
            let mut writable = false;

            for line in content.lines() {
                let lower = line.trim().to_lowercase();
                if let Some(eq_pos) = lower.find('=') {
                    let key = lower[..eq_pos].trim();
                    let val = line.trim()[eq_pos + 1..].trim().trim_matches('"').to_string();
                    if key == present_key && val.to_lowercase() == "true" {
                        present = true;
                    } else if key == host_key {
                        host_path = val;
                    } else if key == name_key {
                        name = val;
                    } else if key == write_key && val.to_lowercase() == "true" {
                        writable = true;
                    }
                }
            }

            if present && !name.is_empty() {
                folders.push(SharedFolderEntry { name, host_path, writable });
            }
        }
    }

    (folders, enabled)
}

/// 渲染共享文件夹视图
fn render_shared_folder_view(frame: &mut Frame, ss: &SharedFolderState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    render_shared_folder_header(frame, chunks[0], ss, ascii_art);

    if ss.folders.is_empty() {
        let empty = Paragraph::new(Text::from("\n  (无共享文件夹)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let col_widths = &[
            Constraint::Length(16),  // 名称
            Constraint::Min(25),     // 宿主路径
            Constraint::Length(8),   // 权限
        ];
        let header = Row::new(vec![
            Cell::from(Span::raw(" NAME")),
            Cell::from(Span::raw(" HOST PATH")),
            Cell::from(Span::raw(" MODE")),
        ]).style(Style::new().fg(Color::White).bg(Color::DarkGray));

        let rows: Vec<Row> = ss.folders.iter().enumerate().map(|(i, f)| {
            let is_selected = i == ss.selected;
            let mode = if f.writable { "RW" } else { "RO" };
            let row = Row::new(vec![
                Cell::from(Span::raw(format!(" {}", f.name))),
                Cell::from(Span::raw(format!(" {}", f.host_path))),
                Cell::from(Span::raw(format!(" {}", mode))),
            ]);
            if is_selected {
                row.style(Style::new().bg(Color::Blue).fg(Color::White))
            } else if i % 2 == 0 {
                row.style(Style::new().bg(Color::Black))
            } else {
                row.style(Style::new().bg(Color::Rgb(30, 30, 30)))
            }
        }).collect();

        let table = Table::new(rows, col_widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
            .column_spacing(1);
        let mut table_state = TableState::default();
        table_state.select(Some(ss.selected));
        frame.render_stateful_widget(table, chunks[1], &mut table_state);
    }

    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray));
        frame.render_widget(msg_para, Rect::new(
            chunks[1].x, chunks[1].y + chunks[1].height.saturating_sub(3), chunks[1].width, 3,
        ));
    }
}

/// 渲染共享文件夹 header
fn render_shared_folder_header(frame: &mut Frame, area: Rect, ss: &SharedFolderState, ascii_art: &str) {
    let block = Block::default().style(Style::new().bg(Color::Black)).borders(Borders::BOTTOM);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter().map(|l| format!("{}\n", l)).collect();
    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan)).alignment(Alignment::Right);
    let ascii_width = 55.min(inner.width / 2);
    frame.render_widget(ascii_para, Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height));

    let enabled_label = if ss.enabled { "启用" } else { "禁用" };
    let left_col = vec![
        Line::from(vec![Span::styled("<w/s>", Style::new().fg(Color::Yellow)), Span::raw(" navigate")]),
        Line::from(vec![Span::styled("<a>", Style::new().fg(Color::Green)), Span::raw(" add")]),
        Line::from(vec![Span::styled("<d>", Style::new().fg(Color::Red)), Span::raw(" delete")]),
    ];
    let right_col = vec![
        Line::from(vec![Span::styled("<e>", Style::new().fg(Color::Yellow)), Span::raw(format!(" toggle (当前: {})", enabled_label))]),
        Line::from(vec![Span::styled("<esc>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
    ];

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 2;
    frame.render_widget(Paragraph::new(Text::from(left_col)).alignment(Alignment::Left),
        Rect::new(inner.x, inner.y, col_width, 4));
    frame.render_widget(Paragraph::new(Text::from(right_col)).alignment(Alignment::Left),
        Rect::new(inner.x + col_width, inner.y, col_width, 4));

    let info = format!("Shared Folders: {} ({})", ss.vm_name, ss.folders.len());
    frame.render_widget(
        Paragraph::new(Text::from(info)).style(Style::new().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 5, 50, 1),
    );
}

/// 渲染共享文件夹添加输入框
fn render_shared_folder_input(frame: &mut Frame, si: &SharedFolderInputState) {
    let area = frame.area();
    let popup_width = 52u16.min(area.width - 4);
    let popup_height = 7u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 添加共享文件夹 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Green).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let mut lines = Vec::new();
    for i in 0..2 {
        let marker = if i == si.field_index { "▶" } else { " " };
        let value_display = if i == si.field_index {
            format!("{}_ ", si.fields[i])
        } else {
            format!("{} ", si.fields[i])
        };
        let style = if i == si.field_index { Style::new().fg(Color::White) } else { Style::new().fg(Color::DarkGray) };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} {}: ", marker, si.field_label(i)), style),
            Span::styled(value_display, Style::new().fg(Color::Cyan)),
        ]));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " [Tab] 切换  [Enter] 确认  [Esc] 取消",
        Style::new().fg(Color::DarkGray),
    )));
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// 渲染共享文件夹删除确认
fn render_shared_folder_delete_confirm(frame: &mut Frame, ss: &SharedFolderState) {
    let area = frame.area();
    let popup_width = 44u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 确认移除 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let prompt = if let Some(f) = ss.folders.get(ss.selected) {
        format!(" 移除共享 \"{}\"？", f.name)
    } else {
        " 移除此共享？".to_string()
    };
    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(prompt, Style::new().fg(Color::White))),
        Line::from(Span::styled(" [y] 确认  [n] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}

/// 渲染客户系统凭据输入
fn render_guest_login(frame: &mut Frame, gl: &GuestLoginState) {
    let area = frame.area();
    let popup_width = 48u16.min(area.width - 4);
    let popup_height = 7u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = format!(" 登录: {} ", gl.vm_name);
    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Cyan).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let labels = ["用户名", "密码"];
    let mut lines = Vec::new();
    for i in 0..2 {
        let marker = if i == gl.field_index { "▶" } else { " " };
        let display = if i == 1 {
            // 密码用 * 遮盖
            if i == gl.field_index {
                format!("{}_ ", "*".repeat(gl.fields[i].len()))
            } else {
                format!("{} ", "*".repeat(gl.fields[i].len()))
            }
        } else if i == gl.field_index {
            format!("{}_ ", gl.fields[i])
        } else {
            format!("{} ", gl.fields[i])
        };
        let style = if i == gl.field_index { Style::new().fg(Color::White) } else { Style::new().fg(Color::DarkGray) };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} {}: ", marker, labels[i]), style),
            Span::styled(display, Style::new().fg(Color::Cyan)),
        ]));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " [Tab] 切换  [Enter] 登录  [Esc] 取消",
        Style::new().fg(Color::DarkGray),
    )));
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// 渲染客户系统文件浏览器
fn render_guest_files_view(frame: &mut Frame, gs: &GuestFilesState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    // Header
    let block = Block::default().style(Style::new().bg(Color::Black)).borders(Borders::BOTTOM);
    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);

    let ascii_lines = ascii_art.trim().lines().collect::<Vec<_>>();
    let right_text: String = ascii_lines.iter().map(|l| format!("{}\n", l)).collect();
    let ascii_para = Paragraph::new(Text::from(right_text.trim_end()))
        .style(Style::new().fg(Color::Cyan)).alignment(Alignment::Right);
    let ascii_width = 55.min(inner.width / 2);
    frame.render_widget(ascii_para, Rect::new(inner.x + inner.width - ascii_width, inner.y, ascii_width, inner.height));

    let left_col = vec![
        Line::from(vec![Span::styled("<w/s>", Style::new().fg(Color::Yellow)), Span::raw(" navigate")]),
        Line::from(vec![Span::styled("<enter>", Style::new().fg(Color::Green)), Span::raw(" open dir")]),
        Line::from(vec![Span::styled("<u>", Style::new().fg(Color::Yellow)), Span::raw(" upload")]),
    ];
    let right_col = vec![
        Line::from(vec![Span::styled("<o>", Style::new().fg(Color::Yellow)), Span::raw(" download")]),
        Line::from(vec![Span::styled("<m>", Style::new().fg(Color::Yellow)), Span::raw(" mkdir")]),
        Line::from(vec![Span::styled("<d>", Style::new().fg(Color::Red)), Span::raw(" delete")]),
        Line::from(vec![Span::styled("<esc>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
    ];

    let content_width = inner.width - ascii_width - 5;
    let col_width = content_width / 2;
    frame.render_widget(Paragraph::new(Text::from(left_col)).alignment(Alignment::Left),
        Rect::new(inner.x, inner.y, col_width, 4));
    frame.render_widget(Paragraph::new(Text::from(right_col)).alignment(Alignment::Left),
        Rect::new(inner.x + col_width, inner.y, col_width, 4));

    let info = format!("{}@{}: {}", gs.user, gs.vm_name, gs.current_dir);
    frame.render_widget(
        Paragraph::new(Text::from(info)).style(Style::new().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.y + 5, inner.width.saturating_sub(ascii_width), 1),
    );

    // 文件列表表格
    if gs.entries.is_empty() {
        let empty = Paragraph::new(Text::from("\n  (空目录)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let col_widths = &[Constraint::Min(30)];
        let header = Row::new(vec![
            Cell::from(Span::raw(" FILE / DIRECTORY")),
        ]).style(Style::new().fg(Color::White).bg(Color::DarkGray));

        let rows: Vec<Row> = gs.entries.iter().enumerate().map(|(i, entry)| {
            let is_selected = i == gs.selected;
            let row = Row::new(vec![
                Cell::from(Span::raw(format!(" {}", entry))),
            ]);
            if is_selected {
                row.style(Style::new().bg(Color::Blue).fg(Color::White))
            } else if i % 2 == 0 {
                row.style(Style::new().bg(Color::Black))
            } else {
                row.style(Style::new().bg(Color::Rgb(30, 30, 30)))
            }
        }).collect();

        let table = Table::new(rows, col_widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)))
            .column_spacing(1);
        let mut table_state = TableState::default();
        table_state.select(Some(gs.selected));
        frame.render_stateful_widget(table, chunks[1], &mut table_state);
    }

    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray));
        frame.render_widget(msg_para, Rect::new(
            chunks[1].x, chunks[1].y + chunks[1].height.saturating_sub(3), chunks[1].width, 3,
        ));
    }
}

/// 渲染客户系统文件操作输入框
fn render_guest_file_input(frame: &mut Frame, gfi: &GuestFileInputState) {
    let area = frame.area();
    let popup_width = 56u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = match &gfi.action {
        GuestFileAction::Upload => " 上传：输入宿主文件路径 ",
        GuestFileAction::Download(_) => " 下载：输入宿主保存路径 ",
        GuestFileAction::Mkdir => " 创建目录：输入目录名 ",
    };

    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Cyan).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(format!(" > {}_ ", gfi.buffer), Style::new().fg(Color::White))),
        Line::from(Span::styled(" [Enter] 确认  [Esc] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}

/// 渲染客户系统文件删除确认
fn render_guest_file_delete_confirm(frame: &mut Frame, gs: &GuestFilesState) {
    let area = frame.area();
    let popup_width = 44u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 确认删除 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Red).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let entry_name = gs.entries.get(gs.selected).map(|s| s.as_str()).unwrap_or("?");
    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(format!(" 删除 \"{}\"？", entry_name), Style::new().fg(Color::White))),
        Line::from(Span::styled(" [y] 确认  [n] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}

/// 执行虚拟机操作
fn execute_operation(
    manager: &VmManager,
    vmx_path: &PathBuf,
    op: &str,
) -> Result<String, String> {
    match op {
        "start" => manager.start_vm(vmx_path).map(|_| "虚拟机已启动".to_string())
            .map_err(|e| e.to_string()),
        "stop" => manager.stop_vm(vmx_path).map(|_| "虚拟机已停止".to_string())
            .map_err(|e| e.to_string()),
        "suspend" => manager.suspend_vm(vmx_path).map(|_| "虚拟机已挂起".to_string())
            .map_err(|e| e.to_string()),
        "resume" => manager.resume_vm(vmx_path).map(|_| "虚拟机已继续运行".to_string())
            .map_err(|e| e.to_string()),
        _ => Err("未知操作".to_string()),
    }
}

fn main() -> io::Result<()> {
    // 加载配置
    let config = Config::load();

    // 加载 ASCII 艺术字
    let ascii_art = load_ascii();

    // 创建 VM 管理器
    let manager = VmManager::new(config.vm_dir);

    // 扫描虚拟机
    if let Err(e) = manager.scan_vms() {
        eprintln!("扫描虚拟机失败: {}", e);
    }

    // 读取每个虚拟机的 IP
    manager.refresh_ip_addresses();

    // 启动状态刷新线程
    manager.start_state_refresher(config.refresh_interval);

    // 初始化终端
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &manager, &ascii_art);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal, manager: &VmManager, ascii_art: &str) -> io::Result<()> {
    let mut list_state = VmListState::new();
    let mut message: Option<String> = None;
    let mut message_timer: Option<std::time::Instant> = None;
    let mut sys = System::new_all();
    let mut cpu_usage: f32 = 0.0;
    let mut mem_usage: f32 = 0.0;
    let mut last_sys_refresh = std::time::Instant::now() - Duration::from_secs(2); // 立即触发首次刷新

    // 应用模式状态
    let mut app_mode = AppMode::List;
    let mut detail_state: Option<DetailState> = None;
    let mut edit_state: Option<EditState> = None;
    let mut snapshot_state: Option<SnapshotState> = None;
    let mut snapshot_input: Option<SnapshotInputState> = None;
    let mut snapshot_action: Option<SnapshotAction> = None;
    let mut clone_input: Option<CloneInputState> = None;
    let mut portfwd_state: Option<PortForwardState> = None;
    let mut portfwd_input: Option<PortForwardInputState> = None;
    let mut shared_state: Option<SharedFolderState> = None;
    let mut shared_input: Option<SharedFolderInputState> = None;
    let mut guest_state: Option<GuestFilesState> = None;
    let mut guest_login: Option<GuestLoginState> = None;
    let mut guest_file_input: Option<GuestFileInputState> = None;

    loop {
        // 获取当前虚拟机列表
        let vms = manager.get_vms();
        let vm_count = vms.len();

        // 确保选中项有效
        if let Some(selected) = list_state.selected_vmx.clone() {
            if !vms.iter().any(|vm| vm.vmx_path == selected) {
                list_state.with_selection(&vms, None);
            }
        }

        // 每秒刷新一次系统资源信息
        if last_sys_refresh.elapsed() >= Duration::from_secs(1) {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            cpu_usage = sys.global_cpu_usage();
            let used_mem = sys.used_memory() as f64;
            let total_mem = sys.total_memory() as f64;
            mem_usage = (used_mem / total_mem * 100.0) as f32;
            last_sys_refresh = std::time::Instant::now();
        }

        // 渲染 UI
        terminal.draw(|frame| {
            match app_mode {
                AppMode::List => {
                    ui(frame, &vms, &mut list_state, &message, vm_count, ascii_art, cpu_usage, mem_usage);
                }
                AppMode::Detail | AppMode::Confirm => {
                    if let Some(ref ds) = detail_state {
                        render_detail_view(frame, ds, &message, ascii_art);
                        if app_mode == AppMode::Confirm {
                            render_confirm_dialog(frame);
                        }
                    }
                }
                AppMode::Editing => {
                    if let Some(ref ds) = detail_state {
                        render_detail_view(frame, ds, &message, ascii_art);
                    }
                    if let Some(ref es) = edit_state {
                        render_edit_overlay(frame, es);
                    }
                }
                AppMode::Snapshot => {
                    if let Some(ref ss) = snapshot_state {
                        render_snapshot_view(frame, ss, &message, ascii_art);
                    }
                }
                AppMode::SnapshotInput => {
                    if let Some(ref ss) = snapshot_state {
                        render_snapshot_view(frame, ss, &message, ascii_art);
                    }
                    if let Some(ref sis) = snapshot_input {
                        render_snapshot_input(frame, sis);
                    }
                }
                AppMode::SnapshotConfirm => {
                    if let Some(ref ss) = snapshot_state {
                        render_snapshot_view(frame, ss, &message, ascii_art);
                    }
                    if let Some(ref action) = snapshot_action {
                        render_snapshot_confirm(frame, action);
                    }
                }
                AppMode::CloneInput => {
                    ui(frame, &vms, &mut list_state, &message, vm_count, ascii_art, cpu_usage, mem_usage);
                    if let Some(ref ci) = clone_input {
                        render_clone_input(frame, ci);
                    }
                }
                AppMode::PortForward => {
                    if let Some(ref pfs) = portfwd_state {
                        render_portfwd_view(frame, pfs, &message, ascii_art);
                    }
                }
                AppMode::PortForwardInput => {
                    if let Some(ref pfs) = portfwd_state {
                        render_portfwd_view(frame, pfs, &message, ascii_art);
                    }
                    if let Some(ref pfi) = portfwd_input {
                        render_portfwd_input(frame, pfi);
                    }
                }
                AppMode::PortForwardConfirm => {
                    if let Some(ref pfs) = portfwd_state {
                        render_portfwd_view(frame, pfs, &message, ascii_art);
                        render_portfwd_delete_confirm(frame, pfs);
                    }
                }
                AppMode::DeleteConfirm => {
                    ui(frame, &vms, &mut list_state, &message, vm_count, ascii_art, cpu_usage, mem_usage);
                    render_delete_vm_confirm(frame, &vms, &list_state);
                }
                AppMode::SharedFolder => {
                    if let Some(ref ss) = shared_state {
                        render_shared_folder_view(frame, ss, &message, ascii_art);
                    }
                }
                AppMode::SharedFolderInput => {
                    if let Some(ref ss) = shared_state {
                        render_shared_folder_view(frame, ss, &message, ascii_art);
                    }
                    if let Some(ref si) = shared_input {
                        render_shared_folder_input(frame, si);
                    }
                }
                AppMode::SharedFolderConfirm => {
                    if let Some(ref ss) = shared_state {
                        render_shared_folder_view(frame, ss, &message, ascii_art);
                        render_shared_folder_delete_confirm(frame, ss);
                    }
                }
                AppMode::GuestLogin => {
                    ui(frame, &vms, &mut list_state, &message, vm_count, ascii_art, cpu_usage, mem_usage);
                    if let Some(ref gl) = guest_login {
                        render_guest_login(frame, gl);
                    }
                }
                AppMode::GuestFiles => {
                    if let Some(ref gs) = guest_state {
                        render_guest_files_view(frame, gs, &message, ascii_art);
                    }
                }
                AppMode::GuestFileInput => {
                    if let Some(ref gs) = guest_state {
                        render_guest_files_view(frame, gs, &message, ascii_art);
                    }
                    if let Some(ref gfi) = guest_file_input {
                        render_guest_file_input(frame, gfi);
                    }
                }
                AppMode::GuestFileConfirm => {
                    if let Some(ref gs) = guest_state {
                        render_guest_files_view(frame, gs, &message, ascii_art);
                        render_guest_file_delete_confirm(frame, gs);
                    }
                }
            }
        })?;

        // 事件处理
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app_mode {
                    AppMode::List => {
                        match key.code {
                            KeyCode::Char('s') | KeyCode::Down => {
                                let new_index = list_state.state.selected()
                                    .map(|i| (i + 1).min(vms.len().saturating_sub(1)))
                                    .unwrap_or(0);
                                list_state.with_selection(&vms, Some(new_index));
                                message = None;
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                let new_index = list_state.state.selected()
                                    .map(|i| i.saturating_sub(1))
                                    .unwrap_or(0);
                                list_state.with_selection(&vms, Some(new_index));
                                message = None;
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(());
                            }
                            KeyCode::Char('i') => {
                                // 进入配置详情视图
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    match VmxFile::parse(&vmx_path) {
                                        Ok(vmx_file) => {
                                            let config = vmx_file.hardware_config();
                                            let vm_name = vms.iter()
                                                .find(|vm| vm.vmx_path == vmx_path)
                                                .map(|vm| vm.name.clone())
                                                .unwrap_or_default();
                                            detail_state = Some(DetailState {
                                                vmx_file,
                                                config,
                                                selected_row: 0,
                                                dirty: false,
                                                vm_name,
                                                vmx_path,
                                            });
                                            app_mode = AppMode::Detail;
                                            message = None;
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 无法读取配置: {}", e));
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('n') => {
                                // 进入快照视图
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    let vm_name = vms.iter()
                                        .find(|vm| vm.vmx_path == vmx_path)
                                        .map(|vm| vm.name.clone())
                                        .unwrap_or_default();
                                    match Vmrun::list_snapshots(&vmx_path) {
                                        Ok(snapshots) => {
                                            snapshot_state = Some(SnapshotState {
                                                snapshots,
                                                selected: 0,
                                                vm_name,
                                                vmx_path,
                                            });
                                            app_mode = AppMode::Snapshot;
                                            message = None;
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 获取快照列表失败: {}", e));
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                // 克隆虚拟机
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    let vm_name = vms.iter()
                                        .find(|vm| vm.vmx_path == vmx_path)
                                        .map(|vm| vm.name.clone())
                                        .unwrap_or_default();
                                    clone_input = Some(CloneInputState {
                                        buffer: format!("{}-clone", vm_name),
                                        source_vmx: vmx_path,
                                        source_name: vm_name,
                                        clone_type: CloneType::Full,
                                    });
                                    app_mode = AppMode::CloneInput;
                                    message = None;
                                }
                            }
                            KeyCode::Char('f') => {
                                // 进入端口转发管理
                                match Vmrun::list_nat_networks() {
                                    Ok(nets) => {
                                        if let Some(network) = nets.first() {
                                            let rules = Vmrun::list_port_forwardings(network).unwrap_or_default();
                                            portfwd_state = Some(PortForwardState {
                                                network: network.clone(),
                                                rules,
                                                selected: 0,
                                            });
                                            app_mode = AppMode::PortForward;
                                            message = None;
                                        } else {
                                            message = Some("✗ 未找到 NAT 网络".to_string());
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                    Err(e) => {
                                        message = Some(format!("✗ 获取网络失败: {}", e));
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                            }
                            KeyCode::Char('D') => {
                                // 删除虚拟机（需确认）
                                if let Some(ref vmx_path) = list_state.selected_vmx {
                                    // 检查 VM 是否已停止
                                    let vm_state = vms.iter()
                                        .find(|vm| &vm.vmx_path == vmx_path)
                                        .map(|vm| &vm.state);
                                    if vm_state == Some(&VmState::Stopped) {
                                        app_mode = AppMode::DeleteConfirm;
                                        message = None;
                                    } else {
                                        message = Some("✗ 虚拟机必须停止后才能删除".to_string());
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                            }
                            KeyCode::Char('h') => {
                                // 进入共享文件夹管理
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    let vm_name = vms.iter()
                                        .find(|vm| vm.vmx_path == vmx_path)
                                        .map(|vm| vm.name.clone())
                                        .unwrap_or_default();
                                    // 从 .vmx 文件解析共享文件夹配置
                                    let (folders, enabled) = parse_shared_folders_from_vmx(&vmx_path);
                                    shared_state = Some(SharedFolderState {
                                        vm_name,
                                        vmx_path,
                                        folders,
                                        selected: 0,
                                        enabled,
                                    });
                                    app_mode = AppMode::SharedFolder;
                                    message = None;
                                }
                            }
                            KeyCode::Char('g') => {
                                // 进入客户系统文件管理（需要 VM 运行中）
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    let vm_state = vms.iter()
                                        .find(|vm| vm.vmx_path == vmx_path)
                                        .map(|vm| &vm.state);
                                    if vm_state == Some(&VmState::Running) {
                                        let vm_name = vms.iter()
                                            .find(|vm| vm.vmx_path == vmx_path)
                                            .map(|vm| vm.name.clone())
                                            .unwrap_or_default();
                                        guest_login = Some(GuestLoginState {
                                            field_index: 0,
                                            fields: ["root".to_string(), String::new()],
                                            vmx_path,
                                            vm_name,
                                        });
                                        app_mode = AppMode::GuestLogin;
                                        message = None;
                                    } else {
                                        message = Some("✗ 虚拟机必须运行中才能操作客户文件".to_string());
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Char('x') | KeyCode::Char('p') | KeyCode::Char('r') => {
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    let op = match key.code {
                                        KeyCode::Enter => "start",
                                        KeyCode::Char('x') => "stop",
                                        KeyCode::Char('p') => "suspend",
                                        KeyCode::Char('r') => "resume",
                                        _ => continue,
                                    };

                                    match execute_operation(manager, &vmx_path, op) {
                                        Ok(msg) => {
                                            message = Some(format!("✓ {}", msg));
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ {}", e));
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::Detail => {
                        let row_count = get_detail_rows(detail_state.as_ref().unwrap()).len();
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('i') => {
                                // 返回列表
                                app_mode = AppMode::List;
                                detail_state = None;
                                message = None;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                if let Some(ref mut ds) = detail_state {
                                    if ds.selected_row < row_count.saturating_sub(1) {
                                        ds.selected_row += 1;
                                    }
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                if let Some(ref mut ds) = detail_state {
                                    if ds.selected_row > 0 {
                                        ds.selected_row -= 1;
                                    }
                                }
                            }
                            KeyCode::Char('e') => {
                                // 进入编辑模式
                                if let Some(ref ds) = detail_state {
                                    let rows = get_detail_rows(ds);
                                    if let Some(row) = rows.get(ds.selected_row) {
                                        if row.editable {
                                            // 检查 VM 是否已停止
                                            let vm_state = vms.iter()
                                                .find(|vm| vm.vmx_path == ds.vmx_path)
                                                .map(|vm| &vm.state);
                                            if vm_state == Some(&VmState::Stopped) {
                                                // 磁盘编辑时初始值为当前 GB 数
                                                let init_buffer = match &row.edit_field {
                                                    Some(EditField::DiskSize(idx)) => {
                                                        ds.config.disks.get(*idx)
                                                            .and_then(|d| d.size_gb)
                                                            .map(|gb| gb.to_string())
                                                            .unwrap_or_default()
                                                    }
                                                    _ => row.value.clone(),
                                                };
                                                edit_state = Some(EditState {
                                                    field: row.edit_field.clone().unwrap(),
                                                    buffer: init_buffer.clone(),
                                                    original_value: init_buffer,
                                                });
                                                app_mode = AppMode::Editing;
                                                message = None;
                                            } else {
                                                message = Some("✗ 虚拟机必须停止后才能编辑配置".to_string());
                                                message_timer = Some(std::time::Instant::now());
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('W') => {
                                // 保存修改
                                if let Some(ref ds) = detail_state {
                                    if ds.dirty {
                                        // 再次检查 VM 状态
                                        let vm_state = vms.iter()
                                            .find(|vm| vm.vmx_path == ds.vmx_path)
                                            .map(|vm| &vm.state);
                                        if vm_state == Some(&VmState::Stopped) {
                                            app_mode = AppMode::Confirm;
                                        } else {
                                            message = Some("✗ 虚拟机必须停止后才能保存配置".to_string());
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('u') => {
                                // 撤销所有未保存修改
                                if let Some(ref mut ds) = detail_state {
                                    if ds.dirty {
                                        match VmxFile::parse(&ds.vmx_path) {
                                            Ok(vmx_file) => {
                                                ds.config = vmx_file.hardware_config();
                                                ds.vmx_file = vmx_file;
                                                ds.dirty = false;
                                                message = Some("✓ 已撤销所有修改".to_string());
                                                message_timer = Some(std::time::Instant::now());
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 重新加载失败: {}", e));
                                                message_timer = Some(std::time::Instant::now());
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::Editing => {
                        match key.code {
                            KeyCode::Esc => {
                                // 取消编辑
                                edit_state = None;
                                app_mode = AppMode::Detail;
                            }
                            KeyCode::Enter => {
                                // 确认编辑
                                if let (Some(ref es), Some(ref mut ds)) = (&edit_state, &mut detail_state) {
                                    let value: u64 = es.buffer.parse().unwrap_or(0);
                                    let valid = match &es.field {
                                        EditField::CpuCount => value >= 1 && value <= 128,
                                        EditField::MemoryMb => value >= 128 && value <= 1048576,
                                        EditField::DiskSize(idx) => {
                                            // 磁盘只能扩容，不能缩小
                                            if let Some(disk) = ds.config.disks.get(*idx) {
                                                let current = disk.size_gb.unwrap_or(0);
                                                value > current && value <= 65536
                                            } else {
                                                false
                                            }
                                        }
                                    };

                                    if valid {
                                        match &es.field {
                                            EditField::CpuCount => {
                                                ds.vmx_file.set("numvcpus", &es.buffer);
                                                ds.config = ds.vmx_file.hardware_config();
                                                ds.dirty = true;
                                                message = None;
                                            }
                                            EditField::MemoryMb => {
                                                ds.vmx_file.set("memsize", &es.buffer);
                                                ds.config = ds.vmx_file.hardware_config();
                                                ds.dirty = true;
                                                message = None;
                                            }
                                            EditField::DiskSize(idx) => {
                                                // 磁盘扩容：直接调用 vdiskmanager
                                                if let Some(disk) = ds.config.disks.get(*idx) {
                                                    match vmx::expand_disk(&disk.full_path, value) {
                                                        Ok(msg) => {
                                                            message = Some(format!("✓ {}", msg));
                                                            // 重新加载配置以反映新大小
                                                            ds.config = ds.vmx_file.hardware_config();
                                                        }
                                                        Err(e) => {
                                                            message = Some(format!("✗ {}", e));
                                                        }
                                                    }
                                                    message_timer = Some(std::time::Instant::now());
                                                }
                                            }
                                        }
                                    } else {
                                        let hint = match &es.field {
                                            EditField::CpuCount => "CPU 数量范围: 1-128".to_string(),
                                            EditField::MemoryMb => "内存范围: 128-1048576 MB".to_string(),
                                            EditField::DiskSize(idx) => {
                                                let current = ds.config.disks.get(*idx)
                                                    .and_then(|d| d.size_gb)
                                                    .unwrap_or(0);
                                                format!("磁盘只能扩容，须大于当前 {}GB", current)
                                            }
                                        };
                                        message = Some(format!("✗ 无效值。{}", hint));
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                edit_state = None;
                                app_mode = AppMode::Detail;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut es) = edit_state {
                                    es.buffer.pop();
                                }
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                if let Some(ref mut es) = edit_state {
                                    if es.buffer.len() < 7 {
                                        es.buffer.push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::Confirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                // 执行保存
                                if let Some(ref ds) = detail_state {
                                    match ds.vmx_file.backup_and_write() {
                                        Ok(()) => {
                                            message = Some("✓ 配置已保存".to_string());
                                            message_timer = Some(std::time::Instant::now());
                                            if let Some(ref mut ds) = detail_state {
                                                ds.dirty = false;
                                            }
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 保存失败: {}", e));
                                            message_timer = Some(std::time::Instant::now());
                                        }
                                    }
                                }
                                app_mode = AppMode::Detail;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app_mode = AppMode::Detail;
                            }
                            _ => {}
                        }
                    }
                    AppMode::Snapshot => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('n') => {
                                // 返回列表
                                app_mode = AppMode::List;
                                snapshot_state = None;
                                message = None;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                if let Some(ref mut ss) = snapshot_state {
                                    if !ss.snapshots.is_empty() && ss.selected < ss.snapshots.len().saturating_sub(1) {
                                        ss.selected += 1;
                                    }
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                if let Some(ref mut ss) = snapshot_state {
                                    if ss.selected > 0 {
                                        ss.selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                // 创建快照：弹出输入框
                                snapshot_input = Some(SnapshotInputState {
                                    buffer: String::new(),
                                });
                                app_mode = AppMode::SnapshotInput;
                                message = None;
                            }
                            KeyCode::Char('d') => {
                                // 删除快照：需确认
                                if let Some(ref ss) = snapshot_state {
                                    if let Some(name) = ss.snapshots.get(ss.selected) {
                                        snapshot_action = Some(SnapshotAction::Delete(name.clone()));
                                        app_mode = AppMode::SnapshotConfirm;
                                    }
                                }
                            }
                            KeyCode::Char('r') => {
                                // 恢复快照：需确认
                                if let Some(ref ss) = snapshot_state {
                                    if let Some(name) = ss.snapshots.get(ss.selected) {
                                        snapshot_action = Some(SnapshotAction::Revert(name.clone()));
                                        app_mode = AppMode::SnapshotConfirm;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::SnapshotInput => {
                        match key.code {
                            KeyCode::Esc => {
                                snapshot_input = None;
                                app_mode = AppMode::Snapshot;
                            }
                            KeyCode::Enter => {
                                // 创建快照
                                if let (Some(ref sis), Some(ref mut ss)) = (&snapshot_input, &mut snapshot_state) {
                                    let name = sis.buffer.trim().to_string();
                                    if !name.is_empty() {
                                        match Vmrun::create_snapshot(&ss.vmx_path, &name) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 快照 \"{}\" 已创建", name));
                                                // 刷新快照列表
                                                if let Ok(snapshots) = Vmrun::list_snapshots(&ss.vmx_path) {
                                                    ss.snapshots = snapshots;
                                                }
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 创建快照失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                snapshot_input = None;
                                app_mode = AppMode::Snapshot;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut sis) = snapshot_input {
                                    sis.buffer.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut sis) = snapshot_input {
                                    if sis.buffer.len() < 64 {
                                        sis.buffer.push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::SnapshotConfirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                if let (Some(ref action), Some(ref mut ss)) = (&snapshot_action, &mut snapshot_state) {
                                    match action {
                                        SnapshotAction::Delete(name) => {
                                            match Vmrun::delete_snapshot(&ss.vmx_path, name) {
                                                Ok(()) => {
                                                    message = Some(format!("✓ 快照 \"{}\" 已删除", name));
                                                    // 刷新快照列表
                                                    if let Ok(snapshots) = Vmrun::list_snapshots(&ss.vmx_path) {
                                                        ss.snapshots = snapshots;
                                                        if ss.selected >= ss.snapshots.len() && ss.selected > 0 {
                                                            ss.selected -= 1;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    message = Some(format!("✗ 删除快照失败: {}", e));
                                                }
                                            }
                                        }
                                        SnapshotAction::Revert(name) => {
                                            match Vmrun::revert_to_snapshot(&ss.vmx_path, name) {
                                                Ok(()) => {
                                                    message = Some(format!("✓ 已恢复到快照 \"{}\"", name));
                                                }
                                                Err(e) => {
                                                    message = Some(format!("✗ 恢复快照失败: {}", e));
                                                }
                                            }
                                        }
                                    }
                                    message_timer = Some(std::time::Instant::now());
                                }
                                snapshot_action = None;
                                app_mode = AppMode::Snapshot;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                snapshot_action = None;
                                app_mode = AppMode::Snapshot;
                            }
                            _ => {}
                        }
                    }
                    AppMode::CloneInput => {
                        match key.code {
                            KeyCode::Esc => {
                                clone_input = None;
                                app_mode = AppMode::List;
                            }
                            KeyCode::Tab => {
                                // Tab 切换克隆类型
                                if let Some(ref mut ci) = clone_input {
                                    ci.clone_type = match ci.clone_type {
                                        CloneType::Full => CloneType::Linked,
                                        CloneType::Linked => CloneType::Full,
                                    };
                                }
                            }
                            KeyCode::Enter => {
                                // 执行克隆
                                if let Some(ref ci) = clone_input {
                                    let name = ci.buffer.trim().to_string();
                                    if !name.is_empty() {
                                        // 构建目标路径: vm_dir/<name>.vmwarevm/<name>.vmx
                                        let source_dir = ci.source_vmx.parent()
                                            .and_then(|p| p.parent())
                                            .unwrap_or(std::path::Path::new("."));
                                        let dest_dir = source_dir.join(format!("{}.vmwarevm", name));
                                        let dest_vmx = dest_dir.join(format!("{}.vmx", name));

                                        // 克隆可能耗时较长
                                        match Vmrun::clone_vm(
                                            &ci.source_vmx,
                                            &dest_vmx,
                                            ci.clone_type.as_str(),
                                            &name,
                                        ) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 已克隆为 \"{}\" ({})", name, ci.clone_type.display_name()));
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 克隆失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                clone_input = None;
                                app_mode = AppMode::List;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut ci) = clone_input {
                                    ci.buffer.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut ci) = clone_input {
                                    if ci.buffer.len() < 64 {
                                        ci.buffer.push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::PortForward => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('f') => {
                                app_mode = AppMode::List;
                                portfwd_state = None;
                                message = None;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                if let Some(ref mut pfs) = portfwd_state {
                                    if !pfs.rules.is_empty() && pfs.selected < pfs.rules.len().saturating_sub(1) {
                                        pfs.selected += 1;
                                    }
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                if let Some(ref mut pfs) = portfwd_state {
                                    if pfs.selected > 0 {
                                        pfs.selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Char('a') => {
                                // 添加端口转发
                                portfwd_input = Some(PortForwardInputState::new());
                                app_mode = AppMode::PortForwardInput;
                                message = None;
                            }
                            KeyCode::Char('d') => {
                                // 删除端口转发
                                if let Some(ref pfs) = portfwd_state {
                                    if !pfs.rules.is_empty() {
                                        app_mode = AppMode::PortForwardConfirm;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::PortForwardInput => {
                        match key.code {
                            KeyCode::Esc => {
                                portfwd_input = None;
                                app_mode = AppMode::PortForward;
                            }
                            KeyCode::Tab => {
                                if let Some(ref mut pfi) = portfwd_input {
                                    pfi.field_index = (pfi.field_index + 1) % 5;
                                }
                            }
                            KeyCode::BackTab => {
                                if let Some(ref mut pfi) = portfwd_input {
                                    pfi.field_index = if pfi.field_index == 0 { 4 } else { pfi.field_index - 1 };
                                }
                            }
                            KeyCode::Enter => {
                                if let (Some(ref pfi), Some(ref mut pfs)) = (&portfwd_input, &mut portfwd_state) {
                                    let protocol = pfi.fields[0].trim();
                                    let host_port: u16 = pfi.fields[1].trim().parse().unwrap_or(0);
                                    let guest_ip = pfi.fields[2].trim();
                                    let guest_port: u16 = pfi.fields[3].trim().parse().unwrap_or(0);
                                    let description = pfi.fields[4].trim();

                                    if (protocol == "tcp" || protocol == "udp")
                                        && host_port > 0
                                        && !guest_ip.is_empty()
                                        && guest_port > 0
                                    {
                                        match Vmrun::set_port_forwarding(
                                            &pfs.network, protocol, host_port,
                                            guest_ip, guest_port, description,
                                        ) {
                                            Ok(()) => {
                                                message = Some(format!(
                                                    "✓ 已添加 {} :{} → {}:{}",
                                                    protocol, host_port, guest_ip, guest_port
                                                ));
                                                if let Ok(rules) = Vmrun::list_port_forwardings(&pfs.network) {
                                                    pfs.rules = rules;
                                                }
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 添加失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    } else {
                                        message = Some("✗ 请填写完整（协议 tcp/udp，端口 > 0）".to_string());
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                portfwd_input = None;
                                app_mode = AppMode::PortForward;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut pfi) = portfwd_input {
                                    pfi.current_field_mut().pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut pfi) = portfwd_input {
                                    if pfi.current_field().len() < 40 {
                                        pfi.current_field_mut().push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::PortForwardConfirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                if let Some(ref mut pfs) = portfwd_state {
                                    if let Some(rule) = pfs.rules.get(pfs.selected).cloned() {
                                        match Vmrun::delete_port_forwarding(
                                            &pfs.network, &rule.protocol, rule.host_port,
                                        ) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 已删除 {} :{}", rule.protocol, rule.host_port));
                                                if let Ok(rules) = Vmrun::list_port_forwardings(&pfs.network) {
                                                    pfs.rules = rules;
                                                    if pfs.selected >= pfs.rules.len() && pfs.selected > 0 {
                                                        pfs.selected -= 1;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 删除失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                app_mode = AppMode::PortForward;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app_mode = AppMode::PortForward;
                            }
                            _ => {}
                        }
                    }
                    AppMode::DeleteConfirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                if let Some(vmx_path) = list_state.selected_vmx.clone() {
                                    match Vmrun::delete_vm(&vmx_path) {
                                        Ok(()) => {
                                            let vm_name = vms.iter()
                                                .find(|vm| vm.vmx_path == vmx_path)
                                                .map(|vm| vm.name.clone())
                                                .unwrap_or_default();
                                            message = Some(format!("✓ 虚拟机 \"{}\" 已删除", vm_name));
                                            list_state.with_selection(&[], None);
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 删除失败: {}", e));
                                        }
                                    }
                                    message_timer = Some(std::time::Instant::now());
                                }
                                app_mode = AppMode::List;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app_mode = AppMode::List;
                            }
                            _ => {}
                        }
                    }
                    AppMode::SharedFolder => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('h') => {
                                app_mode = AppMode::List;
                                shared_state = None;
                                message = None;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                if let Some(ref mut ss) = shared_state {
                                    if !ss.folders.is_empty() && ss.selected < ss.folders.len().saturating_sub(1) {
                                        ss.selected += 1;
                                    }
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                if let Some(ref mut ss) = shared_state {
                                    if ss.selected > 0 {
                                        ss.selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Char('a') => {
                                // 添加共享文件夹
                                shared_input = Some(SharedFolderInputState::new());
                                app_mode = AppMode::SharedFolderInput;
                                message = None;
                            }
                            KeyCode::Char('d') => {
                                // 删除共享文件夹
                                if let Some(ref ss) = shared_state {
                                    if !ss.folders.is_empty() {
                                        app_mode = AppMode::SharedFolderConfirm;
                                    }
                                }
                            }
                            KeyCode::Char('e') => {
                                // 启用/禁用共享文件夹功能
                                if let Some(ref mut ss) = shared_state {
                                    let result = if ss.enabled {
                                        Vmrun::disable_shared_folders(&ss.vmx_path)
                                    } else {
                                        Vmrun::enable_shared_folders(&ss.vmx_path)
                                    };
                                    match result {
                                        Ok(()) => {
                                            ss.enabled = !ss.enabled;
                                            let status = if ss.enabled { "已启用" } else { "已禁用" };
                                            message = Some(format!("✓ 共享文件夹{}", status));
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 操作失败: {}", e));
                                        }
                                    }
                                    message_timer = Some(std::time::Instant::now());
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::SharedFolderInput => {
                        match key.code {
                            KeyCode::Esc => {
                                shared_input = None;
                                app_mode = AppMode::SharedFolder;
                            }
                            KeyCode::Tab => {
                                if let Some(ref mut si) = shared_input {
                                    si.field_index = (si.field_index + 1) % 2;
                                }
                            }
                            KeyCode::Enter => {
                                if let (Some(ref si), Some(ref mut ss)) = (&shared_input, &mut shared_state) {
                                    let name = si.fields[0].trim().to_string();
                                    let host_path = si.fields[1].trim().to_string();
                                    if !name.is_empty() && !host_path.is_empty() {
                                        match Vmrun::add_shared_folder(&ss.vmx_path, &name, &host_path) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 已添加共享 \"{}\"", name));
                                                ss.folders.push(SharedFolderEntry {
                                                    name,
                                                    host_path,
                                                    writable: true,
                                                });
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 添加失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    } else {
                                        message = Some("✗ 名称和路径不能为空".to_string());
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                shared_input = None;
                                app_mode = AppMode::SharedFolder;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut si) = shared_input {
                                    si.fields[si.field_index].pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut si) = shared_input {
                                    if si.fields[si.field_index].len() < 128 {
                                        si.fields[si.field_index].push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::SharedFolderConfirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                if let Some(ref mut ss) = shared_state {
                                    if let Some(folder) = ss.folders.get(ss.selected).cloned() {
                                        match Vmrun::remove_shared_folder(&ss.vmx_path, &folder.name) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 已移除共享 \"{}\"", folder.name));
                                                ss.folders.remove(ss.selected);
                                                if ss.selected >= ss.folders.len() && ss.selected > 0 {
                                                    ss.selected -= 1;
                                                }
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 移除失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                app_mode = AppMode::SharedFolder;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app_mode = AppMode::SharedFolder;
                            }
                            _ => {}
                        }
                    }
                    AppMode::GuestLogin => {
                        match key.code {
                            KeyCode::Esc => {
                                guest_login = None;
                                app_mode = AppMode::List;
                            }
                            KeyCode::Tab => {
                                if let Some(ref mut gl) = guest_login {
                                    gl.field_index = (gl.field_index + 1) % 2;
                                }
                            }
                            KeyCode::Enter => {
                                // 尝试登录并列出根目录
                                if let Some(ref gl) = guest_login {
                                    let user = gl.fields[0].trim().to_string();
                                    let pass = gl.fields[1].clone();
                                    let start_dir = "/".to_string();
                                    match Vmrun::list_directory_in_guest(&gl.vmx_path, &user, &pass, &start_dir) {
                                        Ok(entries) => {
                                            guest_state = Some(GuestFilesState {
                                                vmx_path: gl.vmx_path.clone(),
                                                vm_name: gl.vm_name.clone(),
                                                user,
                                                pass,
                                                current_dir: start_dir,
                                                entries,
                                                selected: 0,
                                            });
                                            app_mode = AppMode::GuestFiles;
                                            message = None;
                                        }
                                        Err(e) => {
                                            message = Some(format!("✗ 登录失败: {}", e));
                                            message_timer = Some(std::time::Instant::now());
                                            app_mode = AppMode::List;
                                        }
                                    }
                                }
                                guest_login = None;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut gl) = guest_login {
                                    gl.fields[gl.field_index].pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut gl) = guest_login {
                                    if gl.fields[gl.field_index].len() < 64 {
                                        gl.fields[gl.field_index].push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::GuestFiles => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app_mode = AppMode::List;
                                guest_state = None;
                                message = None;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                if let Some(ref mut gs) = guest_state {
                                    if !gs.entries.is_empty() && gs.selected < gs.entries.len().saturating_sub(1) {
                                        gs.selected += 1;
                                    }
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                if let Some(ref mut gs) = guest_state {
                                    if gs.selected > 0 {
                                        gs.selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // 进入目录
                                if let Some(ref mut gs) = guest_state {
                                    if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                                        let new_dir = if entry == ".." {
                                            // 返回上级
                                            let parent = std::path::Path::new(&gs.current_dir)
                                                .parent()
                                                .map(|p| p.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "/".to_string());
                                            if parent.is_empty() { "/".to_string() } else { parent }
                                        } else {
                                            let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                                            format!("{}{}{}", gs.current_dir, sep, entry)
                                        };
                                        match Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &new_dir) {
                                            Ok(entries) => {
                                                gs.current_dir = new_dir;
                                                gs.entries = entries;
                                                gs.selected = 0;
                                            }
                                            Err(_) => {
                                                // 可能是文件不是目录，忽略
                                                message = Some("✗ 无法进入（可能是文件）".to_string());
                                                message_timer = Some(std::time::Instant::now());
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('u') => {
                                // 上传文件
                                guest_file_input = Some(GuestFileInputState {
                                    action: GuestFileAction::Upload,
                                    buffer: String::new(),
                                });
                                app_mode = AppMode::GuestFileInput;
                            }
                            KeyCode::Char('o') => {
                                // 下载文件
                                if let Some(ref gs) = guest_state {
                                    if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                                        if entry != ".." {
                                            let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                                            let guest_path = format!("{}{}{}", gs.current_dir, sep, entry);
                                            guest_file_input = Some(GuestFileInputState {
                                                action: GuestFileAction::Download(guest_path),
                                                buffer: String::new(),
                                            });
                                            app_mode = AppMode::GuestFileInput;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('m') => {
                                // 创建目录
                                guest_file_input = Some(GuestFileInputState {
                                    action: GuestFileAction::Mkdir,
                                    buffer: String::new(),
                                });
                                app_mode = AppMode::GuestFileInput;
                            }
                            KeyCode::Char('d') => {
                                // 删除文件
                                if let Some(ref gs) = guest_state {
                                    if let Some(entry) = gs.entries.get(gs.selected) {
                                        if entry != ".." {
                                            app_mode = AppMode::GuestFileConfirm;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::GuestFileInput => {
                        match key.code {
                            KeyCode::Esc => {
                                guest_file_input = None;
                                app_mode = AppMode::GuestFiles;
                            }
                            KeyCode::Enter => {
                                if let (Some(ref gfi), Some(ref mut gs)) = (&guest_file_input, &mut guest_state) {
                                    let path = gfi.buffer.trim().to_string();
                                    if !path.is_empty() {
                                        let result = match &gfi.action {
                                            GuestFileAction::Upload => {
                                                let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                                                let filename = std::path::Path::new(&path)
                                                    .file_name()
                                                    .map(|n| n.to_string_lossy().to_string())
                                                    .unwrap_or_else(|| "file".to_string());
                                                let guest_dest = format!("{}{}{}", gs.current_dir, sep, filename);
                                                Vmrun::copy_file_to_guest(&gs.vmx_path, &gs.user, &gs.pass, &path, &guest_dest)
                                                    .map(|_| format!("✓ 已上传到 {}", guest_dest))
                                            }
                                            GuestFileAction::Download(guest_path) => {
                                                Vmrun::copy_file_from_guest(&gs.vmx_path, &gs.user, &gs.pass, guest_path, &path)
                                                    .map(|_| format!("✓ 已下载到 {}", path))
                                            }
                                            GuestFileAction::Mkdir => {
                                                let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                                                let new_dir = format!("{}{}{}", gs.current_dir, sep, path);
                                                Vmrun::create_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &new_dir)
                                                    .map(|_| format!("✓ 已创建目录 {}", new_dir))
                                            }
                                        };
                                        match result {
                                            Ok(msg) => message = Some(msg),
                                            Err(e) => message = Some(format!("✗ {}", e)),
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                        // 刷新目录
                                        if let Ok(entries) = Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &gs.current_dir) {
                                            gs.entries = entries;
                                        }
                                    }
                                }
                                guest_file_input = None;
                                app_mode = AppMode::GuestFiles;
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut gfi) = guest_file_input {
                                    gfi.buffer.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut gfi) = guest_file_input {
                                    if gfi.buffer.len() < 256 {
                                        gfi.buffer.push(c);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppMode::GuestFileConfirm => {
                        match key.code {
                            KeyCode::Char('y') => {
                                if let Some(ref mut gs) = guest_state {
                                    if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                                        let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                                        let full_path = format!("{}{}{}", gs.current_dir, sep, entry);
                                        match Vmrun::delete_file_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &full_path) {
                                            Ok(()) => {
                                                message = Some(format!("✓ 已删除 {}", entry));
                                                if let Ok(entries) = Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &gs.current_dir) {
                                                    gs.entries = entries;
                                                    if gs.selected >= gs.entries.len() && gs.selected > 0 {
                                                        gs.selected -= 1;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                message = Some(format!("✗ 删除失败: {}", e));
                                            }
                                        }
                                        message_timer = Some(std::time::Instant::now());
                                    }
                                }
                                app_mode = AppMode::GuestFiles;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app_mode = AppMode::GuestFiles;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 清除消息（3 秒后）
        if let Some(timer) = message_timer {
            if timer.elapsed() > Duration::from_secs(3) {
                message = None;
                message_timer = None;
            }
        }
    }
}