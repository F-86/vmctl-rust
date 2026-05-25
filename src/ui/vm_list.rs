use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
};

use crate::vm::{Vm, VmState};
use super::state::VmListState;
use super::header::render_header;

/// 渲染主 UI（列表视图）
pub fn ui(
    frame: &mut Frame,
    vms: &[Vm],
    list_state: &mut VmListState,
    message: &Option<String>,
    vm_count: usize,
    ascii_art: &str,
    cpu_usage: f32,
    mem_usage: f32,
    vmrest_running: bool,
) {
    let area = frame.area();

    // 主布局：标题栏 + 表格区域
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(1),
        ])
        .split(area);

    // 顶部标题栏
    render_header(frame, chunks[0], vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);

    // 表格区域
    render_table(frame, chunks[1], vms, list_state);

    // 如果有消息，在表格下方显示
    if let Some(msg) = message {
        let msg_para = Paragraph::new(Text::from(format!(" {}", msg)))
            .style(Style::new().fg(Color::White).bg(Color::DarkGray))
            .alignment(Alignment::Left);
        frame.render_widget(
            msg_para,
            Rect::new(
                chunks[1].x,
                chunks[1].y + chunks[1].height.saturating_sub(3),
                chunks[1].width,
                3,
            ),
        );
    }
}

/// 渲染表格
pub fn render_table(frame: &mut Frame, area: Rect, vms: &[Vm], list_state: &mut VmListState) {
    let col_widths = &[
        Constraint::Length(30),
        Constraint::Length(15),
        Constraint::Length(18),
    ];

    let header = Row::new(vec![
        Cell::from(Span::raw(" NAME")),
        Cell::from(Span::raw(" STATE")),
        Cell::from(Span::raw(" IP")),
    ])
    .style(Style::new().fg(Color::White).bg(Color::DarkGray));

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
