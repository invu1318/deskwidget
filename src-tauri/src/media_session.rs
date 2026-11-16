use serde::Serialize;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use windows::core::{Error, RuntimeType, HRESULT};
use windows_future::{AsyncStatus, IAsyncOperation};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

// 비동기 호출 하나를 기다리는 최대 시간과, 명령이 작업 스레드의 응답을 기다리는 최대 시간.
const STEP_TIMEOUT: Duration = Duration::from_secs(2);
const REPLY_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Serialize, Default)]
pub struct MediaInfo {
    active: bool,
    title: String,
    artist: String,
    playing: bool,
}

enum Request {
    Read(Sender<MediaInfo>),
    Control(String, Sender<Result<(), String>>),
}

// HRESULT_FROM_WIN32(ERROR_TIMEOUT)
fn timeout_error() -> Error {
    Error::new(HRESULT(0x800705B4_u32 as i32), "media request timed out")
}

// join()은 끝날 때까지 무한정 기다리므로, 상태를 확인하며 기다리다 시간이 지나면 취소한다.
fn wait<T: RuntimeType>(op: &IAsyncOperation<T>) -> windows::core::Result<T> {
    let start = Instant::now();
    while op.Status()? == AsyncStatus::Started {
        if start.elapsed() > STEP_TIMEOUT {
            let _ = op.Cancel();
            return Err(timeout_error());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    op.GetResults()
}

fn current() -> windows::core::Result<Session> {
    wait(&Manager::RequestAsync()?)?.GetCurrentSession()
}

fn read() -> windows::core::Result<MediaInfo> {
    let session = current()?;
    let props = wait(&session.TryGetMediaPropertiesAsync()?)?;
    Ok(MediaInfo {
        active: true,
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        playing: session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing,
    })
}

fn control(action: &str) -> windows::core::Result<()> {
    let session = current()?;
    match action {
        "prev" => wait(&session.TrySkipPreviousAsync()?)?,
        "next" => wait(&session.TrySkipNextAsync()?)?,
        _ => wait(&session.TryTogglePlayPauseAsync()?)?,
    };
    Ok(())
}

// WinRT 호출은 이 전용 스레드에서만 실행한다. 스레드를 직접 만들어 MTA로 초기화하므로
// 다른 명령이 COM을 어떻게 초기화했는지와 상관없이 아파트가 항상 올바르다.
fn worker() -> &'static Sender<Request> {
    static WORKER: OnceLock<Sender<Request>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = channel::<Request>();
        std::thread::spawn(move || {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            for request in rx {
                match request {
                    // 재생 중인 세션이 없거나 시간이 초과되면 오류 대신 active=false를 돌려준다.
                    Request::Read(reply) => {
                        let _ = reply.send(read().unwrap_or_default());
                    }
                    Request::Control(action, reply) => {
                        let _ = reply.send(control(&action).map_err(|e| e.to_string()));
                    }
                }
            }
        });
        tx
    })
}

#[tauri::command]
pub async fn get_media_info() -> MediaInfo {
    tauri::async_runtime::spawn_blocking(|| {
        let (tx, rx) = channel();
        if worker().send(Request::Read(tx)).is_err() {
            return MediaInfo::default();
        }
        rx.recv_timeout(REPLY_TIMEOUT).unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn media_control(action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = channel();
        worker().send(Request::Control(action, tx)).map_err(|e| e.to_string())?;
        rx.recv_timeout(REPLY_TIMEOUT).map_err(|_| "미디어 앱이 응답하지 않아요".to_string())?
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    // 바로가기 열기가 한 번 있었어도 미디어 요청은 제한 시간 안에 응답해야 한다.
    #[test]
    fn read_answers_after_launcher_open() {
        assert!(crate::launcher::open("C:\\이 경로는 없어요\\x.exe").is_err());
        let (tx, rx) = channel();
        worker().send(Request::Read(tx)).unwrap();
        let info = rx.recv_timeout(REPLY_TIMEOUT).expect("worker did not answer");
        println!("active={} title={:?} artist={:?} playing={}", info.active, info.title, info.artist, info.playing);
    }
}
