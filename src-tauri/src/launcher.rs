use windows::core::{w, HSTRING};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub(crate) fn open(target: &str) -> Result<(), String> {
    // 풀 스레드가 아닌 일회용 스레드에서 COM을 초기화하고 끝나기 전에 해제한다.
    // 다른 명령이 쓰는 스레드의 아파트에는 영향을 주지 않는다.
    let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(target), None, None, SW_SHOWNORMAL) };
    if com {
        unsafe { CoUninitialize() };
    }
    // ShellExecute는 32보다 큰 값을 돌려주면 성공이다.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err("열 수 없어요".into())
    }
}

#[tauri::command]
pub async fn open_target(target: String) -> Result<(), String> {
    let target = target.trim().to_string();
    if target.is_empty() {
        return Err("열 수 없어요".into());
    }
    let (tx, rx) = tauri::async_runtime::channel(1);
    std::thread::spawn(move || {
        let _ = tx.blocking_send(open(&target));
    });
    let mut rx = rx;
    rx.recv().await.unwrap_or_else(|| Err("열 수 없어요".into()))
}
