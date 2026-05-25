use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
};

use super::state::SshLoginState;

/// 渲染 SSH 连接凭据输入弹窗
pub fn render_ssh_login(frame: &mut Frame, sl: &SshLoginState) {
    let area = frame.area();
    let popup_width = 48u16.min(area.width - 4);
    let popup_height = 8u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = format!(" SSH: {} ({}) ", sl.vm_name, sl.ip);
    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Green).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let labels = ["用户名", "端口"];
    let mut lines = Vec::new();
    for i in 0..2 {
        let marker = if i == sl.field_index { "▶" } else { " " };
        let display = if i == sl.field_index {
            format!("{}_ ", sl.fields[i])
        } else {
            format!("{} ", sl.fields[i])
        };
        let style = if i == sl.field_index {
            Style::new().fg(Color::White)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} {}: ", marker, labels[i]), style),
            Span::styled(display, Style::new().fg(Color::Cyan)),
        ]));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        format!(" ssh {}@{} -p {}", sl.fields[0], sl.ip, sl.fields[1]),
        Style::new().fg(Color::Green),
    )));
    lines.push(Line::from(Span::styled(
        " [Tab] 切换  [Enter] 连接  [Esc] 取消",
        Style::new().fg(Color::DarkGray),
    )));
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
