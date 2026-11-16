use serde::Serialize;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

// CPU 사용률을 재는 구간. 호출마다 구간의 처음과 끝을 직접 읽어서,
// 여러 위젯이 동시에 불러도 서로의 측정값을 가로채지 않는다.
const CPU_WINDOW: Duration = Duration::from_millis(250);

#[derive(Serialize)]
pub struct Usage {
    used: u64,
    total: u64,
}

// 읽지 못한 항목은 null로 보내고, 화면에서 그 줄만 오류로 표시한다.
#[derive(Serialize)]
pub struct SystemStats {
    cpu: Option<f64>,
    memory: Option<Usage>,
    disk: Option<Usage>,
}

fn ticks(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

// (전체, 유휴) CPU 시간. 커널 시간에는 유휴 시간이 포함되어 있다.
fn cpu_times() -> Option<(u64, u64)> {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some((ticks(kernel) + ticks(user), ticks(idle)))
}

fn cpu_percent() -> Option<f64> {
    let before = cpu_times()?;
    std::thread::sleep(CPU_WINDOW);
    let after = cpu_times()?;
    let total = after.0.checked_sub(before.0).filter(|t| *t > 0)?;
    let idle = after.1.saturating_sub(before.1).min(total);
    Some((total - idle) as f64 * 100.0 / total as f64)
}

fn memory() -> Option<Usage> {
    let mut mem = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut mem) }.ok()?;
    (mem.ullTotalPhys > 0).then(|| Usage {
        used: mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys),
        total: mem.ullTotalPhys,
    })
}

fn disk() -> Option<Usage> {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root: Vec<u16> = format!("{drive}\\").encode_utf16().chain(Some(0)).collect();
    let (mut total, mut free) = (0u64, 0u64);
    unsafe { GetDiskFreeSpaceExW(PCWSTR(root.as_ptr()), None, Some(&mut total), Some(&mut free)) }.ok()?;
    (total > 0).then(|| Usage { used: total.saturating_sub(free), total })
}

#[tauri::command]
pub async fn get_system_stats() -> SystemStats {
    tauri::async_runtime::spawn_blocking(|| SystemStats { cpu: cpu_percent(), memory: memory(), disk: disk() })
        .await
        .unwrap_or(SystemStats { cpu: None, memory: None, disk: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 동시에 측정해도 서로의 표본을 가로채지 않고 둘 다 제대로 된 값을 얻는다.
    #[test]
    fn overlapping_cpu_readings_are_both_valid() {
        let other = std::thread::spawn(cpu_percent);
        let mine = cpu_percent();
        let theirs = other.join().unwrap();
        println!("cpu {mine:?} {theirs:?}");
        for v in [mine, theirs] {
            assert!((0.0..=100.0).contains(&v.expect("cpu reading failed")));
        }
    }
}
