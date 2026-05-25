use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
};

/// 渲染顶部标题栏（ASCII 艺术字 + 操作提示 + 系统资源）
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    vm_count: usize,
    ascii_art: &str,
    cpu_usage: f32,
    mem_usage: f32,
    vmrest_running: bool,
) {
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
    let ascii_area = Rect::new(
        inner.x + inner.width - ascii_width,
        inner.y,
        ascii_width,
        inner.height,
    );
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
        Line::from(vec![Span::styled("<t>", Style::new().fg(Color::Green)), Span::raw(" ssh")]),
        Line::from(vec![Span::styled("<R>", Style::new().fg(Color::Yellow)), Span::raw(" rest")]),
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

    // 左下角：系统资源 + VM 数量 + vmrest 状态
    let rest_status = if vmrest_running { "●" } else { "○" };
    let sys_text = format!(
        "CPU: {:.0}%  MEM: {:.0}%  VMs: {}  REST: {}",
        cpu_usage, mem_usage, vm_count, rest_status
    );
    let sys_para = Paragraph::new(Text::from(sys_text))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(sys_para, Rect::new(inner.x, inner.y + 5, 35, 1));
}
