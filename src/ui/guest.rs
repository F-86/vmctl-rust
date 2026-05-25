use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, TableState},
};

use super::state::{GuestFilesState, GuestLoginState, GuestFileInputState, GuestFileAction};

/// 渲染客户系统凭据输入
pub fn render_guest_login(frame: &mut Frame, gl: &GuestLoginState) {
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
pub fn render_guest_files_view(frame: &mut Frame, gs: &GuestFilesState, message: &Option<String>, ascii_art: &str) {
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
pub fn render_guest_file_input(frame: &mut Frame, gfi: &GuestFileInputState) {
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
pub fn render_guest_file_delete_confirm(frame: &mut Frame, gs: &GuestFilesState) {
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
