use std::path::PathBuf;
use super::{Vmrun, VmrunError};

/// 端口转发规则
#[derive(Debug, Clone)]
pub struct PortForwarding {
    /// 协议 (tcp/udp)
    pub protocol: String,
    /// 宿主端口
    pub host_port: u16,
    /// 客户 IP
    pub guest_ip: String,
    /// 客户端口
    pub guest_port: u16,
    /// 描述
    pub description: String,
}

impl PortForwarding {
    /// 解析 vmrun listPortForwardings 输出的一行
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if !line.starts_with('[') {
            return None;
        }
        let bracket_end = line.find(']')?;
        let protocol = line[1..bracket_end].to_string();
        let rest = line[bracket_end + 1..].trim();

        let parts: Vec<&str> = rest.splitn(4, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let host_port: u16 = parts[0].parse().ok()?;
        let guest_parts: Vec<&str> = parts[2].splitn(2, ':').collect();
        if guest_parts.len() < 2 {
            return None;
        }
        let guest_ip = guest_parts[0].to_string();
        let guest_port: u16 = guest_parts[1].parse().ok()?;
        let description = if parts.len() >= 4 {
            parts[3].to_string()
        } else {
            String::new()
        };

        Some(PortForwarding {
            protocol,
            host_port,
            guest_ip,
            guest_port,
            description,
        })
    }
}

impl Vmrun {
    /// 列出宿主网络，返回 NAT 类型的网络名称
    pub fn list_nat_networks() -> Result<Vec<String>, VmrunError> {
        let output = Self::execute(&["listHostNetworks"])?;
        let mut nat_nets = Vec::new();
        for line in output.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2].to_lowercase() == "nat" {
                nat_nets.push(parts[1].to_string());
            }
        }
        Ok(nat_nets)
    }

    /// 列出指定网络的端口转发规则
    pub fn list_port_forwardings(network: &str) -> Result<Vec<PortForwarding>, VmrunError> {
        let output = Self::execute(&["listPortForwardings", network])?;
        let mut rules = Vec::new();
        for line in output.lines().skip(1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rule) = PortForwarding::parse(trimmed) {
                rules.push(rule);
            }
        }
        Ok(rules)
    }

    /// 添加端口转发规则
    pub fn set_port_forwarding(
        network: &str,
        protocol: &str,
        host_port: u16,
        guest_ip: &str,
        guest_port: u16,
        description: &str,
    ) -> Result<(), VmrunError> {
        Self::execute(&[
            "setPortForwarding",
            network,
            protocol,
            &host_port.to_string(),
            guest_ip,
            &guest_port.to_string(),
            description,
        ])?;
        Ok(())
    }

    /// 删除端口转发规则
    pub fn delete_port_forwarding(
        network: &str,
        protocol: &str,
        host_port: u16,
    ) -> Result<(), VmrunError> {
        Self::execute(&[
            "deletePortForwarding",
            network,
            protocol,
            &host_port.to_string(),
        ])?;
        Ok(())
    }

    /// 启用共享文件夹
    pub fn enable_shared_folders(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["enableSharedFolders", vmx_path.to_str().unwrap(), "runtime"])?;
        Ok(())
    }

    /// 禁用共享文件夹
    pub fn disable_shared_folders(vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Self::execute(&["disableSharedFolders", vmx_path.to_str().unwrap(), "runtime"])?;
        Ok(())
    }

    /// 添加共享文件夹
    pub fn add_shared_folder(vmx_path: &PathBuf, name: &str, host_path: &str) -> Result<(), VmrunError> {
        Self::execute(&["addSharedFolder", vmx_path.to_str().unwrap(), name, host_path])?;
        Ok(())
    }

    /// 移除共享文件夹
    pub fn remove_shared_folder(vmx_path: &PathBuf, name: &str) -> Result<(), VmrunError> {
        Self::execute(&["removeSharedFolder", vmx_path.to_str().unwrap(), name])?;
        Ok(())
    }

    /// 设置共享文件夹状态（可读写/只读）
    pub fn set_shared_folder_state(vmx_path: &PathBuf, name: &str, host_path: &str, writable: bool) -> Result<(), VmrunError> {
        let mode = if writable { "writable" } else { "readonly" };
        Self::execute(&["setSharedFolderState", vmx_path.to_str().unwrap(), name, host_path, mode])?;
        Ok(())
    }

    /// 克隆虚拟机
    pub fn clone_vm(source_vmx: &PathBuf, dest_vmx: &PathBuf, clone_type: &str, clone_name: &str) -> Result<(), VmrunError> {
        Self::execute(&[
            "clone",
            source_vmx.to_str().unwrap(),
            dest_vmx.to_str().unwrap(),
            clone_type,
            &format!("-cloneName={}", clone_name),
        ])?;
        Ok(())
    }
}
