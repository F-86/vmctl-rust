use crossterm::event::{KeyCode, KeyEvent};
use crate::vmrun::Vmrun;
use crate::ui::state::*;
use super::AppState;

/// 处理端口转发列表事件
pub fn handle_portfwd_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('f') => {
            state.app_mode = AppMode::List;
            state.portfwd_state = None;
            state.message = None;
        }
        KeyCode::Char('s') | KeyCode::Down => {
            if let Some(ref mut pfs) = state.portfwd_state {
                if !pfs.rules.is_empty() && pfs.selected < pfs.rules.len().saturating_sub(1) {
                    pfs.selected += 1;
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Up => {
            if let Some(ref mut pfs) = state.portfwd_state {
                if pfs.selected > 0 {
                    pfs.selected -= 1;
                }
            }
        }
        KeyCode::Char('a') => {
            state.portfwd_input = Some(PortForwardInputState::new());
            state.app_mode = AppMode::PortForwardInput;
            state.message = None;
        }
        KeyCode::Char('d') => {
            if let Some(ref pfs) = state.portfwd_state {
                if !pfs.rules.is_empty() {
                    state.app_mode = AppMode::PortForwardConfirm;
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理端口转发输入事件
pub fn handle_portfwd_input_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.portfwd_input = None;
            state.app_mode = AppMode::PortForward;
        }
        KeyCode::Tab => {
            if let Some(ref mut pfi) = state.portfwd_input {
                pfi.field_index = (pfi.field_index + 1) % 5;
            }
        }
        KeyCode::BackTab => {
            if let Some(ref mut pfi) = state.portfwd_input {
                pfi.field_index = if pfi.field_index == 0 { 4 } else { pfi.field_index - 1 };
            }
        }
        KeyCode::Enter => {
            if let (Some(ref pfi), Some(ref mut pfs)) = (&state.portfwd_input, &mut state.portfwd_state) {
                let protocol = pfi.fields[0].trim();
                let host_port: u16 = pfi.fields[1].trim().parse().unwrap_or(0);
                let guest_ip = pfi.fields[2].trim();
                let guest_port: u16 = pfi.fields[3].trim().parse().unwrap_or(0);
                let description = pfi.fields[4].trim();

                if (protocol == "tcp" || protocol == "udp")
                    && host_port > 0
                    && !guest_ip.is_empty()
                    && guest_port > 0
                {
                    match Vmrun::set_port_forwarding(
                        &pfs.network, protocol, host_port,
                        guest_ip, guest_port, description,
                    ) {
                        Ok(()) => {
                            state.message = Some(format!(
                                "✓ 已添加 {} :{} → {}:{}",
                                protocol, host_port, guest_ip, guest_port
                            ));
                            if let Ok(rules) = Vmrun::list_port_forwardings(&pfs.network) {
                                pfs.rules = rules;
                            }
                        }
                        Err(e) => {
                            state.message = Some(format!("✗ 添加失败: {}", e));
                        }
                    }
                    state.message_timer = Some(std::time::Instant::now());
                } else {
                    state.set_message("✗ 请填写完整（协议 tcp/udp，端口 > 0）".to_string());
                }
            }
            state.portfwd_input = None;
            state.app_mode = AppMode::PortForward;
        }
        KeyCode::Backspace => {
            if let Some(ref mut pfi) = state.portfwd_input {
                pfi.current_field_mut().pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut pfi) = state.portfwd_input {
                if pfi.current_field().len() < 40 {
                    pfi.current_field_mut().push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理端口转发删除确认事件
pub fn handle_portfwd_confirm_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(ref mut pfs) = state.portfwd_state {
                if let Some(rule) = pfs.rules.get(pfs.selected).cloned() {
                    match Vmrun::delete_port_forwarding(
                        &pfs.network, &rule.protocol, rule.host_port,
                    ) {
                        Ok(()) => {
                            state.message = Some(format!("✓ 已删除 {} :{}", rule.protocol, rule.host_port));
                            if let Ok(rules) = Vmrun::list_port_forwardings(&pfs.network) {
                                pfs.rules = rules;
                                if pfs.selected >= pfs.rules.len() && pfs.selected > 0 {
                                    pfs.selected -= 1;
                                }
                            }
                        }
                        Err(e) => {
                            state.message = Some(format!("✗ 删除失败: {}", e));
                        }
                    }
                    state.message_timer = Some(std::time::Instant::now());
                }
            }
            state.app_mode = AppMode::PortForward;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.app_mode = AppMode::PortForward;
        }
        _ => {}
    }
    false
}
