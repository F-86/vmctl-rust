use crossterm::event::{KeyCode, KeyEvent};
use crate::ui::state::*;
use super::{AppState, SshExecRequest};

/// 处理 SSH 登录输入事件
pub fn handle_ssh_login_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Tab => {
            if let Some(ref mut sl) = state.ssh_login {
                sl.field_index = (sl.field_index + 1) % 2;
            }
        }
        KeyCode::BackTab => {
            if let Some(ref mut sl) = state.ssh_login {
                sl.field_index = if sl.field_index == 0 { 1 } else { 0 };
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut sl) = state.ssh_login {
                sl.fields[sl.field_index].push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(ref mut sl) = state.ssh_login {
                sl.fields[sl.field_index].pop();
            }
        }
        KeyCode::Enter => {
            if let Some(sl) = state.ssh_login.take() {
                let user = sl.fields[0].trim().to_string();
                let port = sl.fields[1].trim().to_string();
                if user.is_empty() {
                    state.set_message("✗ 用户名不能为空".to_string());
                    state.ssh_login = Some(sl);
                    return false;
                }
                // 设置 SSH 执行请求，主循环会处理
                state.ssh_exec = Some(SshExecRequest {
                    user,
                    ip: sl.ip,
                    port: if port.is_empty() { "22".to_string() } else { port },
                });
                state.app_mode = AppMode::List;
            }
        }
        KeyCode::Esc => {
            state.ssh_login = None;
            state.app_mode = AppMode::List;
        }
        _ => {}
    }
    false
}
