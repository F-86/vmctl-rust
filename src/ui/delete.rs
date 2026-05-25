use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Paragraph},
};

use crate::vm::Vm;
use super::state::VmListState;

/// 渲染删除 VM 确认对话框
pub fn render_delete_vm_confirm(frame: &mut Frame, vms: &[Vm], list_state: &VmListState) {
    let area = frame.area();
    let popup_width = 48u16.min(area.width - 4);
    let popup_height = 5u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    let popup_block = Block::default()
        .title(" ⚠ 确认删除 ")
        .borders(Borders::ALL)
        .style(Style::new().fg(Color::Red).bg(Color::Black));
    let inner = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let vm_name = list_state.selected_vmx.as_ref()
        .and_then(|path| vms.iter().find(|vm| &vm.vmx_path == path))
        .map(|vm| vm.name.as_str())
        .unwrap_or("?");

    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(format!(" 永久删除 \"{}\"？此操作不可恢复！", vm_name), Style::new().fg(Color::White))),
        Line::from(Span::styled(" [y] 确认删除  [n] 取消", Style::new().fg(Color::DarkGray))),
    ]));
    frame.render_widget(para, inner);
}
