use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, TableState},
};

use super::state::{PortForwardState, PortForwardInputState};

/// 渲染端口转发列表视图
pub fn render_portfwd_view(frame: &mut Frame, pfs: &PortForwardState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    render_portfwd_header(frame, chunks[0], pfs, ascii_art);

    if pfs.rules.is_empty() {
        let empty = Paragraph::new(Text::from("\n  (无端口转发规则)"))
            .style(Style::new().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).style(Style::new().bg(Color::Black)));
        frame.render_widget(empty, chunks[1]);
    } else {
        let col_widths = &[
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(22),
            Constraint::Min(15),
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
pub fn render_portfwd_header(frame: &mut Frame, area: Rect, pfs: &PortForwardState, ascii_art: &str) {
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
pub fn render_portfwd_input(frame: &mut Frame, pfi: &PortForwardInputState) {
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
pub fn render_portfwd_delete_confirm(frame: &mut Frame, pfs: &PortForwardState) {
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
