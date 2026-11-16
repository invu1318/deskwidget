//! '바탕화면에 고정' 처리. 창의 소유자를 바탕화면(Progman)으로 바꾸면
//! Win+D(바탕화면 보기)에도 숨겨지지 않고, 맨 아래로 내려 다른 창 뒤에 깔리게 한다.

/// 자석 정렬 대상인 다른 위젯들의 `[왼쪽, 위, 오른쪽, 아래]`(물리 픽셀)를 돌려준다.
pub type SnapTargets = Box<dyn Fn() -> Vec<[i32; 4]> + Send>;

#[cfg(windows)]
mod imp {
    use super::SnapTargets;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateRoundRectRgn, GetMonitorInfoW, MonitorFromRect, SetWindowRgn, MONITORINFO,
        MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::Shell::{
        DefSubclassProc, RemoveWindowSubclass, SHQueryUserNotificationState, SetWindowSubclass,
        QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetWindowLongPtrW, SetWindowPos, GWLP_HWNDPARENT, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_MOVING, WM_NCDESTROY,
    };

    /// 이 거리(물리 픽셀) 안으로 들어오면 가장자리에 붙는다.
    const SNAP_RANGE: i32 = 12;
    const SNAP_SUBCLASS_ID: usize = 1;

    /// 창 모서리를 둥글게 자른다. 창 크기가 바뀌면 다시 불러야 한다.
    pub fn set_radius(hwnd: isize, width: u32, height: u32, radius: u32) {
        let radius = radius.min(width.min(height) / 2);
        unsafe {
            let region = (radius > 0).then(|| {
                let diameter = (radius * 2) as i32;
                CreateRoundRectRgn(0, 0, width as i32 + 1, height as i32 + 1, diameter, diameter)
            });
            // 영역 핸들은 창이 가져가므로 따로 지우지 않는다.
            SetWindowRgn(HWND(hwnd as _), region, true);
        }
    }

    /// 전체화면 앱(게임, 전체화면 영상, 발표 모드)이 떠 있는지 Windows에 묻는다.
    pub fn fullscreen_active() -> bool {
        unsafe { SHQueryUserNotificationState() }.is_ok_and(|state| {
            state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE
        })
    }

    /// 창을 끌 때 화면 작업 영역과 다른 위젯의 가장자리에 붙게 한다.
    /// 창을 만든 스레드(메인 스레드)에서 불러야 한다.
    pub fn enable_snap(hwnd: isize, targets: SnapTargets) {
        let data = Box::into_raw(Box::new(targets));
        unsafe {
            if !SetWindowSubclass(HWND(hwnd as _), Some(snap_proc), SNAP_SUBCLASS_ID, data as usize).as_bool() {
                drop(Box::from_raw(data));
            }
        }
    }

    fn snap_delta(start: i32, end: i32, edges: &[i32]) -> i32 {
        edges
            .iter()
            .flat_map(|edge| [edge - start, edge - end])
            .filter(|delta| delta.abs() <= SNAP_RANGE)
            .min_by_key(|delta| delta.abs())
            .unwrap_or(0)
    }

    unsafe extern "system" fn snap_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        match msg {
            // 끄는 동안 Windows가 제안한 위치(lparam의 RECT)를 붙는 위치로 고친다.
            WM_MOVING => {
                let rect = &mut *(lparam.0 as *mut RECT);
                let targets = &*(data as *const SnapTargets);
                let (mut xs, mut ys) = (Vec::new(), Vec::new());
                let mut info = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
                    xs.extend([info.rcWork.left, info.rcWork.right]);
                    ys.extend([info.rcWork.top, info.rcWork.bottom]);
                }
                for [left, top, right, bottom] in targets() {
                    xs.extend([left, right]);
                    ys.extend([top, bottom]);
                }
                let dx = snap_delta(rect.left, rect.right, &xs);
                let dy = snap_delta(rect.top, rect.bottom, &ys);
                rect.left += dx;
                rect.right += dx;
                rect.top += dy;
                rect.bottom += dy;
                DefSubclassProc(hwnd, msg, wparam, lparam);
                return LRESULT(1);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(snap_proc), SNAP_SUBCLASS_ID);
                drop(Box::from_raw(data as *mut SnapTargets));
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    pub fn set_pinned(hwnd: isize, pinned: bool) {
        unsafe {
            let hwnd = HWND(hwnd as _);
            let owner = if pinned {
                FindWindowW(w!("Progman"), PCWSTR::null()).unwrap_or_default()
            } else {
                HWND::default()
            };
            SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner.0 as isize);
        }
        if pinned {
            push_bottom(hwnd);
        }
    }

    pub fn push_bottom(hwnd: isize) {
        unsafe {
            let _ = SetWindowPos(
                HWND(hwnd as _),
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_pinned(_hwnd: isize, _pinned: bool) {}
    pub fn push_bottom(_hwnd: isize) {}
    pub fn set_radius(_hwnd: isize, _width: u32, _height: u32, _radius: u32) {}
    pub fn fullscreen_active() -> bool {
        false
    }
    pub fn enable_snap(_hwnd: isize, _targets: super::SnapTargets) {}
}

pub use imp::{enable_snap, fullscreen_active, push_bottom, set_pinned, set_radius};
