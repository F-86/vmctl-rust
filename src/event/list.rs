use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyEvent};
use crate::vm::{Vm, VmState};
use crate::manager::VmManager;
use crate::vmrun::Vmrun;
use crate::vmx::VmxFile;
use crate::vmrest::VmrestService;
use crate::ui::state::*;
use crate::ui::shared::parse_shared_folders_from_vmx;
use super::AppState;

/// 执行虚拟机操作
fn execute_operation(manager: &VmManager, vmx_path: &PathBuf, op: &str) -> Result<String, String> {
    match op {
        "start" => manager.start_vm(vmx_path).map(|_| "虚拟机已启动".to_string()).map_err(|e| e.to_string()),
        "stop" => manager.stop_vm(vmx_path).map(|_| "虚拟机已停止".to_string()).map_err(|e| e.to_string()),
        "suspend" => manager.suspend_vm(vmx_path).map(|_| "虚拟机已挂起".to_string()).map_err(|e| e.to_string()),
        "resume" => manager.resume_vm(vmx_path).map(|_| "虚拟机已继续运行".to_string()).map_err(|e| e.to_string()),
        _ => Err("未知操作".to_string()),
    }
}

/// 处理列表视图事件，返回 true 表示退出程序
pub fn handle_list_event(
    state: &mut AppState,
    key: KeyEvent,
    vms: &[Vm],
    list_state: &mut VmListState,
    manager: &VmManager,
    vmrest: &VmrestService,
) -> bool {
    match key.code {
        KeyCode::Char('s') | KeyCode::Down => {
            let new_index = list_state.state.selected()
                .map(|i| (i + 1).min(vms.len().saturating_sub(1)))
                .unwrap_or(0);
            list_state.with_selection(vms, Some(new_index));
            state.message = None;
        }
        KeyCode::Char('w') | KeyCode::Up => {
            let new_index = list_state.state.selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            list_state.with_selection(vms, Some(new_index));
            state.message = None;
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            return true;
        }
        KeyCode::Char('R') => {
            if vmrest.is_running() {
                match vmrest.stop() {
                    Ok(()) => state.set_message("✓ vmrest 服务已停止".to_string()),
                    Err(e) => state.set_message(format!("✗ {}", e)),
                }
            } else {
                match vmrest.start() {
                    Ok(()) => state.set_message("✓ vmrest 服务已启动".to_string()),
                    Err(e) => state.set_message(format!("✗ {}", e)),
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                match VmxFile::parse(&vmx_path) {
                    Ok(vmx_file) => {
                        let config = vmx_file.hardware_config();
                        let vm_name = vms.iter()
                            .find(|vm| vm.vmx_path == vmx_path)
                            .map(|vm| vm.name.clone())
                            .unwrap_or_default();
                        state.detail_state = Some(DetailState {
                            vmx_file, config, selected_row: 0,
                            dirty: false, vm_name, vmx_path,
                        });
                        state.app_mode = AppMode::Detail;
                        state.message = None;
                    }
                    Err(e) => state.set_message(format!("✗ 无法读取配置: {}", e)),
                }
            }
        }
        KeyCode::Char('n') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                let vm_name = vms.iter()
                    .find(|vm| vm.vmx_path == vmx_path)
                    .map(|vm| vm.name.clone())
                    .unwrap_or_default();
                match Vmrun::list_snapshots(&vmx_path) {
                    Ok(snapshots) => {
                        state.snapshot_state = Some(SnapshotState {
                            snapshots, selected: 0, vm_name, vmx_path,
                        });
                        state.app_mode = AppMode::Snapshot;
                        state.message = None;
                    }
                    Err(e) => state.set_message(format!("✗ 获取快照列表失败: {}", e)),
                }
            }
        }
        KeyCode::Char('c') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                let vm_name = vms.iter()
                    .find(|vm| vm.vmx_path == vmx_path)
                    .map(|vm| vm.name.clone())
                    .unwrap_or_default();
                state.clone_input = Some(CloneInputState {
                    buffer: format!("{}-clone", vm_name),
                    source_vmx: vmx_path,
                    source_name: vm_name,
                    clone_type: CloneType::Full,
                });
                state.app_mode = AppMode::CloneInput;
                state.message = None;
            }
        }
        KeyCode::Char('f') => {
            match Vmrun::list_nat_networks() {
                Ok(nets) => {
                    if let Some(network) = nets.first() {
                        let rules = Vmrun::list_port_forwardings(network).unwrap_or_default();
                        state.portfwd_state = Some(PortForwardState {
                            network: network.clone(), rules, selected: 0,
                        });
                        state.app_mode = AppMode::PortForward;
                        state.message = None;
                    } else {
                        state.set_message("✗ 未找到 NAT 网络".to_string());
                    }
                }
                Err(e) => state.set_message(format!("✗ 获取网络失败: {}", e)),
            }
        }
        KeyCode::Char('D') => {
            if let Some(ref vmx_path) = list_state.selected_vmx {
                let vm_state = vms.iter()
                    .find(|vm| &vm.vmx_path == vmx_path)
                    .map(|vm| &vm.state);
                if vm_state == Some(&VmState::Stopped) {
                    state.app_mode = AppMode::DeleteConfirm;
                    state.message = None;
                } else {
                    state.set_message("✗ 虚拟机必须停止后才能删除".to_string());
                }
            }
        }
        KeyCode::Char('h') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                let vm_name = vms.iter()
                    .find(|vm| vm.vmx_path == vmx_path)
                    .map(|vm| vm.name.clone())
                    .unwrap_or_default();
                let (folders, enabled) = parse_shared_folders_from_vmx(&vmx_path);
                state.shared_state = Some(SharedFolderState {
                    vm_name, vmx_path, folders, selected: 0, enabled,
                });
                state.app_mode = AppMode::SharedFolder;
                state.message = None;
            }
        }
        KeyCode::Char('g') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                let vm_state = vms.iter()
                    .find(|vm| vm.vmx_path == vmx_path)
                    .map(|vm| &vm.state);
                if vm_state == Some(&VmState::Running) {
                    let vm_name = vms.iter()
                        .find(|vm| vm.vmx_path == vmx_path)
                        .map(|vm| vm.name.clone())
                        .unwrap_or_default();
                    state.guest_login = Some(GuestLoginState {
                        field_index: 0,
                        fields: ["root".to_string(), String::new()],
                        vmx_path, vm_name,
                    });
                    state.app_mode = AppMode::GuestLogin;
                    state.message = None;
                } else {
                    state.set_message("✗ 虚拟机必须运行中才能操作客户文件".to_string());
                }
            }
        }
        KeyCode::Enter | KeyCode::Char('x') | KeyCode::Char('p') | KeyCode::Char('r') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                let op = match key.code {
                    KeyCode::Enter => "start",
                    KeyCode::Char('x') => "stop",
                    KeyCode::Char('p') => "suspend",
                    KeyCode::Char('r') => "resume",
                    _ => return false,
                };
                match execute_operation(manager, &vmx_path, op) {
                    Ok(msg) => state.set_message(format!("✓ {}", msg)),
                    Err(e) => state.set_message(format!("✗ {}", e)),
                }
            }
        }
        _ => {}
    }
    false
}

/// 处理删除 VM 确认事件
pub fn handle_delete_confirm_event(
    state: &mut AppState,
    key: KeyEvent,
    vms: &[Vm],
    list_state: &mut VmListState,
) -> bool {
    match key.code {
        KeyCode::Char('y') => {
            if let Some(vmx_path) = list_state.selected_vmx.clone() {
                match Vmrun::delete_vm(&vmx_path) {
                    Ok(()) => {
                        let vm_name = vms.iter()
                            .find(|vm| vm.vmx_path == vmx_path)
                            .map(|vm| vm.name.clone())
                            .unwrap_or_default();
                        state.set_message(format!("✓ 虚拟机 \"{}\" 已删除", vm_name));
                        list_state.with_selection(&[], None);
                    }
                    Err(e) => state.set_message(format!("✗ 删除失败: {}", e)),
                }
            }
            state.app_mode = AppMode::List;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.app_mode = AppMode::List;
        }
        _ => {}
    }
    false
}
