use std::path::{Path, PathBuf};
use super::{VmxFile, VmxError};

/// 硬件配置信息
#[derive(Debug, Clone)]
pub struct HardwareConfig {
    /// CPU 核心数
    pub cpu_count: u32,
    /// 内存大小（MB）
    pub memory_mb: u32,
    /// 网络适配器列表
    pub networks: Vec<NetworkAdapter>,
    /// 磁盘列表
    pub disks: Vec<DiskEntry>,
    /// 客户操作系统类型
    pub guest_os: String,
    /// 固件类型 (bios/efi)
    pub firmware: String,
}

/// 网络适配器信息
#[derive(Debug, Clone)]
pub struct NetworkAdapter {
    /// 适配器编号 (0, 1, 2...)
    pub index: u8,
    /// 连接类型 (nat, bridged, hostonly)
    pub connection_type: String,
    /// MAC 地址
    pub mac_address: Option<String>,
}

/// 磁盘条目
#[derive(Debug, Clone)]
pub struct DiskEntry {
    /// 控制器名称 (scsi0, sata0, nvme0)
    pub controller: String,
    /// 单元号
    pub unit: u8,
    /// VMDK 文件名
    pub file_name: String,
    /// 磁盘容量（GB）
    pub size_gb: Option<u64>,
    /// VMDK 文件完整路径（用于 vdiskmanager 扩容）
    pub full_path: PathBuf,
}

impl VmxFile {
    /// 从 VMX 键值中提取结构化的硬件配置信息
    pub fn hardware_config(&self) -> HardwareConfig {
        let cpu_count = self.get("numvcpus")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let memory_mb = self.get("memsize")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let guest_os = self.get("guestos").unwrap_or("unknown").to_string();
        let firmware = self.get("firmware").unwrap_or("bios").to_string();

        // 解析网络适配器（最多扫描 4 个）
        let mut networks = Vec::new();
        for i in 0..4 {
            let present_key = format!("ethernet{}.present", i);
            if let Some(present) = self.get(&present_key) {
                if present.to_lowercase() == "true" {
                    let conn_key = format!("ethernet{}.connectiontype", i);
                    let mac_key = format!("ethernet{}.generatedaddress", i);
                    let mac_key2 = format!("ethernet{}.address", i);

                    let connection_type = self.get(&conn_key)
                        .unwrap_or("unknown")
                        .to_string();

                    let mac_address = self.get(&mac_key)
                        .or_else(|| self.get(&mac_key2))
                        .map(|s| s.to_string());

                    networks.push(NetworkAdapter {
                        index: i as u8,
                        connection_type,
                        mac_address,
                    });
                }
            }
        }

        // 解析磁盘（扫描 scsi, sata, nvme 控制器）
        let vmx_dir = self.path.parent().unwrap_or(Path::new("."));
        let mut disks = Vec::new();
        for controller in &["scsi0", "scsi1", "sata0", "sata1", "nvme0"] {
            for unit in 0..4 {
                let present_key = format!("{}:{}.present", controller, unit);
                let file_key = format!("{}:{}.filename", controller, unit);
                if let Some(present) = self.get(&present_key) {
                    if present.to_lowercase() == "true" {
                        if let Some(file_name) = self.get(&file_key) {
                            if file_name.ends_with(".vmdk") {
                                let full_path = vmx_dir.join(file_name);
                                let size_gb = read_vmdk_size_gb(&full_path);
                                disks.push(DiskEntry {
                                    controller: controller.to_string(),
                                    unit,
                                    file_name: file_name.to_string(),
                                    size_gb,
                                    full_path,
                                });
                            }
                        }
                    }
                }
            }
        }

        HardwareConfig {
            cpu_count,
            memory_mb,
            networks,
            disks,
            guest_os,
            firmware,
        }
    }
}

/// 从 VMDK 描述文件读取磁盘容量（GB）
pub fn read_vmdk_size_gb(vmdk_path: &Path) -> Option<u64> {
    let content = std::fs::read_to_string(vmdk_path).ok()?;
    let mut total_sectors: u64 = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("RW ") || trimmed.starts_with("RDONLY "))
            && (trimmed.contains("SPARSE") || trimmed.contains("FLAT") || trimmed.contains("VMFS"))
        {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(sectors) = parts[1].parse::<u64>() {
                    total_sectors += sectors;
                }
            }
        }
    }

    if total_sectors > 0 {
        Some(total_sectors * 512 / 1024 / 1024 / 1024)
    } else {
        None
    }
}

/// 使用 vmware-vdiskmanager 扩容磁盘
pub fn expand_disk(vmdk_path: &Path, new_size_gb: u64) -> Result<String, VmxError> {
    use std::process::Command;

    let vdisk_path = "/Applications/VMware Fusion.app/Contents/Library/vmware-vdiskmanager";
    let size_arg = format!("{}GB", new_size_gb);

    let output = Command::new(vdisk_path)
        .args(["-x", &size_arg, vmdk_path.to_str().unwrap_or("")])
        .output()
        .map_err(VmxError::Io)?;

    if output.status.success() {
        Ok(format!("磁盘已扩容到 {}GB", new_size_gb))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(VmxError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("磁盘扩容失败: {}", stderr.trim()),
        )))
    }
}
