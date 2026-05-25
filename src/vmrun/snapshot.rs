use std::path::PathBuf;
use super::{Vmrun, VmrunError};

impl Vmrun {
    /// 列出虚拟机的所有快照
    pub fn list_snapshots(vmx_path: &PathBuf) -> Result<Vec<String>, VmrunError> {
        let output = Self::execute(&["listSnapshots", vmx_path.to_str().unwrap()])?;
        let mut snapshots = Vec::new();

        for line in output.lines().skip(1) {
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
