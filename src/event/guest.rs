use crossterm::event::{KeyCode, KeyEvent};
use crate::vmrun::Vmrun;
use crate::ui::state::*;
use super::AppState;

/// 处理客户系统登录事件
pub fn handle_guest_login_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.guest_login = None;
            state.app_mode = AppMode::List;
        }
        KeyCode::Tab => {
            if let Some(ref mut gl) = state.guest_login {
                gl.field_index = (gl.field_index + 1) % 2;
            }
        }
        KeyCode::Enter => {
            if let Some(ref gl) = state.guest_login {
                let user = gl.fields[0].trim().to_string();
                let pass = gl.fields[1].clone();
                let start_dir = "/".to_string();
                match Vmrun::list_directory_in_guest(&gl.vmx_path, &user, &pass, &start_dir) {
                    Ok(entries) => {
                        state.guest_state = Some(GuestFilesState {
                            vmx_path: gl.vmx_path.clone(),
                            vm_name: gl.vm_name.clone(),
                            user, pass,
                            current_dir: start_dir,
                            entries, selected: 0,
                        });
                        state.app_mode = AppMode::GuestFiles;
                        state.message = None;
                    }
                    Err(e) => {
                        state.set_message(format!("✗ 登录失败: {}", e));
                        state.app_mode = AppMode::List;
                    }
                }
            }
            state.guest_login = None;
        }
        KeyCode::Backspace => {
            if let Some(ref mut gl) = state.guest_login {
                gl.fields[gl.field_index].pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut gl) = state.guest_login {
                if gl.fields[gl.field_index].len() < 64 {
                    gl.fields[gl.field_index].push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理客户系统文件浏览事件
pub fn handle_guest_files_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.app_mode = AppMode::List;
            state.guest_state = None;
            state.message = None;
        }
        KeyCode::Char('s') | KeyCode::Down => {
            if let Some(ref mut gs) = state.guest_state {
                if !gs.entries.is_empty() && gs.selected < gs.entries.len().saturating_sub(1) {
                    gs.selected += 1;
                }
            }
        }
        KeyCode::Char('w') | KeyCode::Up => {
            if let Some(ref mut gs) = state.guest_state {
                if gs.selected > 0 {
                    gs.selected -= 1;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(ref mut gs) = state.guest_state {
                if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                    let new_dir = if entry == ".." {
                        let parent = std::path::Path::new(&gs.current_dir)
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string());
                        if parent.is_empty() { "/".to_string() } else { parent }
                    } else {
                        let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                        format!("{}{}{}", gs.current_dir, sep, entry)
                    };
                    match Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &new_dir) {
                        Ok(entries) => {
                            gs.current_dir = new_dir;
                            gs.entries = entries;
                            gs.selected = 0;
                        }
                        Err(_) => {
                            state.set_message("✗ 无法进入（可能是文件）".to_string());
                        }
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            state.guest_file_input = Some(GuestFileInputState {
                action: GuestFileAction::Upload,
                buffer: String::new(),
            });
            state.app_mode = AppMode::GuestFileInput;
        }
        KeyCode::Char('o') => {
            if let Some(ref gs) = state.guest_state {
                if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                    if entry != ".." {
                        let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                        let guest_path = format!("{}{}{}", gs.current_dir, sep, entry);
                        state.guest_file_input = Some(GuestFileInputState {
                            action: GuestFileAction::Download(guest_path),
                            buffer: String::new(),
                        });
                        state.app_mode = AppMode::GuestFileInput;
                    }
                }
            }
        }
        KeyCode::Char('m') => {
            state.guest_file_input = Some(GuestFileInputState {
                action: GuestFileAction::Mkdir,
                buffer: String::new(),
            });
            state.app_mode = AppMode::GuestFileInput;
        }
        KeyCode::Char('d') => {
            if let Some(ref gs) = state.guest_state {
                if let Some(entry) = gs.entries.get(gs.selected) {
                    if entry != ".." {
                        state.app_mode = AppMode::GuestFileConfirm;
                    }
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理客户系统文件操作输入事件
pub fn handle_guest_file_input_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            state.guest_file_input = None;
            state.app_mode = AppMode::GuestFiles;
        }
        KeyCode::Enter => {
            if let (Some(ref gfi), Some(ref mut gs)) = (&state.guest_file_input, &mut state.guest_state) {
                let path = gfi.buffer.trim().to_string();
                if !path.is_empty() {
                    let result = match &gfi.action {
                        GuestFileAction::Upload => {
                            let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                            let filename = std::path::Path::new(&path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "file".to_string());
                            let guest_dest = format!("{}{}{}", gs.current_dir, sep, filename);
                            Vmrun::copy_file_to_guest(&gs.vmx_path, &gs.user, &gs.pass, &path, &guest_dest)
                                .map(|_| format!("✓ 已上传到 {}", guest_dest))
                        }
                        GuestFileAction::Download(guest_path) => {
                            Vmrun::copy_file_from_guest(&gs.vmx_path, &gs.user, &gs.pass, guest_path, &path)
                                .map(|_| format!("✓ 已下载到 {}", path))
                        }
                        GuestFileAction::Mkdir => {
                            let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                            let new_dir = format!("{}{}{}", gs.current_dir, sep, path);
                            Vmrun::create_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &new_dir)
                                .map(|_| format!("✓ 已创建目录 {}", new_dir))
                        }
                    };
                    match result {
                        Ok(msg) => state.message = Some(msg),
                        Err(e) => state.message = Some(format!("✗ {}", e)),
                    }
                    state.message_timer = Some(std::time::Instant::now());
                    // 刷新目录
                    if let Ok(entries) = Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &gs.current_dir) {
                        gs.entries = entries;
                    }
                }
            }
            state.guest_file_input = None;
            state.app_mode = AppMode::GuestFiles;
        }
        KeyCode::Backspace => {
            if let Some(ref mut gfi) = state.guest_file_input {
                gfi.buffer.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut gfi) = state.guest_file_input {
                if gfi.buffer.len() < 256 {
                    gfi.buffer.push(c);
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理客户系统文件删除确认事件
pub fn handle_guest_file_confirm_event(state: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(ref mut gs) = state.guest_state {
                if let Some(entry) = gs.entries.get(gs.selected).cloned() {
                    let sep = if gs.current_dir.ends_with('/') { "" } else { "/" };
                    let full_path = format!("{}{}{}", gs.current_dir, sep, entry);
                    match Vmrun::delete_file_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &full_path) {
                        Ok(()) => {
                            state.message = Some(format!("✓ 已删除 {}", entry));
                            if let Ok(entries) = Vmrun::list_directory_in_guest(&gs.vmx_path, &gs.user, &gs.pass, &gs.current_dir) {
                                gs.entries = entries;
                                if gs.selected >= gs.entries.len() && gs.selected > 0 {
                                    gs.selected -= 1;
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
            state.app_mode = AppMode::GuestFiles;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.app_mode = AppMode::GuestFiles;
        }
        _ => {}
    }
    false
}
