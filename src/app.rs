use std::io;
use std::time::Duration;
use std::process::Command;
use std::sync::Arc;

use crossterm::event::{self, Event, KeyEventKind};
use sysinfo::System;

use crate::manager::VmManager;
use crate::vmrest::VmrestService;
use crate::ui::state::*;
use crate::ui::vm_list::ui;
use crate::ui::detail::{render_detail_view};
use crate::ui::edit::{render_edit_overlay, render_confirm_dialog};
use crate::ui::snapshot::{render_snapshot_view, render_snapshot_input, render_snapshot_confirm};
use crate::ui::clone::render_clone_input;
use crate::ui::portfwd::{render_portfwd_view, render_portfwd_input, render_portfwd_delete_confirm};
use crate::ui::shared::{render_shared_folder_view, render_shared_folder_input, render_shared_folder_delete_confirm};
use crate::ui::guest::{render_guest_login, render_guest_files_view, render_guest_file_input, render_guest_file_delete_confirm};
use crate::ui::delete::render_delete_vm_confirm;
use crate::ui::ssh::render_ssh_login;
use crate::event::{AppState, handle_event};

/// 应用主循环
pub fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    manager: &VmManager,
    ascii_art: &str,
    vmrest: &Arc<VmrestService>,
) -> io::Result<()> {
    let mut state = AppState::new();
    let mut list_state = VmListState::new();
    let mut sys = System::new_all();
    let mut cpu_usage: f32 = 0.0;
    let mut mem_usage: f32 = 0.0;
    let mut last_sys_refresh = std::time::Instant::now() - Duration::from_secs(2);

    loop {
        // 获取当前虚拟机列表
        let vms = manager.get_vms();
        let vm_count = vms.len();

        // 确保选中项有效
        if let Some(selected) = list_state.selected_vmx.clone() {
            if !vms.iter().any(|vm| vm.vmx_path == selected) {
                list_state.with_selection(&vms, None);
            }
        }

        // 每秒刷新一次系统资源信息
        if last_sys_refresh.elapsed() >= Duration::from_secs(1) {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            cpu_usage = sys.global_cpu_usage();
            let used_mem = sys.used_memory() as f64;
            let total_mem = sys.total_memory() as f64;
            mem_usage = (used_mem / total_mem * 100.0) as f32;
            last_sys_refresh = std::time::Instant::now();
        }

        // vmrest 状态
        let vmrest_running = vmrest.is_running();

        // 渲染 UI
        terminal.draw(|frame| {
            match state.app_mode {
                AppMode::List => {
                    ui(frame, &vms, &mut list_state, &state.message, vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);
                }
                AppMode::Detail | AppMode::Confirm => {
                    if let Some(ref ds) = state.detail_state {
                        render_detail_view(frame, ds, &state.message, ascii_art);
                        if state.app_mode == AppMode::Confirm {
                            render_confirm_dialog(frame);
                        }
                    }
                }
                AppMode::Editing => {
                    if let Some(ref ds) = state.detail_state {
                        render_detail_view(frame, ds, &state.message, ascii_art);
                    }
                    if let Some(ref es) = state.edit_state {
                        render_edit_overlay(frame, es);
                    }
                }
                AppMode::Snapshot => {
                    if let Some(ref ss) = state.snapshot_state {
                        render_snapshot_view(frame, ss, &state.message, ascii_art);
                    }
                }
                AppMode::SnapshotInput => {
                    if let Some(ref ss) = state.snapshot_state {
                        render_snapshot_view(frame, ss, &state.message, ascii_art);
                    }
                    if let Some(ref sis) = state.snapshot_input {
                        render_snapshot_input(frame, sis);
                    }
                }
                AppMode::SnapshotConfirm => {
                    if let Some(ref ss) = state.snapshot_state {
                        render_snapshot_view(frame, ss, &state.message, ascii_art);
                    }
                    if let Some(ref action) = state.snapshot_action {
                        render_snapshot_confirm(frame, action);
                    }
                }
                AppMode::CloneInput => {
                    ui(frame, &vms, &mut list_state, &state.message, vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);
                    if let Some(ref ci) = state.clone_input {
                        render_clone_input(frame, ci);
                    }
                }
                AppMode::PortForward => {
                    if let Some(ref pfs) = state.portfwd_state {
                        render_portfwd_view(frame, pfs, &state.message, ascii_art);
                    }
                }
                AppMode::PortForwardInput => {
                    if let Some(ref pfs) = state.portfwd_state {
                        render_portfwd_view(frame, pfs, &state.message, ascii_art);
                    }
                    if let Some(ref pfi) = state.portfwd_input {
                        render_portfwd_input(frame, pfi);
                    }
                }
                AppMode::PortForwardConfirm => {
                    if let Some(ref pfs) = state.portfwd_state {
                        render_portfwd_view(frame, pfs, &state.message, ascii_art);
                        render_portfwd_delete_confirm(frame, pfs);
                    }
                }
                AppMode::DeleteConfirm => {
                    ui(frame, &vms, &mut list_state, &state.message, vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);
                    render_delete_vm_confirm(frame, &vms, &list_state);
                }
                AppMode::SharedFolder => {
                    if let Some(ref ss) = state.shared_state {
                        render_shared_folder_view(frame, ss, &state.message, ascii_art);
                    }
                }
                AppMode::SharedFolderInput => {
                    if let Some(ref ss) = state.shared_state {
                        render_shared_folder_view(frame, ss, &state.message, ascii_art);
                    }
                    if let Some(ref si) = state.shared_input {
                        render_shared_folder_input(frame, si);
                    }
                }
                AppMode::SharedFolderConfirm => {
                    if let Some(ref ss) = state.shared_state {
                        render_shared_folder_view(frame, ss, &state.message, ascii_art);
                        render_shared_folder_delete_confirm(frame, ss);
                    }
                }
                AppMode::GuestLogin => {
                    ui(frame, &vms, &mut list_state, &state.message, vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);
                    if let Some(ref gl) = state.guest_login {
                        render_guest_login(frame, gl);
                    }
                }
                AppMode::GuestFiles => {
                    if let Some(ref gs) = state.guest_state {
                        render_guest_files_view(frame, gs, &state.message, ascii_art);
                    }
                }
                AppMode::GuestFileInput => {
                    if let Some(ref gs) = state.guest_state {
                        render_guest_files_view(frame, gs, &state.message, ascii_art);
                    }
                    if let Some(ref gfi) = state.guest_file_input {
                        render_guest_file_input(frame, gfi);
                    }
                }
                AppMode::GuestFileConfirm => {
                    if let Some(ref gs) = state.guest_state {
                        render_guest_files_view(frame, gs, &state.message, ascii_art);
                        render_guest_file_delete_confirm(frame, gs);
                    }
                }
                AppMode::SshLogin => {
                    ui(frame, &vms, &mut list_state, &state.message, vm_count, ascii_art, cpu_usage, mem_usage, vmrest_running);
                    if let Some(ref sl) = state.ssh_login {
                        render_ssh_login(frame, sl);
                    }
                }
            }
        })?;

        // 事件处理
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let should_quit = handle_event(
                    &mut state, key, &vms, &mut list_state, manager, vmrest,
                );
                if should_quit {
                    return Ok(());
                }
            }
        }

        // 检查是否需要执行 SSH
        if let Some(ssh_req) = state.ssh_exec.take() {
            // 暂时退出 TUI，恢复终端
            ratatui::restore();

            // 执行 SSH 命令
            let status = Command::new("ssh")
                .arg("-p")
                .arg(&ssh_req.port)
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg(format!("{}@{}", ssh_req.user, ssh_req.ip))
                .status();

            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        state.set_message("✓ SSH 会话已结束".to_string());
                    } else {
                        state.set_message(format!(
                            "✗ SSH 退出码: {}",
                            exit_status.code().unwrap_or(-1)
                        ));
                    }
                }
                Err(e) => {
                    state.set_message(format!("✗ SSH 启动失败: {}", e));
                }
            }

            // 重新初始化终端
            *terminal = ratatui::init();
        }

        // 清除消息（3 秒后）
        if let Some(timer) = state.message_timer {
            if timer.elapsed() > Duration::from_secs(3) {
                state.message = None;
                state.message_timer = None;
            }
        }
    }
}
