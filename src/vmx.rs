use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// VMX 文件解析错误
#[derive(Debug)]
pub enum VmxError {
    /// IO 错误
    Io(std::io::Error),
    /// VM 未停止，不能修改配置
    VmNotStopped,
    /// 备份失败
    BackupFailed(std::io::Error),
}

impl std::fmt::Display for VmxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmxError::Io(e) => write!(f, "IO 错误: {}", e),
            VmxError::VmNotStopped => write!(f, "虚拟机必须停止后才能修改配置"),
            VmxError::BackupFailed(e) => write!(f, "备份失败: {}", e),
        }
    }
}

impl From<std::io::Error> for VmxError {
    fn from(e: std::io::Error) -> Self {
        VmxError::Io(e)
    }
}

/// VMX 文件中的一行条目
#[derive(Debug, Clone)]
enum VmxEntry {
    /// 键值对: key = "value"
    KeyValue { key: String, value: String },
    /// 注释行或空行（原样保留）
    Other(String),
}

/// 表示一个解析后的 .vmx 文件
/// 保留原始文件的顺序和注释，确保写回时不丢失任何内容
#[derive(Debug, Clone)]
pub struct VmxFile {
    /// 按顺序存储的所有条目
    entries: Vec<VmxEntry>,
    /// 快速查找索引: key(小写) -> entries 中的位置
    index: HashMap<String, usize>,
    /// 文件路径
    pub path: PathBuf,
}

impl VmxFile {
    /// 从磁盘解析 .vmx 文件
    pub fn parse(path: &Path) -> Result<Self, VmxError> {
        let content = std::fs::read_to_string(path)?;
        let mut entries = Vec::new();
        let mut index = HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                entries.push(VmxEntry::Other(line.to_string()));
            } else if let Some((key, value)) = Self::parse_line(trimmed) {
                let idx = entries.len();
                index.insert(key.clone(), idx);
                entries.push(VmxEntry::KeyValue { key, value });
            } else {
                // 无法解析的行，原样保留
                entries.push(VmxEntry::Other(line.to_string()));
            }
        }

        Ok(Self {
            entries,
            index,
            path: path.to_path_buf(),
        })
    }

    /// 解析单行 "key = value" 或 "key = \"value\""
    fn parse_line(line: &str) -> Option<(String, String)> {
        let eq_pos = line.find('=')?;
        let key = line[..eq_pos].trim().to_lowercase();
        if key.is_empty() {
            return None;
        }
        let raw_value = line[eq_pos + 1..].trim();
        // 去除引号
        let value = if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2
        {
            raw_value[1..raw_value.len() - 1].to_string()
        } else {
            raw_value.to_string()
        };
        Some((key, value))
    }

    /// 获取指定 key 的值（key 不区分大小写）
    pub fn get(&self, key: &str) -> Option<&str> {
        let idx = self.index.get(&key.to_lowercase())?;
        match &self.entries[*idx] {
            VmxEntry::KeyValue { value, .. } => Some(value.as_str()),
            _ => None,
        }
    }

    /// 设置指定 key 的值（已存在则更新，不存在则追加）
    pub fn set(&mut self, key: &str, value: &str) {
        let lower_key = key.to_lowercase();
        if let Some(&idx) = self.index.get(&lower_key) {
            self.entries[idx] = VmxEntry::KeyValue {
                key: lower_key,
                value: value.to_string(),
            };
        } else {
            let idx = self.entries.len();
            self.index.insert(lower_key.clone(), idx);
            self.entries.push(VmxEntry::KeyValue {
                key: lower_key,
                value: value.to_string(),
            });
        }
    }

    /// 备份原文件并写入修改后的内容
    pub fn backup_and_write(&self) -> Result<(), VmxError> {
        // 创建备份
        let backup_path = self.path.with_extension("vmx.bak");
        std::fs::copy(&self.path, &backup_path).map_err(VmxError::BackupFailed)?;

        // 写入临时文件后 rename（原子写入）
        let tmp_path = self.path.with_extension("vmx.tmp");
        self.write_to(&tmp_path)?;
        std::fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }

    /// 将内容写入指定路径
    fn write_to(&self, path: &Path) -> Result<(), VmxError> {
        let mut output = String::new();
        for entry in &self.entries {
            match entry {
                VmxEntry::KeyValue { key, value } => {
                    output.push_str(&format!("{} = \"{}\"\n", key, value));
                }
                VmxEntry::Other(text) => {
                    output.push_str(text);
                    output.push('\n');
                }
            }
        }
        std::fs::write(path, output)?;
        Ok(())
    }

    /// 从 VMX 键值中提取结构化的硬件配置信息
    pub fn hardware_config(&self) -> HardwareConfig {
        let cpu_count = self
            .get("numvcpus")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let memory_mb = self
            .get("memsize")
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

                    let connection_type = self
                        .get(&conn_key)
                        .unwrap_or("unknown")
                        .to_string();

                    let mac_address = self
                        .get(&mac_key)
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
                            // 过滤掉非 vmdk 文件（如 auto detect cdrom）
                            if file_name.ends_with(".vmdk") {
                                let full_path = vmx_dir.join(file_name);
                                let size_gb = Self::read_vmdk_size_gb(&full_path);
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

    /// 从 VMDK 描述文件读取磁盘容量（GB）
    /// VMDK 文本描述文件中包含 createType 和 extent description 行
    /// 格式如: RW 83886080 VMFSSPARSE "disk-s001.vmdk"
    /// 数字是扇区数（每扇区 512 字节）
    fn read_vmdk_size_gb(vmdk_path: &Path) -> Option<u64> {
        let content = std::fs::read_to_string(vmdk_path).ok()?;
        let mut total_sectors: u64 = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            // 匹配 extent description 行: RW/RDONLY <sectors> <type> "<file>"
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
            // 扇区 -> GB (512 bytes/sector)
            Some(total_sectors * 512 / 1024 / 1024 / 1024)
        } else {
            None
        }
    }
}

/// 使用 vmware-vdiskmanager 扩容磁盘
/// new_size_gb: 新的磁盘大小（GB），必须大于当前大小
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
