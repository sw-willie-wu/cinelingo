use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cuda,
    Vulkan,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HwInfo {
    pub backend: String,            // "cuda" | "vulkan" | "cpu"
    pub gpu_name: Option<String>,
    pub has_gpu: bool,
}

/// 給 hw_info_from 的描述（從 wgpu AdapterInfo 萃取，便於單元測試）。
pub struct AdapterDesc {
    pub name: String,
    pub vendor: Vendor,
    pub is_cpu: bool,               // device_type == DeviceType::Cpu（過濾 WARP 等軟體轉接器）
}

fn backend_str(b: Backend) -> String {
    match b { Backend::Cuda => "cuda", Backend::Vulkan => "vulkan", Backend::Cpu => "cpu" }.to_string()
}

/// 有 NVIDIA→CUDA；否則有任何 GPU→Vulkan；無→CPU。
pub fn pick_backend(adapters: &[Vendor]) -> Backend {
    if adapters.contains(&Vendor::Nvidia) {
        Backend::Cuda
    } else if !adapters.is_empty() {
        Backend::Vulkan
    } else {
        Backend::Cpu
    }
}

pub fn vendor_from_pci(id: u32) -> Vendor {
    match id {
        0x10DE => Vendor::Nvidia,
        0x1002 => Vendor::Amd,
        0x8086 => Vendor::Intel,
        _ => Vendor::Other,
    }
}

pub fn hw_info_from(adapters: &[AdapterDesc]) -> HwInfo {
    let mut seen = std::collections::HashSet::new();
    let gpus: Vec<&AdapterDesc> = adapters
        .iter()
        .filter(|a| !a.is_cpu)
        .filter(|a| seen.insert(a.name.clone()))
        .collect();
    let vendors: Vec<Vendor> = gpus.iter().map(|a| a.vendor).collect();
    let backend = pick_backend(&vendors);
    let has_gpu = !gpus.is_empty();
    let gpu_name = match backend {
        Backend::Cuda => gpus.iter().find(|a| a.vendor == Vendor::Nvidia).map(|a| a.name.clone()),
        Backend::Vulkan => gpus.first().map(|a| a.name.clone()),
        Backend::Cpu => None,
    };
    HwInfo { backend: backend_str(backend), gpu_name, has_gpu }
}

use std::sync::OnceLock;
static HW: OnceLock<HwInfo> = OnceLock::new();

/// 實際枚舉（阻塞）：過濾軟體/CPU 轉接器、dedupe；結果 memoize（硬體 runtime 不變）。
pub fn detect_hardware_blocking() -> HwInfo {
    HW.get_or_init(|| {
        let inst = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let descs: Vec<AdapterDesc> = inst
            .enumerate_adapters(wgpu::Backends::all())
            .iter()
            .map(|a| {
                let info = a.get_info();
                AdapterDesc {
                    name: info.name.clone(),
                    vendor: vendor_from_pci(info.vendor),
                    is_cpu: info.device_type == wgpu::DeviceType::Cpu,
                }
            })
            .collect();
        hw_info_from(&descs)
    })
    .clone()
}

/// 列舉系統 GPU adapters 的 vendor；失敗回空（→CPU）。
pub fn detect_adapters() -> Vec<Vendor> {
    let inst = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    inst.enumerate_adapters(wgpu::Backends::all())
        .iter()
        .map(|a| vendor_from_pci(a.get_info().vendor))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_wins() {
        assert_eq!(pick_backend(&[Vendor::Intel, Vendor::Nvidia]), Backend::Cuda);
    }

    #[test]
    fn gpu_no_nvidia_vulkan() {
        assert_eq!(pick_backend(&[Vendor::Amd]), Backend::Vulkan);
    }

    #[test]
    fn none_cpu() {
        assert_eq!(pick_backend(&[]), Backend::Cpu);
    }

    #[test]
    fn pci() {
        assert_eq!(vendor_from_pci(0x10DE), Vendor::Nvidia);
        assert_eq!(vendor_from_pci(1), Vendor::Other);
    }

    fn d(name: &str, vendor: Vendor, is_cpu: bool) -> AdapterDesc {
        AdapterDesc { name: name.into(), vendor, is_cpu }
    }
    #[test]
    fn warp_only_is_cpu() {
        let hw = hw_info_from(&[d("Microsoft Basic Render Driver", Vendor::Other, true)]);
        assert_eq!(hw.backend, "cpu");
        assert!(!hw.has_gpu);
        assert!(hw.gpu_name.is_none());
    }
    #[test]
    fn nvidia_plus_intel_cuda_named() {
        let hw = hw_info_from(&[d("Intel UHD", Vendor::Intel, false), d("RTX 3080", Vendor::Nvidia, false)]);
        assert_eq!(hw.backend, "cuda");
        assert!(hw.has_gpu);
        assert_eq!(hw.gpu_name.as_deref(), Some("RTX 3080"));
    }
    #[test]
    fn amd_only_vulkan() {
        let hw = hw_info_from(&[d("RX 6800", Vendor::Amd, false)]);
        assert_eq!(hw.backend, "vulkan");
        assert_eq!(hw.gpu_name.as_deref(), Some("RX 6800"));
    }
    #[test]
    fn dedupe_same_name() {
        let hw = hw_info_from(&[d("RTX 3080", Vendor::Nvidia, false), d("RTX 3080", Vendor::Nvidia, false)]);
        assert_eq!(hw.backend, "cuda");
        assert!(hw.has_gpu);
    }
}
