use std::path::PathBuf;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, TableState},
};

use super::state::{SharedFolderState, SharedFolderEntry, SharedFolderInputState};

/// 从 .vmx 文件解析共享文件夹配置
pub fn parse_shared_folders_from_vmx(vmx_path: &PathBuf) -> (Vec<SharedFolderEntry>, bool) {
    let mut folders = Vec::new();
    let mut enabled = false;

    if let Ok(content) = std::fs::read_to_string(vmx_path) {
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
pub fn render_shared_folder_view(frame: &mut Frame, ss: &SharedFolderState, message: &Option<String>, ascii_art: &str) {
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
            Constraint::Length(16),
            Constraint::Min(25),
            Constraint::Length(8),
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
pub fn render_shared_folder_header(frame: &mut Frame, area: Rect, ss: &SharedFolderState, ascii_art: &str) {
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
pub fn render_shared_folder_input(frame: &mut Frame, si: &SharedFolderInputState) {
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
pub fn render_shared_folder_delete_confirm(frame: &mut Frame, ss: &SharedFolderState) {
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
