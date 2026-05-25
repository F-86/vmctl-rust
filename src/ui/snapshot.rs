use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, TableState},
};

use super::state::{SnapshotState, SnapshotInputState, SnapshotAction};

/// 渲染快照列表视图
pub fn render_snapshot_view(frame: &mut Frame, ss: &SnapshotState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    render_snapshot_header(frame, chunks[0], ss, ascii_art);
    render_snapshot_table(frame, chunks[1], ss);

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
pub fn render_snapshot_header(frame: &mut Frame, area: Rect, ss: &SnapshotState, ascii_art: &str) {
    let block = Block::default()
        .style(Style::new().bg(Color::Black))
        .borders(Borders::BOTTOM);

    let inner = block.inner(area);
    frame.render_widget(block, area);

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

    let left_col = vec![
        Line::from(vec![Span::styled("<w/s>", Style::new().fg(Color::Yellow)), Span::raw(" navigate")]),
        Line::from(vec![Span::styled("<c>", Style::new().fg(Color::Green)), Span::raw(" create")]),
        Line::from(vec![Span::styled("<d>", Style::new().fg(Color::Red)), Span::raw(" delete")]),
    ];
    let right_col = vec![
        Line::from(vec![Span::styled("<r>", Style::new().fg(Color::Yellow)), Span::raw(" revert")]),
        Line::from(vec![Span::styled("<esc>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
        Line::from(vec![Span::styled("<n>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
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

    let info_text = format!("Snapshots: {} ({})", ss.vm_name, ss.snapshots.len());
    let info_para = Paragraph::new(Text::from(info_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(info_para, Rect::new(inner.x, inner.y + 5, 50, 1));
}

/// 渲染快照表格
pub fn render_snapshot_table(frame: &mut Frame, area: Rect, ss: &SnapshotState) {
    if ss.snapshots.is_empty() {
        let empty_para = Paragraph::new(Text::from("\n  (无快照)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty_para, area);
        return;
    }

    let col_widths = &[
        Constraint::Length(3),
        Constraint::Min(30),
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
            Cell::from(Span::styled(format!(" {}", name), Style::new().fg(Color::White))),
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
pub fn render_snapshot_input(frame: &mut Frame, sis: &SnapshotInputState) {
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
pub fn render_snapshot_confirm(frame: &mut Frame, action: &SnapshotAction) {
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
