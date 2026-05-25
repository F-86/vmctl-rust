use crossterm::event::{KeyCode, KeyEvent};
use crate::vm::VmState;
use crate::vm::Vm;
use crate::vmx::VmxFile;
use crate::ui::state::*;
use crate::ui::detail::get_detail_rows;
use super::AppState;

/// 处理详情视图事件
pub fn handle_detail_event(state: &mut AppState, key: KeyEvent, vms: &[Vm]) -> bool {
    let row_count = get_detail_rows(state.detail_state.as_ref().unwrap()).len();
    match key.code {
        KeyCode::Esc | KeyCode::Char('i') => {
            state.app_mode = AppMode::List;
            state.detail_state = None;
            state.message = None;
        }
        KeyCode::Char('s') | KeyCode::Down => {
            if let Some(ref mut ds) = state.detail_state {
                if ds.selected_row < row_count.saturating_sub(1) {
                    ds.selected_row += 1;
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Up => {
            if let Some(ref mut ds) = state.detail_state {
                if ds.selected_row > 0 {
                    ds.selected_row -= 1;
                }
            }
        }
        KeyCode::Char('e') => {
            if let Some(ref ds) = state.detail_state {
                let rows = get_detail_rows(ds);
                if let Some(row) = rows.get(ds.selected_row) {
                    if row.editable {
                        let vm_state = vms.iter()
                            .find(|vm| vm.vmx_path == ds.vmx_path)
                            .map(|vm| &vm.state);
                        if vm_state == Some(&VmState::Stopped) {
                            let init_buffer = match &row.edit_field {
                                Some(EditField::DiskSize(idx)) => {
                                    ds.config.disks.get(*idx)
                                        .and_then(|d| d.size_gb)
                                        .map(|gb| gb.to_string())
                                        .unwrap_or_default()
                                }
                                _ => row.value.clone(),
                            };
                            state.edit_state = Some(EditState {
                                field: row.edit_field.clone().unwrap(),
                                buffer: init_buffer.clone(),
                                original_value: init_buffer,
                            });
                            state.app_mode = AppMode::Editing;
                            state.message = None;
                        } else {
                            state.set_message("✗ 虚拟机必须停止后才能编辑配置".to_string());
                        }
                    }
                }
            }
        }
        KeyCode::Char('W') => {
            if let Some(ref ds) = state.detail_state {
                if ds.dirty {
                    let vm_state = vms.iter()
                        .find(|vm| vm.vmx_path == ds.vmx_path)
                        .map(|vm| &vm.state);
                    if vm_state == Some(&VmState::Stopped) {
                        state.app_mode = AppMode::Confirm;
                    } else {
                        state.set_message("✗ 虚拟机必须停止后才能保存配置".to_string());
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(ref mut ds) = state.detail_state {
                if ds.dirty {
                    match VmxFile::parse(&ds.vmx_path) {
                        Ok(vmx_file) => {
                            ds.config = vmx_file.hardware_config();
                            ds.vmx_file = vmx_file;
                            ds.dirty = false;
                            state.set_message("✓ 已撤销所有修改".to_string());
                        }
                        Err(e) => state.set_message(format!("✗ 重新加载失败: {}", e)),
                    }
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理编辑模式事件
pub fn handle_editing_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.edit_state = None;
            state.app_mode = AppMode::Detail;
        }
        KeyCode::Enter => {
            if let (Some(ref es), Some(ref mut ds)) = (&state.edit_state, &mut state.detail_state) {
                let value: u64 = es.buffer.parse().unwrap_or(0);
                let valid = match &es.field {
                    EditField::CpuCount => value >= 1 && value <= 128,
                    EditField::MemoryMb => value >= 128 && value <= 1048576,
                    EditField::DiskSize(idx) => {
                        if let Some(disk) = ds.config.disks.get(*idx) {
                            let current = disk.size_gb.unwrap_or(0);
                            value > current && value <= 65536
                        } else {
                            false
                        }
                    }
                };

                if valid {
                    match &es.field {
                        EditField::CpuCount => {
                            ds.vmx_file.set("numvcpus", &es.buffer);
                            ds.config = ds.vmx_file.hardware_config();
                            ds.dirty = true;
                            state.message = None;
                        }
                        EditField::MemoryMb => {
                            ds.vmx_file.set("memsize", &es.buffer);
                            ds.config = ds.vmx_file.hardware_config();
                            ds.dirty = true;
                            state.message = None;
                        }
                        EditField::DiskSize(idx) => {
                            if let Some(disk) = ds.config.disks.get(*idx) {
                                match crate::vmx::expand_disk(&disk.full_path, value) {
                                    Ok(msg) => {
                                        state.message = Some(format!("✓ {}", msg));
                                        ds.config = ds.vmx_file.hardware_config();
                                    }
                                    Err(e) => {
                                        state.message = Some(format!("✗ {}", e));
                                    }
                                }
                                state.message_timer = Some(std::time::Instant::now());
                            }
                        }
                    }
                } else {
                    let hint = match &es.field {
                        EditField::CpuCount => "CPU 数量范围: 1-128".to_string(),
                        EditField::MemoryMb => "内存范围: 128-1048576 MB".to_string(),
                        EditField::DiskSize(idx) => {
                            let current = ds.config.disks.get(*idx)
                                .and_then(|d| d.size_gb)
                                .unwrap_or(0);
                            format!("磁盘只能扩容，须大于当前 {}GB", current)
                        }
                    };
                    state.set_message(format!("✗ 无效值。{}", hint));
                }
            }
            state.edit_state = None;
            state.app_mode = AppMode::Detail;
        }
        KeyCode::Backspace => {
            if let Some(ref mut es) = state.edit_state {
                es.buffer.pop();
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(ref mut es) = state.edit_state {
                if es.buffer.len() < 7 {
                    es.buffer.push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理保存确认事件
pub fn handle_confirm_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(ref ds) = state.detail_state {
                match ds.vmx_file.backup_and_write() {
                    Ok(()) => {
                        state.set_message("✓ 配置已保存".to_string());
                        if let Some(ref mut ds) = state.detail_state {
                            ds.dirty = false;
                        }
                    }
                    Err(e) => state.set_message(format!("✗ 保存失败: {}", e)),
                }
            }
            state.app_mode = AppMode::Detail;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.app_mode = AppMode::Detail;
        }
        _ => {}
    }
    false
}
