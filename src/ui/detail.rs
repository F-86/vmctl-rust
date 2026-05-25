use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, TableState},
};

use super::state::{DetailState, DetailRow, EditField};

/// 获取详情视图的行数据
pub fn get_detail_rows(ds: &DetailState) -> Vec<DetailRow> {
    let mut rows = Vec::new();

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

    for net in &ds.config.networks {
        let mac_str = net.mac_address.as_deref().unwrap_or("-");
        rows.push(DetailRow {
            label: format!("网络 eth{}", net.index),
            value: format!("{} ({})", net.connection_type, mac_str),
            editable: false,
            edit_field: None,
        });
    }

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
pub fn render_detail_view(frame: &mut Frame, ds: &DetailState, message: &Option<String>, ascii_art: &str) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(1),
        ])
        .split(area);

    render_detail_header(frame, chunks[0], ds, ascii_art);
    render_detail_table(frame, chunks[1], ds);

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

/// 渲染详情视图的顶部 header
pub fn render_detail_header(frame: &mut Frame, area: Rect, ds: &DetailState, ascii_art: &str) {
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
        Line::from(vec![Span::styled("<e>", Style::new().fg(Color::Green)), Span::raw(" edit")]),
        Line::from(vec![Span::styled("<W>", Style::new().fg(Color::Yellow)), Span::raw(" save")]),
    ];
    let right_col = vec![
        Line::from(vec![Span::styled("<u>", Style::new().fg(Color::Yellow)), Span::raw(" undo")]),
        Line::from(vec![Span::styled("<esc>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
        Line::from(vec![Span::styled("<i>", Style::new().fg(Color::Yellow)), Span::raw(" back")]),
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

    let dirty_mark = if ds.dirty { " [已修改]" } else { "" };
    let info_text = format!("VM: {}{}", ds.vm_name, dirty_mark);
    let info_para = Paragraph::new(Text::from(info_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(info_para, Rect::new(inner.x, inner.y + 5, 50, 1));
}

/// 渲染详情视图的配置表格
pub fn render_detail_table(frame: &mut Frame, area: Rect, ds: &DetailState) {
    let rows = get_detail_rows(ds);

    let col_widths = &[
        Constraint::Length(3),
        Constraint::Length(16),
        Constraint::Min(20),
    ];

    let header = Row::new(vec![
        Cell::from(Span::raw(" ")),
        Cell::from(Span::raw(" PROPERTY")),
        Cell::from(Span::raw(" VALUE")),
    ])
    .style(Style::new().fg(Color::White).bg(Color::DarkGray));

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
