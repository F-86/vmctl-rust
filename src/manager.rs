use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::vm::{Vm, VmState};
use crate::vmrun::{Vmrun, VmrunError};
use crate::vmrest::{VmrestService, vmrest_power_to_state};

/// 虚拟机管理器
pub struct VmManager {
    /// 虚拟机列表
    vms: Arc<Mutex<Vec<Vm>>>,
    /// 扫描目录
    scan_dir: PathBuf,
    /// vmrest 服务引用（可选）
    vmrest: Option<Arc<VmrestService>>,
}

impl VmManager {
    /// 创建新的 VM 管理器
    pub fn new(scan_dir: PathBuf, vmrest: Option<Arc<VmrestService>>) -> Self {
        VmManager {
            vms: Arc::new(Mutex::new(Vec::new())),
            scan_dir,
            vmrest,
        }
    }

    /// 扫描目录获取所有 .vmx 文件
    pub fn scan_vms(&self) -> Result<(), VmrunError> {
        let mut vms = self.vms.lock().unwrap();
        vms.clear();

        if !self.scan_dir.exists() {
            return Ok(());
        }

        // 遍历目录查找 .vmwarevm 包
        let entries = match std::fs::read_dir(&self.scan_dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // 在 .vmwarevm 目录中查找 .vmx 文件
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.ends_with(".vmwarevm") {
                        // 查找 .vmx 文件
                        if let Ok(vmx_entries) = std::fs::read_dir(&path) {
                            for vmx_entry in vmx_entries.flatten() {
                                let vmx_path = vmx_entry.path();
                                if vmx_path.extension().and_then(|e| e.to_str()) == Some("vmx") {
                                    let mut vm = Vm::from_vmx_path(vmx_path);
                                    // 立即获取状态
                                    if let Ok(state) = Vmrun::get_state(&vm.vmx_path) {
                                        vm.state = state;
                                    }
                                    vms.push(vm);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按虚拟机名称排序
        vms.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(())
    }

    /// 获取虚拟机列表
    pub fn get_vms(&self) -> Vec<Vm> {
        self.vms.lock().unwrap().clone()
    }

    /// 刷新所有虚拟机的 IP 地址
    pub fn refresh_ip_addresses(&self) {
        let mut vms = self.vms.lock().unwrap();
        for vm in vms.iter_mut() {
            vm.refresh_ip();
        }
    }

    /// 根据 vmx_path 查找虚拟机索引
    pub fn find_vm_index(&self, vmx_path: &Path) -> Option<usize> {
        let vms = self.vms.lock().unwrap();
        vms.iter().position(|vm| &vm.vmx_path == vmx_path)
    }

    /// 启动虚拟机
    pub fn start_vm(&self, vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Vmrun::start(vmx_path)?;
        self.refresh_vm_state(vmx_path);
        Ok(())
    }

    /// 停止虚拟机
    pub fn stop_vm(&self, vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Vmrun::stop(vmx_path)?;
        self.refresh_vm_state(vmx_path);
        Ok(())
    }

    /// 挂起虚拟机
    pub fn suspend_vm(&self, vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Vmrun::suspend(vmx_path)?;
        self.refresh_vm_state(vmx_path);
        Ok(())
    }

    /// 继续运行（恢复挂起的虚拟机）
    pub fn resume_vm(&self, vmx_path: &PathBuf) -> Result<(), VmrunError> {
        Vmrun::resume(vmx_path)?;
        self.refresh_vm_state(vmx_path);
        Ok(())
    }

    /// 刷新单个虚拟机状态
    fn refresh_vm_state(&self, vmx_path: &Path) {
        // 将 Path 转换为 PathBuf
        let path_buf = vmx_path.to_path_buf();
        if let Ok(state) = Vmrun::get_state(&path_buf) {
            let mut vms = self.vms.lock().unwrap();
            if let Some(vm) = vms.iter_mut().find(|vm| &vm.vmx_path == vmx_path) {
                vm.state = state;
            }
        }
    }

    /// 刷新所有虚拟机状态
    pub fn refresh_all_states(&self) {
        let vms: Vec<PathBuf> = self.vms.lock().unwrap()
            .iter()
            .map(|vm| vm.vmx_path.clone())
            .collect();

        for vmx_path in vms {
            self.refresh_vm_state(&vmx_path);
        }
    }

    /// 启动后台状态刷新线程
    pub fn start_state_refresher(&self, interval_secs: u64) {
        let vms = Arc::clone(&self.vms);
        let scan_dir = self.scan_dir.clone();
        let vmrest = self.vmrest.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(interval_secs));

                // 优先使用 vmrest REST API 刷新
                let used_vmrest = if let Some(ref vmrest_svc) = vmrest {
                    if vmrest_svc.is_running() {
                        Self::refresh_via_vmrest(vmrest_svc, &vms, &scan_dir)
                    } else {
                        false
                    }
                } else {
                    false
                };

                // vmrest 不可用时，回退到 vmrun 方式
                if !used_vmrest {
                    Self::refresh_via_vmrun(&vms, &scan_dir);
                }
            }
        });
    }

    /// 通过 vmrest REST API 刷新虚拟机列表和状态
    fn refresh_via_vmrest(
        vmrest_svc: &VmrestService,
        vms: &Arc<Mutex<Vec<Vm>>>,
        scan_dir: &Path,
    ) -> bool {
        // 尝试通过 REST API 获取 VM 列表
        let rest_vms = match vmrest_svc.list_vms() {
            Ok(v) => v,
            Err(_) => return false,
        };

        // 保留现有 IP 信息
        let existing_ips: HashMap<PathBuf, Option<String>> = {
            let vm_list = vms.lock().unwrap();
            vm_list.iter()
                .map(|vm| (vm.vmx_path.clone(), vm.ip.clone()))
                .collect()
        };

        // 构建新的 VM 列表
        let mut new_vms: Vec<Vm> = rest_vms.iter()
            .filter(|rvm| rvm.path.starts_with(scan_dir))
            .map(|rvm| {
                let state = rvm.power_state.as_deref()
                    .map(vmrest_power_to_state)
                    .unwrap_or(VmState::Unknown);
                let ip = existing_ips.get(&rvm.path).cloned().flatten();
                Vm {
                    vmx_path: rvm.path.clone(),
                    name: rvm.name.clone(),
                    state,
                    ip,
                }
            })
            .collect();

        // 按虚拟机名称排序
        new_vms.sort_by(|a, b| a.name.cmp(&b.name));

        // 更新列表
        {
            let mut vm_list = vms.lock().unwrap();
            *vm_list = new_vms;
        }

        // 刷新运行中 VM 的 IP 地址
        let vm_paths: Vec<(PathBuf, VmState)> = vms.lock().unwrap()
            .iter()
            .map(|vm| (vm.vmx_path.clone(), vm.state.clone()))
            .collect();

        for (path_buf, state) in vm_paths {
            if state == VmState::Running {
                // 尝试获取 IP
                if let Ok(ip) = Vmrun::get_guest_ip(&path_buf) {
                    let mut vm_list = vms.lock().unwrap();
                    if let Some(m) = vm_list.iter_mut().find(|m| m.vmx_path == path_buf) {
                        m.ip = Some(ip);
                    }
                } else {
                    // 回退到 vmx 文件读取 MAC 推算
                    let mut vm_list = vms.lock().unwrap();
                    if let Some(m) = vm_list.iter_mut().find(|m| m.vmx_path == path_buf) {
                        m.read_ip_from_vmx();
                    }
                }
            }
        }

        true
    }

    /// 通过 vmrun 命令行刷新虚拟机列表和状态（回退方案）
    fn refresh_via_vmrun(vms: &Arc<Mutex<Vec<Vm>>>, scan_dir: &Path) {
        // 重新扫描目录（检测新增/删除的虚拟机）
        let existing: HashMap<PathBuf, (VmState, Option<String>)> = {
            let vm_list = vms.lock().unwrap();
            vm_list.iter()
                .map(|vm| (vm.vmx_path.clone(), (vm.state.clone(), vm.ip.clone())))
                .collect()
        };

        // 重新扫描
        let mut new_vms = Vec::new();
        if let Ok(entries) = std::fs::read_dir(scan_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if dir_name.ends_with(".vmwarevm") {
                            if let Ok(entries) = std::fs::read_dir(&path) {
                                for vmx_entry in entries.flatten() {
                                    let vmx_path = vmx_entry.path();
                                    if vmx_path.extension().and_then(|e| e.to_str()) == Some("vmx") {
                                        let (state, ip) = existing
                                            .get(&vmx_path)
                                            .cloned()
                                            .unwrap_or((VmState::Unknown, None));
                                        let name = vmx_path
                                            .parent()
                                            .and_then(|p| p.file_name())
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("Unknown")
                                            .replace(".vmwarevm", "")
                                            .to_string();
                                        new_vms.push(Vm {
                                            vmx_path,
                                            name,
                                            state,
                                            ip,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按虚拟机名称排序
        new_vms.sort_by(|a, b| a.name.cmp(&b.name));

        {
            let mut vm_list = vms.lock().unwrap();
            *vm_list = new_vms;
        }

        // 刷新所有状态 - 收集所有 vm 路径
        let vm_paths: Vec<PathBuf> = vms.lock().unwrap()
            .iter()
            .map(|vm| vm.vmx_path.clone())
            .collect();

        // 对每个虚拟机获取状态
        for path_buf in vm_paths {
            if let Ok(state) = Vmrun::get_state(&path_buf) {
                let mut vm_list = vms.lock().unwrap();
                if let Some(m) = vm_list.iter_mut().find(|m| m.vmx_path == path_buf) {
                    m.state = state;
                    // 状态更新后刷新 IP
                    m.refresh_ip();
                }
            }
        }
    }
}