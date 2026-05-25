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
use vmrun::set_vmrun_path;
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

    // 左侧：操作提示（K9S 风格，两列竖排）
    let left_col = vec![
        Line::from(vec![
            Span::styled("<w/s>", Style::new().fg(Color::Yellow)),
            Span::raw(" navigate"),
        ]),
        Line::from(vec![
            Span::styled("<enter>", Style::new().fg(Color::Green)),
            Span::raw(" start"),
        ]),
        Line::from(vec![
            Span::styled("<x>", Style::new().fg(Color::Yellow)),
            Span::raw(" stop"),
        ]),
    ];

    let right_col = vec![
        Line::from(vec![
            Span::styled("<p>", Style::new().fg(Color::Yellow)),
            Span::raw(" pause"),
        ]),
        Line::from(vec![
            Span::styled("<r>", Style::new().fg(Color::Yellow)),
            Span::raw(" resume"),
        ]),
        Line::from(vec![
            Span::styled("<i>", Style::new().fg(Color::Yellow)),
            Span::raw(" info"),
        ]),
        Line::from(vec![
            Span::styled("<q>", Style::new().fg(Color::Yellow)),
            Span::raw(" quit"),
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

    // 应用模式状态
    let mut app_mode = AppMode::List;
    let mut detail_state: Option<DetailState> = None;
    let mut edit_state: Option<EditState> = None;

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

        // 刷新系统资源信息
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let cpu_usage = sys.global_cpu_usage();
        let used_mem = sys.used_memory() as f64;
        let total_mem = sys.total_memory() as f64;
        let mem_usage = (used_mem / total_mem * 100.0) as f32;

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
            }
        })?;

        // 事件处理
        if event::poll(Duration::from_millis(100)).is_ok() {
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