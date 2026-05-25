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

use vm::{Vm, VmState};
use manager::VmManager;
use vmrun::set_vmrun_path;

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
        terminal.draw(|frame| ui(frame, &vms, &mut list_state, &message, vm_count, ascii_art, cpu_usage, mem_usage))?;

        // 设置超时以允许定时刷新
        if event::poll(Duration::from_millis(100)).is_ok() {
            if let Event::Key(key) = event::read()? {
                // 忽略 key release 事件
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('s') | KeyCode::Down => {
                        // 向下选择
                        let new_index = list_state.state.selected()
                            .map(|i| (i + 1).min(vms.len().saturating_sub(1)))
                            .unwrap_or(0);
                        list_state.with_selection(&vms, Some(new_index));
                        message = None;
                    }
                    KeyCode::Char('w') | KeyCode::Up => {
                        // 向上选择
                        let new_index = list_state.state.selected()
                            .map(|i| i.saturating_sub(1))
                            .unwrap_or(0);
                        list_state.with_selection(&vms, Some(new_index));
                        message = None;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Enter | KeyCode::Char('x') | KeyCode::Char('p') | KeyCode::Char('r') => {
                        // 执行操作
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