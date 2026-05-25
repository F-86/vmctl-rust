use crossterm::event::{KeyCode, KeyEvent};
use crate::vmrun::Vmrun;
use crate::ui::state::*;
use super::AppState;

/// 处理快照视图事件
pub fn handle_snapshot_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => {
            state.app_mode = AppMode::List;
            state.snapshot_state = None;
            state.message = None;
        }
        KeyCode::Char('s') | KeyCode::Down => {
            if let Some(ref mut ss) = state.snapshot_state {
                if !ss.snapshots.is_empty() && ss.selected < ss.snapshots.len().saturating_sub(1) {
                    ss.selected += 1;
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Up => {
            if let Some(ref mut ss) = state.snapshot_state {
                if ss.selected > 0 {
                    ss.selected -= 1;
                }
            }
        }
        KeyCode::Char('c') => {
            state.snapshot_input = Some(SnapshotInputState {
                buffer: String::new(),
            });
            state.app_mode = AppMode::SnapshotInput;
            state.message = None;
        }
        KeyCode::Char('d') => {
            if let Some(ref ss) = state.snapshot_state {
                if let Some(name) = ss.snapshots.get(ss.selected) {
                    state.snapshot_action = Some(SnapshotAction::Delete(name.clone()));
                    state.app_mode = AppMode::SnapshotConfirm;
                }
            }
        }
        KeyCode::Char('r') => {
            if let Some(ref ss) = state.snapshot_state {
                if let Some(name) = ss.snapshots.get(ss.selected) {
                    state.snapshot_action = Some(SnapshotAction::Revert(name.clone()));
                    state.app_mode = AppMode::SnapshotConfirm;
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理快照输入事件
pub fn handle_snapshot_input_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.snapshot_input = None;
            state.app_mode = AppMode::Snapshot;
        }
        KeyCode::Enter => {
            if let (Some(ref sis), Some(ref mut ss)) = (&state.snapshot_input, &mut state.snapshot_state) {
                let name = sis.buffer.trim().to_string();
                if !name.is_empty() {
                    match Vmrun::create_snapshot(&ss.vmx_path, &name) {
                        Ok(()) => {
                            state.message = Some(format!("✓ 快照 \"{}\" 已创建", name));
                            if let Ok(snapshots) = Vmrun::list_snapshots(&ss.vmx_path) {
                                ss.snapshots = snapshots;
                            }
                        }
                        Err(e) => {
                            state.message = Some(format!("✗ 创建快照失败: {}", e));
                        }
                    }
                    state.message_timer = Some(std::time::Instant::now());
                }
            }
            state.snapshot_input = None;
            state.app_mode = AppMode::Snapshot;
        }
        KeyCode::Backspace => {
            if let Some(ref mut sis) = state.snapshot_input {
                sis.buffer.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut sis) = state.snapshot_input {
                if sis.buffer.len() < 64 {
                    sis.buffer.push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理快照确认事件
pub fn handle_snapshot_confirm_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let (Some(ref action), Some(ref mut ss)) = (&state.snapshot_action, &mut state.snapshot_state) {
                match action {
                    SnapshotAction::Delete(name) => {
                        match Vmrun::delete_snapshot(&ss.vmx_path, name) {
                            Ok(()) => {
                                state.message = Some(format!("✓ 快照 \"{}\" 已删除", name));
                                if let Ok(snapshots) = Vmrun::list_snapshots(&ss.vmx_path) {
                                    ss.snapshots = snapshots;
                                    if ss.selected >= ss.snapshots.len() && ss.selected > 0 {
                                        ss.selected -= 1;
                                    }
                                }
                            }
                            Err(e) => {
                                state.message = Some(format!("✗ 删除快照失败: {}", e));
                            }
                        }
                    }
                    SnapshotAction::Revert(name) => {
                        match Vmrun::revert_to_snapshot(&ss.vmx_path, name) {
                            Ok(()) => {
                                state.message = Some(format!("✓ 已恢复到快照 \"{}\"", name));
                            }
                            Err(e) => {
                                state.message = Some(format!("✗ 恢复快照失败: {}", e));
                            }
                        }
                    }
                }
                state.message_timer = Some(std::time::Instant::now());
            }
            state.snapshot_action = None;
            state.app_mode = AppMode::Snapshot;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.snapshot_action = None;
            state.app_mode = AppMode::Snapshot;
        }
        _ => {}
    }
    false
}
