mod snapshot;
mod guest;
mod network;

use std::path::PathBuf;
use std::process::Command;
use crate::vm::VmState;

// 子模块通过 impl Vmrun 块扩展方法，无需 pub use
// 仅导出 PortForwarding 结构体
pub use network::PortForwarding;

static mut VMRUN_PATH: &str = "/Applications/VMware Fusion.app/Contents/Library/vmrun";

/// 设置 vmrun 路径（需要在启动时调用）
pub fn set_vmrun_path(path: String) {
    unsafe {
        VMRUN_PATH = Box::leak(path.into_boxed_str());
    }
}

/// 获取 vmrun 路径
pub(crate) fn get_vmrun_path() -> &'static str {
    unsafe { VMRUN_PATH }
}

/// vmrun 命令错误
#[derive(Debug)]
pub enum VmrunError {
    CommandFailed(String),
    NotFound,
    InvalidOutput,
}

impl std::fmt::Display for VmrunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmrunError::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
            VmrunError::NotFound => write!(f, "vmrun not found"),
            VmrunError::InvalidOutput => write!(f, "Invalid output from vmrun"),
        }
    }
}

impl std::error::Error for VmrunError {}

/// 封装 vmrun 命令调用
pub struct Vmrun;

impl Vmrun {
    /// 执行 vmrun 命令
    pub(crate) fn execute(args: &[&str]) -> Result<String, VmrunError> {
        let output = Command::new(get_vmrun_path())
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    VmrunError::NotFound
                } else {
                    VmrunError::CommandFailed(e.to_string())
                }
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(VmrunError::CommandFailed(stderr.to_string()))
        }
    }

    /// 获取所有正在运行的虚拟机路径列表
    pub fn list_running() -> Result<Vec<PathBuf>, VmrunError> {
        let output = Self::execute(&["list"])?;
        let mut vms = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if !line.is_empty() && line != "Total running VMs: 0" && line != "Total running VMs:" {
                if !line.starts_with("Total running VMs:") {
                    let path = PathBuf::from(line);
                    if path.exists() {
                        vms.push(path);
                    }
                }
            }
        }

        Ok(vms)
    }

    /// 启动虚拟机（无界面模式）
    pub fn start(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["start", vmx_path.to_str().unwrap(), "nogui"])?;
        Ok(())
    }

    /// 停止虚拟机（软停止）
    pub fn stop(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["stop", vmx_path.to_str().unwrap()])?;
        Ok(())
    }

    /// 挂起虚拟机
    pub fn suspend(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["suspend", vmx_path.to_str().unwrap()])?;
        Ok(())
    }

    /// 继续运行（恢复挂起的虚拟机）
    pub fn resume(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["start", vmx_path.to_str().unwrap()])?;
        Ok(())
    }

    /// 检查目录下是否有 .vmss 挂起文件
    fn has_vmss_file(vmx_path: &PathBuf) -> bool {
        if let Some(parent) = vmx_path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let filename = entry.file_name();
                    if let Some(filename_str) = filename.to_str() {
                        if filename_str.ends_with(".vmss") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// 获取虚拟机状态
    pub fn get_state(vmx_path: &PathBuf) -> Result<VmState, VmrunError> {
        let running_vms = Self::list_running()?;
        let is_running = running_vms.iter().any(|p| p == vmx_path);
        let is_suspended = Self::has_vmss_file(vmx_path);

        if is_running {
            if is_suspended {
                Ok(VmState::Paused)
            } else {
                Ok(VmState::Running)
            }
        } else {
            if is_suspended {
                Ok(VmState::Paused)
            } else {
                Ok(VmState::Stopped)
            }
        }
    }

    /// 获取客户操作系统的 IP 地址
    pub fn get_guest_ip(vmx_path: &PathBuf) -> Result<String, VmrunError> {
        let output = Self::execute(&["getGuestIPAddress", vmx_path.to_str().unwrap()])?;
        let ip = output.trim().to_string();
        if ip.is_empty() {
            Err(VmrunError::CommandFailed("未获取到 IP 地址".to_string()))
        } else {
            Ok(ip)
        }
    }

    /// 删除虚拟机（不可恢复）
    pub fn delete_vm(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["deleteVM", vmx_path.to_str().unwrap()])?;
        Ok(())
    }
}
