use std::path::PathBuf;
use std::process::Command;
use super::{Vmrun, VmrunError, get_vmrun_path};

impl Vmrun {
    /// 带凭据执行 vmrun 命令（用于客户系统操作）
    fn execute_guest(_vmx_path: &PathBuf, user: &str, pass: &str, args: &[&str]) -> Result<String, VmrunError> {
        let mut cmd_args: Vec<&str> = vec![
            "-gu", user,
            "-gp", pass,
        ];
        cmd_args.extend_from_slice(args);

        let output = Command::new(get_vmrun_path())
            .args(&cmd_args)
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

    /// 列出客户系统中的目录内容
    pub fn list_directory_in_guest(vmx_path: &PathBuf, user: &str, pass: &str, dir: &str) -> Result<Vec<String>, VmrunError> {
        let output = Self::execute_guest(vmx_path, user, pass, &[
            "listDirectoryInGuest", vmx_path.to_str().unwrap(), dir,
        ])?;
        let mut entries = Vec::new();
        for line in output.lines() {
            let name = line.trim();
            if !name.is_empty() && !name.starts_with("Directory list:") {
                entries.push(name.to_string());
            }
        }
        Ok(entries)
    }

    /// 从宿主复制文件到客户系统
    pub fn copy_file_to_guest(vmx_path: &PathBuf, user: &str, pass: &str, host_path: &str, guest_path: &str) -> Result<(), VmrunError> {
        Self::execute_guest(vmx_path, user, pass, &[
            "CopyFileFromHostToGuest", vmx_path.to_str().unwrap(), host_path, guest_path,
        ])?;
        Ok(())
    }

    /// 从客户系统复制文件到宿主
    pub fn copy_file_from_guest(vmx_path: &PathBuf, user: &str, pass: &str, guest_path: &str, host_path: &str) -> Result<(), VmrunError> {
        Self::execute_guest(vmx_path, user, pass, &[
            "CopyFileFromGuestToHost", vmx_path.to_str().unwrap(), guest_path, host_path,
        ])?;
        Ok(())
    }

    /// 在客户系统中创建目录
    pub fn create_directory_in_guest(vmx_path: &PathBuf, user: &str, pass: &str, dir: &str) -> Result<(), VmrunError> {
        Self::execute_guest(vmx_path, user, pass, &[
            "createDirectoryInGuest", vmx_path.to_str().unwrap(), dir,
        ])?;
        Ok(())
    }

    /// 在客户系统中删除文件
    pub fn delete_file_in_guest(vmx_path: &PathBuf, user: &str, pass: &str, path: &str) -> Result<(), VmrunError> {
        Self::execute_guest(vmx_path, user, pass, &[
            "deleteFileInGuest", vmx_path.to_str().unwrap(), path,
        ])?;
        Ok(())
    }
}
