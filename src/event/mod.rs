pub mod list;
pub mod detail;
pub mod snapshot;
pub mod clone;
pub mod portfwd;
pub mod shared;
pub mod guest;
pub mod ssh;

use crossterm::event::KeyEvent;
use crate::vm::Vm;
use crate::manager::VmManager;
use crate::vmrest::VmrestService;
use crate::ui::state::*;

/// 应用可变状态（集中管理）
pub struct AppState {
    pub app_mode: AppMode,
    pub detail_state: Option<DetailState>,
    pub edit_state: Option<EditState>,
    pub snapshot_state: Option<SnapshotState>,
    pub snapshot_input: Option<SnapshotInputState>,
    pub snapshot_action: Option<SnapshotAction>,
    pub clone_input: Option<CloneInputState>,
    pub portfwd_state: Option<PortForwardState>,
    pub portfwd_input: Option<PortForwardInputState>,
    pub shared_state: Option<SharedFolderState>,
    pub shared_input: Option<SharedFolderInputState>,
    pub guest_state: Option<GuestFilesState>,
    pub guest_login: Option<GuestLoginState>,
    pub guest_file_input: Option<GuestFileInputState>,
    pub ssh_login: Option<SshLoginState>,
    /// SSH 执行请求：当设置为 Some 时，主循环会执行 SSH
    pub ssh_exec: Option<SshExecRequest>,
    pub message: Option<String>,
    pub message_timer: Option<std::time::Instant>,
}

/// SSH 执行请求
pub struct SshExecRequest {
    pub user: String,
    pub ip: String,
    pub port: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            app_mode: AppMode::List,
            detail_state: None,
            edit_state: None,
            snapshot_state: None,
            snapshot_input: None,
            snapshot_action: None,
            clone_input: None,
            portfwd_state: None,
            portfwd_input: None,
            shared_state: None,
            shared_input: None,
            guest_state: None,
            guest_login: None,
            guest_file_input: None,
            ssh_login: None,
            ssh_exec: None,
            message: None,
            message_timer: None,
        }
    }

    /// 设置消息并启动计时器
    pub fn set_message(&mut self, msg: String) {
        self.message = Some(msg);
        self.message_timer = Some(std::time::Instant::now());
    }
}

/// 事件分发函数，根据当前模式分发到对应的处理函数
/// 返回 true 表示应退出程序
pub fn handle_event(
    state: &mut AppState,
    key: KeyEvent,
    vms: &[Vm],
    list_state: &mut VmListState,
    manager: &VmManager,
    vmrest: &VmrestService,
) -> bool {
    match state.app_mode {
        AppMode::List => list::handle_list_event(state, key, vms, list_state, manager, vmrest),
        AppMode::Detail => detail::handle_detail_event(state, key, vms),
        AppMode::Editing => detail::handle_editing_event(state, key),
        AppMode::Confirm => detail::handle_confirm_event(state, key),
        AppMode::Snapshot => snapshot::handle_snapshot_event(state, key),
        AppMode::SnapshotInput => snapshot::handle_snapshot_input_event(state, key),
        AppMode::SnapshotConfirm => snapshot::handle_snapshot_confirm_event(state, key),
        AppMode::CloneInput => clone::handle_clone_input_event(state, key),
        AppMode::PortForward => portfwd::handle_portfwd_event(state, key),
        AppMode::PortForwardInput => portfwd::handle_portfwd_input_event(state, key),
        AppMode::PortForwardConfirm => portfwd::handle_portfwd_confirm_event(state, key),
        AppMode::DeleteConfirm => list::handle_delete_confirm_event(state, key, vms, list_state),
        AppMode::SharedFolder => shared::handle_shared_event(state, key),
        AppMode::SharedFolderInput => shared::handle_shared_input_event(state, key),
        AppMode::SharedFolderConfirm => shared::handle_shared_confirm_event(state, key),
        AppMode::GuestLogin => guest::handle_guest_login_event(state, key),
        AppMode::GuestFiles => guest::handle_guest_files_event(state, key),
        AppMode::GuestFileInput => guest::handle_guest_file_input_event(state, key),
        AppMode::GuestFileConfirm => guest::handle_guest_file_confirm_event(state, key),
        AppMode::SshLogin => ssh::handle_ssh_login_event(state, key),
    }
}
