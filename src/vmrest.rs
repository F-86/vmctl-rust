use std::io::{Read, Write, BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Child};
use std::sync::{Arc, Mutex};

/// vmrest Unix Socket 路径
const VMREST_SOCKET: &str = "/tmp/vmrest.sock";

/// vmrest 服务管理器
pub struct VmrestService {
    /// vmrest 子进程
    process: Arc<Mutex<Option<Child>>>,
    /// vmrest 可执行文件路径
    vmrest_path: String,
}

impl VmrestService {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            vmrest_path: "/Applications/VMware Fusion.app/Contents/Library/vmrest".to_string(),
        }
    }

    /// 启动 vmrest 服务（Unix Socket 模式，无需凭据）
    pub fn start(&self) -> Result<(), String> {
        let mut proc = self.process.lock().unwrap();
        if proc.is_some() {
            return Err("vmrest 已在运行".to_string());
        }

        // 清理旧 socket 文件
        let _ = std::fs::remove_file(VMREST_SOCKET);

        let child = Command::new(&self.vmrest_path)
            .args(["-U", "-p", "-1"])  // Unix socket 模式，禁用 TCP
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("启动 vmrest 失败: {}", e))?;

        *proc = Some(child);

        // 等待 socket 就绪
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if std::path::Path::new(VMREST_SOCKET).exists() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// 停止 vmrest 服务
    pub fn stop(&self) -> Result<(), String> {
        let mut proc = self.process.lock().unwrap();
        if let Some(ref mut child) = *proc {
            let _ = child.kill();
            let _ = child.wait();
        }
        *proc = None;
        let _ = std::fs::remove_file(VMREST_SOCKET);
        Ok(())
    }

    /// 检查 vmrest 是否正在运行
    pub fn is_running(&self) -> bool {
        // 检查进程是否存活
        let mut proc = self.process.lock().unwrap();
        if let Some(ref mut child) = *proc {
            match child.try_wait() {
                Ok(None) => return true,  // 仍在运行
                _ => {
                    *proc = None;
                    return false;
                }
            }
        }
        false
    }

    /// 通过 vmrest API 获取所有 VM 列表
    /// 返回 Vec<(id, name, path, power_state)>
    pub fn list_vms(&self) -> Result<Vec<VmrestVm>, String> {
        let response = self.request("GET", "/api/vms")?;
        parse_vm_list(&response)
    }

    /// 通过 vmrest API 获取 VM 电源状态
    pub fn get_power_state(&self, vm_id: &str) -> Result<String, String> {
        let response = self.request("GET", &format!("/api/vms/{}/power", vm_id))?;
        // 响应格式: {"power_state":"poweredOn"}
        extract_json_value(&response, "power_state")
    }

    /// 发送 HTTP 请求到 vmrest Unix Socket
    fn request(&self, method: &str, path: &str) -> Result<String, String> {
        let mut stream = UnixStream::connect(VMREST_SOCKET)
            .map_err(|e| format!("连接 vmrest 失败: {}", e))?;

        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|e| format!("设置超时失败: {}", e))?;

        // 构建 HTTP 请求
        let request = format!(
            "{} {} HTTP/1.1\r\nHost: localhost\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            method, path
        );

        stream.write_all(request.as_bytes())
            .map_err(|e| format!("发送请求失败: {}", e))?;

        // 读取响应
        let mut reader = BufReader::new(stream);
        let mut headers = String::new();
        let mut content_length: usize = 0;

        // 读取 HTTP 头
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if line == "\r\n" {
                        break;
                    }
                    let lower = line.to_lowercase();
                    if lower.starts_with("content-length:") {
                        if let Some(val) = lower.split(':').nth(1) {
                            content_length = val.trim().parse().unwrap_or(0);
                        }
                    }
                    headers.push_str(&line);
                }
                Err(_) => break,
            }
        }

        // 检查状态码
        if !headers.starts_with("HTTP/1.1 2") && !headers.starts_with("HTTP/1.0 2") {
            return Err(format!("HTTP 错误: {}", headers.lines().next().unwrap_or("")));
        }

        // 读取 body
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body)
                .map_err(|e| format!("读取响应体失败: {}", e))?;
        }

        String::from_utf8(body).map_err(|e| format!("UTF-8 解码失败: {}", e))
    }
}

impl Drop for VmrestService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// vmrest 返回的 VM 信息
#[derive(Debug, Clone)]
pub struct VmrestVm {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

/// 简易 JSON 解析：从 VM 列表响应中提取 VM 信息
/// 响应格式: [{"id":"...","path":"..."},...]
fn parse_vm_list(json: &str) -> Result<Vec<VmrestVm>, String> {
    let mut vms = Vec::new();
    // 简易解析：按 { } 分割每个对象
    let json = json.trim();
    if !json.starts_with('[') {
        return Err("无效的 JSON 数组".to_string());
    }

    let mut i = 1; // 跳过 '['
    let bytes = json.as_bytes();

    while i < bytes.len() {
        // 找到下一个 '{'
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // 找到匹配的 '}'
        let start = i;
        let mut depth = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' { depth += 1; }
            if bytes[i] == b'}' {
                depth -= 1;
                if depth == 0 {
                    i += 1;
                    break;
                }
            }
            i += 1;
        }

        let obj = &json[start..i];
        let id = extract_json_value(obj, "id").unwrap_or_default();
        let path_str = extract_json_value(obj, "path").unwrap_or_default();

        if !id.is_empty() && !path_str.is_empty() {
            let path = PathBuf::from(&path_str);
            let name = path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .replace(".vmwarevm", "");
            vms.push(VmrestVm { id, name, path });
        }
    }

    Ok(vms)
}

/// 从 JSON 对象字符串中提取指定 key 的值（简易实现）
fn extract_json_value(json: &str, key: &str) -> Result<String, String> {
    let search = format!("\"{}\"", key);
    if let Some(pos) = json.find(&search) {
        let after_key = &json[pos + search.len()..];
        // 跳过 : 和空格
        let after_colon = after_key.trim_start().strip_prefix(':')
            .ok_or("JSON 格式错误")?
            .trim_start();

        if after_colon.starts_with('"') {
            // 字符串值
            let value_start = 1;
            let mut end = value_start;
            let chars: Vec<char> = after_colon.chars().collect();
            while end < chars.len() {
                if chars[end] == '"' && (end == 0 || chars[end - 1] != '\\') {
                    break;
                }
                end += 1;
            }
            let value: String = chars[value_start..end].iter().collect();
            Ok(value)
        } else {
            // 非字符串值（数字、bool 等）
            let end = after_colon.find(|c: char| c == ',' || c == '}' || c == ']')
                .unwrap_or(after_colon.len());
            Ok(after_colon[..end].trim().to_string())
        }
    } else {
        Err(format!("未找到 key: {}", key))
    }
}
