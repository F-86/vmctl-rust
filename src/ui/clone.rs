use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
};

use super::state::CloneInputState;

/// 渲染克隆 VM 输入框
pub fn render_clone_input(frame: &mut Frame, ci: &CloneInputState) {
    let area = frame.area();

    let popup_width = 52u16.min(area.width - 4);
    let popup_height = 7u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let title = format!(" 克隆: {} ", ci.source_name);
    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Cyan).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let type_label = format!(" 类型: {} [Tab 切换]", ci.clone_type.display_name());
    let input_text = format!(" 名称: {}_ ", ci.buffer);

    let input_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(type_label, Style::new().fg(Color::Yellow))),
        Line::from(Span::styled(input_text, Style::new().fg(Color::White))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            " [Enter] 克隆  [Tab] 切换类型  [Esc] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(input_para, inner);
}
