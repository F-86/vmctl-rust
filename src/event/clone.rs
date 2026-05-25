use crossterm::event::{KeyCode, KeyEvent};
use crate::vmrun::Vmrun;
use crate::ui::state::*;
use super::AppState;

/// 处理克隆输入事件
pub fn handle_clone_input_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.clone_input = None;
            state.app_mode = AppMode::List;
        }
        KeyCode::Tab => {
            if let Some(ref mut ci) = state.clone_input {
                ci.clone_type = match ci.clone_type {
                    CloneType::Full => CloneType::Linked,
                    CloneType::Linked => CloneType::Full,
                };
            }
        }
        KeyCode::Enter => {
            if let Some(ref ci) = state.clone_input {
                let name = ci.buffer.trim().to_string();
                if !name.is_empty() {
                    let source_dir = ci.source_vmx.parent()
                        .and_then(|p| p.parent())
                        .unwrap_or(std::path::Path::new("."));
                    let dest_dir = source_dir.join(format!("{}.vmwarevm", name));
                    let dest_vmx = dest_dir.join(format!("{}.vmx", name));

                    match Vmrun::clone_vm(
                        &ci.source_vmx,
                        &dest_vmx,
                        ci.clone_type.as_str(),
                        &name,
                    ) {
                        Ok(()) => {
                            state.set_message(format!(
                                "✓ 已克隆为 \"{}\" ({})", name, ci.clone_type.display_name()
                            ));
                        }
                        Err(e) => {
                            state.set_message(format!("✗ 克隆失败: {}", e));
                        }
                    }
                }
            }
            state.clone_input = None;
            state.app_mode = AppMode::List;
        }
        KeyCode::Backspace => {
            if let Some(ref mut ci) = state.clone_input {
                ci.buffer.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut ci) = state.clone_input {
                if ci.buffer.len() < 64 {
                    ci.buffer.push(c);
                }
            }
        }
        _ => {}
    }
    false
}
