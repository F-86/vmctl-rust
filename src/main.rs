use std::env;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

mod vm;
mod vmrun;
mod manager;
mod vmx;
mod vmrest;
mod ui;
mod event;
mod app;

use manager::VmManager;
use vmrun::set_vmrun_path;
use vmrest::VmrestService;

/// 环境变量名称常量
const ENV_VMRUN_PATH: &str = "VMCTL_VMRUN_PATH";
const ENV_VM_DIR: &str = "VMCTL_VM_DIR";
const ENV_REFRESH_INTERVAL: &str = "VMCTL_REFRESH_INTERVAL";
const ENV_LOGO_PATH: &str = "VMCTL_LOGO_PATH";

/// 内置的 ASCII 艺术字（当未指定 logo 路径或读取失败时回退使用）
const DEFAULT_ASCII: &str = "__     ____  __  ____ _____ _     
\\ \\   / /  \\/  |/ ___|_   _| |    
 \\ \\ / /| |\\/| | |     | | | |    
  \\ V / | |  | | |___  | | | |___ 
   \\_/  |_|  |_|\\____| |_| |_____|
";

/// 配置结构（来源：环境变量）
struct Config {
    #[allow(dead_code)]
    vmrun_path: String,
    vm_dir: PathBuf,
    refresh_interval: u64,
}

impl Config {
    /// 从环境变量加载配置，未设置时使用默认值
    fn load() -> Self {
        // vmrun 可执行文件路径
        let vmrun_path = env::var(ENV_VMRUN_PATH)
            .unwrap_or_else(|_| {
                "/Applications/VMware Fusion.app/Contents/Library/vmrun".to_string()
            });

        // 虚拟机目录
        let vm_dir = env::var(ENV_VM_DIR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // 默认使用 $HOME/Virtual Machines.localized
                let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
                PathBuf::from(home).join("Virtual Machines.localized")
            });

        // 状态刷新间隔（秒）
        let refresh_interval = env::var(ENV_REFRESH_INTERVAL)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);

        // 将 vmrun 路径同步到全局
        set_vmrun_path(vmrun_path.clone());

        Config {
            vmrun_path,
            vm_dir,
            refresh_interval,
        }
    }
}

/// 从环境变量 `VMCTL_LOGO_PATH` 指定的文件读取 ASCII 艺术字
/// 未设置或读取失败时回退到内置 logo
fn load_ascii() -> String {
    match env::var(ENV_LOGO_PATH) {
        Ok(path) if !path.is_empty() => {
            match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("读取 logo 文件失败 ({}): {}，使用内置 logo", path, e);
                    DEFAULT_ASCII.to_string()
                }
            }
        }
        _ => DEFAULT_ASCII.to_string(),
    }
}

fn main() -> io::Result<()> {
    // 加载配置（来自环境变量）
    let config = Config::load();

    // 加载 ASCII 艺术字（来自 VMCTL_LOGO_PATH 指定的文件）
    let ascii_art = load_ascii();

    // 创建 vmrest 服务（用 Arc 共享给 manager 和 app）
    let vmrest_service = Arc::new(VmrestService::new());

    // 自动启动 vmrest 服务
    if let Err(e) = vmrest_service.start() {
        eprintln!("vmrest 自动启动失败（将回退到 vmrun）: {}", e);
    }

    // 创建 VM 管理器（传入 vmrest 引用）
    let manager = VmManager::new(config.vm_dir, Some(Arc::clone(&vmrest_service)));

    // 扫描虚拟机
    if let Err(e) = manager.scan_vms() {
        eprintln!("扫描虚拟机失败: {}", e);
    }

    // 读取每个虚拟机的 IP
    manager.refresh_ip_addresses();

    // 启动状态刷新线程
    manager.start_state_refresher(config.refresh_interval);

    // 初始化终端
    let mut terminal = ratatui::init();
    let result = app::run_app(&mut terminal, &manager, &ascii_art, &vmrest_service);
    ratatui::restore();
    result
}
