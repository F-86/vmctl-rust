use crossterm::event::{KeyCode, KeyEvent};
use crate::vmrun::Vmrun;
use crate::ui::state::*;
use super::AppState;

/// 处理共享文件夹列表事件
pub fn handle_shared_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('h') => {
            state.app_mode = AppMode::List;
            state.shared_state = None;
            state.message = None;
        }
        KeyCode::Char('s') | KeyCode::Down => {
            if let Some(ref mut ss) = state.shared_state {
                if !ss.folders.is_empty() && ss.selected < ss.folders.len().saturating_sub(1) {
                    ss.selected += 1;
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Up => {
            if let Some(ref mut ss) = state.shared_state {
                if ss.selected > 0 {
                    ss.selected -= 1;
                }
            }
        }
        KeyCode::Char('a') => {
            state.shared_input = Some(SharedFolderInputState::new());
            state.app_mode = AppMode::SharedFolderInput;
            state.message = None;
        }
        KeyCode::Char('d') => {
            if let Some(ref ss) = state.shared_state {
                if !ss.folders.is_empty() {
                    state.app_mode = AppMode::SharedFolderConfirm;
                }
            }
        }
        KeyCode::Char('e') => {
            if let Some(ref mut ss) = state.shared_state {
                let result = if ss.enabled {
                    Vmrun::disable_shared_folders(&ss.vmx_path)
                } else {
                    Vmrun::enable_shared_folders(&ss.vmx_path)
                };
                match result {
                    Ok(()) => {
                        ss.enabled = !ss.enabled;
                        let status = if ss.enabled { "已启用" } else { "已禁用" };
                        state.set_message(format!("✓ 共享文件夹{}", status));
                    }
                    Err(e) => state.set_message(format!("✗ 操作失败: {}", e)),
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理共享文件夹输入事件
pub fn handle_shared_input_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.shared_input = None;
            state.app_mode = AppMode::SharedFolder;
        }
        KeyCode::Tab => {
            if let Some(ref mut si) = state.shared_input {
                si.field_index = (si.field_index + 1) % 2;
            }
        }
        KeyCode::Enter => {
            if let (Some(ref si), Some(ref mut ss)) = (&state.shared_input, &mut state.shared_state) {
                let name = si.fields[0].trim().to_string();
                let host_path = si.fields[1].trim().to_string();
                if !name.is_empty() && !host_path.is_empty() {
                    match Vmrun::add_shared_folder(&ss.vmx_path, &name, &host_path) {
                        Ok(()) => {
                            state.message = Some(format!("✓ 已添加共享 \"{}\"", name));
                            ss.folders.push(SharedFolderEntry {
                                name, host_path, writable: true,
                            });
                        }
                        Err(e) => {
                            state.message = Some(format!("✗ 添加失败: {}", e));
                        }
                    }
                    state.message_timer = Some(std::time::Instant::now());
                } else {
                    state.set_message("✗ 名称和路径不能为空".to_string());
                }
            }
            state.shared_input = None;
            state.app_mode = AppMode::SharedFolder;
        }
        KeyCode::Backspace => {
            if let Some(ref mut si) = state.shared_input {
                si.fields[si.field_index].pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut si) = state.shared_input {
                if si.fields[si.field_index].len() < 128 {
                    si.fields[si.field_index].push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理共享文件夹删除确认事件
pub fn handle_shared_confirm_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(ref mut ss) = state.shared_state {
                if let Some(folder) = ss.folders.get(ss.selected).cloned() {
                    match Vmrun::remove_shared_folder(&ss.vmx_path, &folder.name) {
                        Ok(()) => {
                            state.message = Some(format!("✓ 已移除共享 \"{}\"", folder.name));
                            ss.folders.remove(ss.selected);
                            if ss.selected >= ss.folders.len() && ss.selected > 0 {
                                ss.selected -= 1;
                            }
                        }
                        Err(e) => {
                            state.message = Some(format!("✗ 移除失败: {}", e));
                        }
                    }
                    state.message_timer = Some(std::time::Instant::now());
                }
            }
            state.app_mode = AppMode::SharedFolder;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.app_mode = AppMode::SharedFolder;
        }
        _ => {}
    }
    false
}
