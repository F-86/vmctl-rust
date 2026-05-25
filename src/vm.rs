use std::path::PathBuf;

/// 虚拟机状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Running,
    Stopped,
    Paused,
    Unknown,
}

impl VmState {
    /// 获取状态显示文本
    pub fn display_name(&self) -> &'static str {
        match self {
            VmState::Running => "Running",
            VmState::Stopped => "Stopped",
            VmState::Paused => "Paused",
            VmState::Unknown => "Unknown",
        }
    }
}

/// 虚拟机信息结构体
#[derive(Debug, Clone)]
pub struct Vm {
    /// .vmx 文件路径
    pub vmx_path: PathBuf,
    /// 虚拟机名称（从目录名获取）
    pub name: String,
    /// 当前状态
    pub state: VmState,
    /// IP 地址
    pub ip: Option<String>,
}

impl Vm {
    /// 从 .vmx 文件路径创建 Vm 实例
    pub fn from_vmx_path(vmx_path: PathBuf) -> Self {
        let name = vmx_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .replace(".vmwarevm", "")
            .to_string();

        Vm {
            vmx_path,
            name,
            state: VmState::Unknown,
            ip: None,
        }
    }

    /// 从 .vmx 文件读取 IP 地址
    pub fn read_ip_from_vmx(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.vmx_path) {
            for line in content.lines() {
                let line = line.trim();
                // 查找 ethernet0.generatedAddress 或 ethernet0.address
                if line.starts_with("ethernet0.generatedAddress") || line.starts_with("ethernet0.address") {
                    if let Some(ip) = Self::parse_mac_to_ip(line) {
                        self.ip = Some(ip);
                        return;
                    }
                }
            }
        }
    }

    /// 从 MAC 地址生成 IP（简单映射，VMware 虚拟机通常使用 172.16 网段）
    fn parse_mac_to_ip(line: &str) -> Option<String> {
        // 格式: ethernet0.generatedAddress = "00:0c:29:xx:xx:xx"
        if let Some(eq_pos) = line.find('=') {
            let value = line[eq_pos + 1..].trim();
            let mac = value.trim_matches('"');
            let parts: Vec<&str> = mac.split(':').collect();
            // 需要 6 个字节 (00:0c:29:02:5f:34)
            if parts.len() >= 6 {
                if let (Ok(b3), Ok(b4)) = (
                    u8::from_str_radix(parts[3], 16),
                    u8::from_str_radix(parts[4], 16),
                ) {
                    // 生成 172.16.B3.B4 形式的 IP
                    return Some(format!("172.16.{}.{}", b3, b4));
                }
            }
        }
        None
    }
}