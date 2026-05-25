use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
};

use super::state::{EditState, EditField};

/// 渲染编辑覆盖层（输入框）
pub fn render_edit_overlay(frame: &mut Frame, es: &EditState) {
    let area = frame.area();

    let popup_width = 40u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let clear_block = Block::default()
        .style(Style::new().bg(Color::Black));
    frame.render_widget(clear_block, popup_area);

    let field_name = match es.field {
        EditField::CpuCount => "CPU 核心数".to_string(),
        EditField::MemoryMb => "内存 (MB)".to_string(),
        EditField::DiskSize(_) => "磁盘大小 (GB) - 仅支持扩容".to_string(),
    };

    let title = format!(" 编辑: {} ", field_name);
    let input_text = format!(" > {}_ ", es.buffer);

    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let input_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(input_text, Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [Enter] 确认  [Esc] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(input_para, inner);
}

/// 渲染保存确认对话框
pub fn render_confirm_dialog(frame: &mut Frame) {
    let area = frame.area();

    let popup_width = 36u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" 确认保存 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Yellow).bg(Color::Black));

    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let confirm_para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(" 保存修改到 .vmx 文件？", Style::new().fg(Color::White))),
        Line::from(Span::styled(
            " [y] 确认  [n] 取消",
            Style::new().fg(Color::DarkGray),
        )),
    ]));
    frame.render_widget(confirm_para, inner);
}
