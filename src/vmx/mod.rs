pub mod hardware;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use hardware::*;

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
        let value = if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
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
        let backup_path = self.path.with_extension("vmx.bak");
        std::fs::copy(&self.path, &backup_path).map_err(VmxError::BackupFailed)?;

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
}
