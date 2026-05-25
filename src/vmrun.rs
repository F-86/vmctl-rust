use std::path::PathBuf;
use std::process::Command;
use crate::vm::VmState;

static mut VMRUN_PATH: &str = "/Applications/VMware Fusion.app/Contents/Library/vmrun";

/// 设置 vmrun 路径（需要在启动时调用）
pub fn set_vmrun_path(path: String) {
    unsafe {
        VMRUN_PATH = Box::leak(path.into_boxed_str());
    }
}

/// 获取 vmrun 路径
fn get_vmrun_path() -> &'static str {
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
    fn execute(args: &[&str]) -> Result<String, VmrunError> {
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

    /// 获取虚拟机状态（通过 list 命令 + .vmss 文件判断）
    pub fn get_state(vmx_path: &PathBuf) -> Result<VmState, VmrunError> {
        let running_vms = Self::list_running()?;
        let is_running = running_vms.iter().any(|p| p == vmx_path);
        let is_suspended = Self::has_vmss_file(vmx_path);

        if is_running {
            if is_suspended {
                Ok(VmState::Paused)  // 运行中但有挂起文件
            } else {
                Ok(VmState::Running)
            }
        } else {
            if is_suspended {
                Ok(VmState::Paused)  // 挂起状态（不在运行但有挂起文件）
            } else {
                Ok(VmState::Stopped)
            }
        }
    }

    /// 列出虚拟机的所有快照
    pub fn list_snapshots(vmx_path: &PathBuf) -> Result<Vec<String>, VmrunError> {
        let output = Self::execute(&["listSnapshots", vmx_path.to_str().unwrap()])?;
        let mut snapshots = Vec::new();

        for line in output.lines().skip(1) {
            // 跳过首行 "Total snapshots: N"
            let name = line.trim();
            if !name.is_empty() {
                snapshots.push(name.to_string());
            }
        }

        Ok(snapshots)
    }

    /// 创建快照
    pub fn create_snapshot(vmx_path: &PathBuf, name: &str) -> Result<(), VmrunError> {
        Self::execute(&["snapshot", vmx_path.to_str().unwrap(), name])?;
        Ok(())
    }

    /// 删除快照
    pub fn delete_snapshot(vmx_path: &PathBuf, name: &str) -> Result<(), VmrunError> {
        Self::execute(&["deleteSnapshot", vmx_path.to_str().unwrap(), name])?;
        Ok(())
    }

    /// 恢复到快照
    pub fn revert_to_snapshot(vmx_path: &PathBuf, name: &str) -> Result<(), VmrunError> {
        Self::execute(&["revertToSnapshot", vmx_path.to_str().unwrap(), name])?;
        Ok(())
    }
}