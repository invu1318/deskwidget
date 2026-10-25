//! '바탕화면에 고정' 처리. 창의 소유자를 바탕화면(Progman)으로 바꾸면
//! Win+D(바탕화면 보기)에도 숨겨지지 않고, 맨 아래로 내려 다른 창 뒤에 깔리게 한다.

#[cfg(windows)]
mod imp {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetWindowLongPtrW, SetWindowPos, GWLP_HWNDPARENT, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

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
}

pub use imp::{push_bottom, set_pinned};
