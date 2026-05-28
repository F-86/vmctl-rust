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

/// 配置文件结构
struct Config {
    vmrun_path: String,
    vm_dir: PathBuf,
    refresh_interval: u64,
}

impl Config {
    fn load() -> Self {
        let mut vmrun_path = "/Applications/VMware Fusion.app/Contents/Library/vmrun".to_string();
        let mut vm_dir = PathBuf::from("/Users/jane/Virtual Machines.localized");
        let mut refresh_interval = 5u64;

        if let Ok(content) = std::fs::read_to_string("config.toml") {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim();
                    let value = line[eq_pos + 1..].trim();
                    match key {
                        "vmrun_path" => vmrun_path = value.to_string(),
                        "vm_dir" => vm_dir = PathBuf::from(value),
                        "refresh_interval" => {
                            if let Ok(interval) = value.parse() {
                                refresh_interval = interval;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        set_vmrun_path(vmrun_path.clone());

        Config {
            vmrun_path,
            vm_dir,
            refresh_interval,
        }
    }
}

/// 读取 ASCII 艺术字
fn load_ascii() -> String {
    if let Ok(content) = std::fs::read_to_string("ascii.txt") {
        content
    } else {
        "__     ____  __  ____ _____ _     
\\ \\   / /  \\/  |/ ___|_   _| |    
 \\ \\ / /| |\\/| | |     | | | |    
  \\ V / | |  | | |___  | | | |___ 
   \\_/  |_|  |_|\\____| |_| |_____|
"
        .to_string()
    }
}

fn main() -> io::Result<()> {
    // 加载配置
    let config = Config::load();

    // 加载 ASCII 艺术字
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
