use crate::config::{self, Config, Layer, Widget, WidgetKind};
use crate::desktop;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::CheckMenuItem;
use tauri::webview::PageLoadEvent;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Url, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent, Wry,
};

const WIDGET_PREFIX: &str = "widget-";
/// 오버레이 라벨은 `overlay-<편집세션>-<위젯id>`. 세션 번호로 라벨이 겹치지 않게 한다.
const OVERLAY_PREFIX: &str = "overlay-";

/// 배경을 지정하지 않은 웹페이지는 투명 창에서 비쳐 보이므로 흰 배경을 깔아준다.
const WEB_BACKGROUND_SCRIPT: &str = "(function(){var t=function(e){if(!e)return true;var c=getComputedStyle(e).backgroundColor;return c==='transparent'||c==='rgba(0, 0, 0, 0)';};if(t(document.documentElement)&&t(document.body))document.documentElement.style.setProperty('background-color','#ffffff');})();";

pub struct AppState {
    pub config: Mutex<Config>,
    pub edit_mode: Mutex<bool>,
    pub tray_edit: Mutex<Option<CheckMenuItem<Wry>>>,
    overlay_session: AtomicU32,
    /// 동영상 위젯의 영상 비율(가로/세로). 위젯 페이지가 영상을 읽으면 알려준다.
    media_ratio: Mutex<HashMap<String, f64>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Mutex::new(config),
            edit_mode: Mutex::new(false),
            tray_edit: Mutex::new(None),
            overlay_session: AtomicU32::new(0),
            media_ratio: Mutex::new(HashMap::new()),
        }
    }
}

/// 위젯 창을 편집된 영역 안에 들어가는 영상 비율 크기로 줄인다(왼쪽 위 고정).
/// 동영상은 비율이 안 맞는 여백을 검게 칠하므로 여백 자체를 없앤다.
fn snap_to_ratio(app: &AppHandle, id: &str) {
    let Some(ratio) = app.state::<AppState>().media_ratio.lock().unwrap().get(id).copied() else {
        return;
    };
    let mut size = None;
    modify(app, id, |w| {
        let (width, height) = (w.width as f64, w.height as f64);
        if width / height > ratio {
            w.width = (height * ratio).round().max(1.0) as u32;
        } else {
            w.height = (width / ratio).round().max(1.0) as u32;
        }
        size = Some(PhysicalSize::new(w.width, w.height));
    });
    if let (Some(size), Some(win)) = (size, app.get_webview_window(&widget_label(id))) {
        let _ = win.set_size(size);
    }
}

pub fn widget_label(id: &str) -> String {
    format!("{WIDGET_PREFIX}{id}")
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{nanos:x}")
}

pub fn find(app: &AppHandle, id: &str) -> Option<Widget> {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap();
    config.widgets.iter().find(|w| w.id == id).cloned()
}

fn modify(app: &AppHandle, id: &str, f: impl FnOnce(&mut Widget)) {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap();
    if let Some(w) = config.widgets.iter_mut().find(|w| w.id == id) {
        f(w);
    }
}

pub fn is_edit_mode(app: &AppHandle) -> bool {
    *app.state::<AppState>().edit_mode.lock().unwrap()
}

pub fn persist(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap();
    config::save(app, &config)
}

pub fn notify_changed(app: &AppHandle) {
    let _ = app.emit("widgets-changed", ());
}

fn page_script(w: &Widget) -> String {
    let mut script = format!(
        "document.documentElement.style.setProperty('opacity','{}','important');",
        w.opacity
    );
    if w.kind == WidgetKind::Web {
        script.push_str(WEB_BACKGROUND_SCRIPT);
    }
    script
}

fn normalize_source(kind: WidgetKind, source: &str) -> Result<String, String> {
    // 탐색기의 '경로로 복사'는 앞뒤에 따옴표를 붙인다.
    let source = source.trim().trim_matches('"').trim();
    if source.is_empty() {
        return Err("소스를 입력하세요.".into());
    }
    let is_url = source.starts_with("http://") || source.starts_with("https://");
    match kind {
        WidgetKind::Web => {
            let url = if source.contains("://") {
                source.to_string()
            } else {
                format!("https://{source}")
            };
            Url::parse(&url).map_err(|e| format!("잘못된 주소입니다: {e}"))?;
            Ok(url)
        }
        WidgetKind::Image if is_url => Ok(source.to_string()),
        WidgetKind::Image if Path::new(source).is_file() => Ok(source.to_string()),
        WidgetKind::Image => Err(format!("파일을 찾을 수 없습니다: {source}")),
    }
}

fn default_name(kind: WidgetKind, source: &str) -> String {
    let name = match kind {
        WidgetKind::Web => Url::parse(source).ok().and_then(|u| u.host_str().map(str::to_string)),
        WidgetKind::Image => source
            .rsplit(['/', '\\'])
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    };
    name.unwrap_or_else(|| "새 위젯".into())
}

/// 위젯이 어느 모니터에도 걸쳐 있지 않으면(모니터 분리 등) 주 모니터로 옮긴다.
fn ensure_on_screen(app: &AppHandle, w: &mut Widget) -> bool {
    const MARGIN: i32 = 40;
    let Ok(monitors) = app.available_monitors() else {
        return false;
    };
    let visible = monitors.iter().any(|m| {
        let (p, s) = (m.position(), m.size());
        w.x + w.width as i32 > p.x + MARGIN
            && w.x < p.x + s.width as i32 - MARGIN
            && w.y + w.height as i32 > p.y + MARGIN
            && w.y < p.y + s.height as i32 - MARGIN
    });
    if visible {
        return false;
    }
    let Ok(Some(primary)) = app.primary_monitor() else {
        return false;
    };
    w.x = primary.position().x + 100;
    w.y = primary.position().y + 100;
    true
}

fn default_rect(app: &AppHandle, index: usize) -> (i32, i32, u32, u32) {
    let (origin, scale) = match app.primary_monitor() {
        Ok(Some(m)) => (*m.position(), m.scale_factor()),
        _ => (PhysicalPosition::new(0, 0), 1.0),
    };
    let offset = (index % 10) as i32 * (30.0 * scale) as i32;
    (
        origin.x + (200.0 * scale) as i32 + offset,
        origin.y + (150.0 * scale) as i32 + offset,
        (400.0 * scale) as u32,
        (300.0 * scale) as u32,
    )
}

fn apply_flags(win: &WebviewWindow, w: &Widget) {
    let top = w.layer == Layer::Top;
    let _ = win.set_ignore_cursor_events(w.click_through);
    let _ = win.set_always_on_top(top);
    if let Ok(hwnd) = win.hwnd() {
        desktop::set_pinned(hwnd.0 as isize, !top);
    }
    let _ = win.eval(page_script(w));
}

const MOBILE_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

/// WebView2 기본 사용자 에이전트. 모바일을 껐을 때 되돌리기 위해 첫 웹뷰에서 읽어둔다.
static DEFAULT_USER_AGENT: OnceLock<String> = OnceLock::new();

/// 웹 위젯의 사용자 에이전트(PC/모바일)를 정한 뒤 주소를 연다.
/// 웹 위젯은 빈 페이지로 만든 다음 이 함수로 열어서, 첫 로딩부터 맞는 화면을 받는다.
#[cfg(windows)]
fn load_web(win: &WebviewWindow, mobile: bool, url: String) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings2;
    use windows::core::{Interface, HSTRING, PWSTR};

    let _ = win.with_webview(move |webview| unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else {
            return;
        };
        if let Ok(settings) = core.Settings().and_then(|s| s.cast::<ICoreWebView2Settings2>()) {
            if DEFAULT_USER_AGENT.get().is_none() {
                let mut current = PWSTR::null();
                if settings.UserAgent(&mut current).is_ok() {
                    let _ = DEFAULT_USER_AGENT.set(webview2_com::take_pwstr(current));
                }
            }
            let agent = if mobile {
                Some(MOBILE_USER_AGENT)
            } else {
                DEFAULT_USER_AGENT.get().map(String::as_str)
            };
            if let Some(agent) = agent {
                let _ = settings.SetUserAgent(&HSTRING::from(agent));
            }
        }
        let _ = core.Navigate(&HSTRING::from(url));
    });
}

#[cfg(not(windows))]
fn load_web(win: &WebviewWindow, _mobile: bool, url: String) {
    if let Ok(url) = Url::parse(&url) {
        let _ = win.navigate(url);
    }
}

fn open_widget(app: &AppHandle, w: &Widget) -> Result<(), String> {
    let url = match w.kind {
        WidgetKind::Web => WebviewUrl::External(Url::parse("about:blank").map_err(|e| e.to_string())?),
        WidgetKind::Image => WebviewUrl::App(format!("widget.html?id={}", w.id).into()),
    };
    let id = w.id.clone();
    let win = WebviewWindowBuilder::new(app, widget_label(&w.id), url)
        .title(&w.name)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .on_page_load(move |webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                if let Some(w) = find(webview.app_handle(), &id) {
                    let _ = webview.eval(page_script(&w));
                }
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    if w.kind == WidgetKind::Web {
        crate::menu::attach_web_menu(&win, w.id.clone());
        load_web(&win, w.mobile, w.source.clone());
    }
    let _ = win.set_position(PhysicalPosition::new(w.x, w.y));
    let _ = win.set_size(PhysicalSize::new(w.width, w.height));
    apply_flags(&win, w);
    if w.visible {
        show_widget(&win, w);
    }
    Ok(())
}

fn show_widget(win: &WebviewWindow, w: &Widget) {
    let _ = win.show();
    if w.layer == Layer::Desktop {
        if let Ok(hwnd) = win.hwnd() {
            desktop::push_bottom(hwnd.0 as isize);
        }
    }
}

fn overlay_for(app: &AppHandle, id: &str) -> Option<WebviewWindow> {
    let suffix = format!("-{id}");
    app.webview_windows()
        .into_iter()
        .find(|(label, _)| label.starts_with(OVERLAY_PREFIX) && label.ends_with(&suffix))
        .map(|(_, win)| win)
}

fn open_overlay(app: &AppHandle, w: &Widget) -> Result<(), String> {
    let session = app.state::<AppState>().overlay_session.load(Ordering::SeqCst);
    let win = WebviewWindowBuilder::new(
        app,
        format!("{OVERLAY_PREFIX}{session}-{}", w.id),
        WebviewUrl::App(format!("overlay.html?id={}", w.id).into()),
    )
    .title("위젯 편집")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .resizable(true)
    .min_inner_size(60.0, 40.0)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;
    let _ = win.set_position(PhysicalPosition::new(w.x, w.y));
    let _ = win.set_size(PhysicalSize::new(w.width, w.height));
    let _ = win.show();
    Ok(())
}

pub fn apply_edit_mode(app: &AppHandle, on: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut edit = state.edit_mode.lock().unwrap();
        if *edit == on {
            return Ok(());
        }
        *edit = on;
    }
    if let Some(item) = state.tray_edit.lock().unwrap().as_ref() {
        let _ = item.set_checked(on);
    }
    if on {
        state.overlay_session.fetch_add(1, Ordering::SeqCst);
        let widgets = state.config.lock().unwrap().widgets.clone();
        for w in widgets.iter().filter(|w| w.visible) {
            open_overlay(app, w)?;
        }
    } else {
        for (label, win) in app.webview_windows() {
            if label.starts_with(OVERLAY_PREFIX) {
                let _ = win.destroy();
            }
        }
        let ids: Vec<String> = state.media_ratio.lock().unwrap().keys().cloned().collect();
        for id in ids {
            snap_to_ratio(app, &id);
        }
        persist(app)?;
    }
    let _ = app.emit("edit-mode-changed", on);
    notify_changed(app);
    Ok(())
}

pub fn restore_all(app: &AppHandle) {
    let widgets = {
        let state = app.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        for w in config.widgets.iter_mut() {
            ensure_on_screen(app, w);
        }
        config.widgets.clone()
    };
    for w in widgets.iter().filter(|w| w.visible) {
        if let Err(e) = open_widget(app, w) {
            eprintln!("위젯 '{}'을 열지 못했습니다: {e}", w.name);
        }
    }
}

/// 트레이의 '위젯을 화면 안으로 모으기': 보이지 않는 위치의 위젯을 주 모니터로 옮긴다.
pub fn gather(app: &AppHandle) {
    let moved: Vec<Widget> = {
        let state = app.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        config
            .widgets
            .iter_mut()
            .filter_map(|w| ensure_on_screen(app, w).then(|| w.clone()))
            .collect()
    };
    for w in &moved {
        let pos = PhysicalPosition::new(w.x, w.y);
        if let Some(win) = app.get_webview_window(&widget_label(&w.id)) {
            let _ = win.set_position(pos);
        }
        if let Some(overlay) = overlay_for(app, &w.id) {
            let _ = overlay.set_position(pos);
        }
    }
    let _ = persist(app);
    notify_changed(app);
}

pub fn open_settings(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("위젯 설정")
        .inner_size(860.0, 680.0)
        .min_inner_size(600.0, 400.0)
        .center()
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    let label = window.label();
    if let Some(rest) = label.strip_prefix(OVERLAY_PREFIX) {
        // 오버레이를 옮기거나 크기를 바꾸면 위젯 창이 따라간다.
        // 이벤트 값 대신 현재 창 상태를 읽어서, 생성 직후의 늦은 이벤트가 위치를 덮어쓰지 않게 한다.
        if !matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_))
            || !window.is_visible().unwrap_or(false)
        {
            return;
        }
        let Some((_, id)) = rest.split_once('-') else {
            return;
        };
        let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) else {
            return;
        };
        if size.width == 0 || size.height == 0 {
            return;
        }
        modify(app, id, |w| {
            w.x = pos.x;
            w.y = pos.y;
            w.width = size.width;
            w.height = size.height;
        });
        if let Some(win) = app.get_webview_window(&widget_label(id)) {
            let _ = win.set_position(pos);
            let _ = win.set_size(size);
        }
    } else if let Some(id) = label.strip_prefix(WIDGET_PREFIX) {
        // 바탕화면 고정 위젯은 클릭해서 활성화돼도 다시 맨 아래로 내린다.
        if let WindowEvent::Focused(true) = event {
            if find(app, id).is_some_and(|w| w.layer == Layer::Desktop) {
                if let Ok(hwnd) = window.hwnd() {
                    desktop::push_bottom(hwnd.0 as isize);
                }
            }
        }
    }
}

// 창을 만드는 명령은 async여야 한다. 동기 명령은 메인 스레드에서 돌기 때문에
// 그 안에서 창을 만들면 Windows에서 교착(앱 멈춤)이 생긴다.
#[tauri::command]
pub fn get_widgets(app: AppHandle) -> Vec<Widget> {
    app.state::<AppState>().config.lock().unwrap().widgets.clone()
}

#[tauri::command]
pub fn get_widget(app: AppHandle, id: String) -> Option<Widget> {
    find(&app, &id)
}

#[tauri::command]
pub async fn add_widget(app: AppHandle, kind: WidgetKind, source: String) -> Result<Widget, String> {
    let source = normalize_source(kind, &source)?;
    let count = app.state::<AppState>().config.lock().unwrap().widgets.len();
    let (x, y, width, height) = default_rect(&app, count);
    let widget = Widget {
        id: new_id(),
        name: default_name(kind, &source),
        kind,
        source,
        x,
        y,
        width,
        height,
        opacity: 1.0,
        layer: Layer::Desktop,
        locked: false,
        click_through: false,
        visible: true,
        mobile: false,
    };
    app.state::<AppState>().config.lock().unwrap().widgets.push(widget.clone());
    if let Err(e) = open_widget(&app, &widget) {
        app.state::<AppState>()
            .config
            .lock()
            .unwrap()
            .widgets
            .retain(|w| w.id != widget.id);
        return Err(e);
    }
    persist(&app)?;
    // 새 위젯은 바로 옮기고 크기를 맞출 수 있게 편집 모드로 들어간다.
    if is_edit_mode(&app) {
        open_overlay(&app, &widget)?;
    } else {
        apply_edit_mode(&app, true)?;
    }
    Ok(widget)
}

#[tauri::command]
pub async fn update_widget(app: AppHandle, widget: Widget) -> Result<(), String> {
    apply_update(&app, widget)
}

/// 설정창·우클릭 메뉴에서 바꾼 값 반영. 종류와 위치·크기는 앱 쪽 값을 유지한다.
/// 창을 만들 수 있으므로 메인 스레드가 아닌 곳에서 호출해야 한다.
pub fn apply_update(app: &AppHandle, widget: Widget) -> Result<(), String> {
    let app = app.clone();
    let old = find(&app, &widget.id).ok_or("위젯을 찾을 수 없습니다.")?;
    let source = normalize_source(old.kind, &widget.source)?;
    let new = Widget {
        kind: old.kind,
        source,
        x: old.x,
        y: old.y,
        width: old.width,
        height: old.height,
        opacity: widget.opacity.clamp(0.1, 1.0),
        ..widget
    };
    modify(&app, &new.id, |w| *w = new.clone());

    match app.get_webview_window(&widget_label(&new.id)) {
        Some(win) => {
            if new.kind == WidgetKind::Web && (new.source != old.source || new.mobile != old.mobile) {
                load_web(&win, new.mobile, new.source.clone());
            }
            let _ = win.set_title(&new.name);
            apply_flags(&win, &new);
            if new.visible {
                show_widget(&win, &new);
            } else {
                let _ = win.hide();
            }
        }
        None if new.visible => open_widget(&app, &new)?,
        None => {}
    }

    if is_edit_mode(&app) {
        match overlay_for(&app, &new.id) {
            Some(overlay) if new.visible => {
                let _ = overlay.show();
            }
            Some(overlay) => {
                let _ = overlay.hide();
            }
            None if new.visible => open_overlay(&app, &new)?,
            None => {}
        }
    }

    persist(&app)?;
    let _ = app.emit("widget-updated", &new.id);
    Ok(())
}

#[tauri::command]
pub fn remove_widget(app: AppHandle, id: String) -> Result<(), String> {
    delete(&app, &id)
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(&widget_label(id)) {
        let _ = win.destroy();
    }
    if let Some(overlay) = overlay_for(app, id) {
        let _ = overlay.destroy();
    }
    let state = app.state::<AppState>();
    state.media_ratio.lock().unwrap().remove(id);
    state.config.lock().unwrap().widgets.retain(|w| w.id != id);
    persist(app)?;
    notify_changed(app);
    Ok(())
}

/// 위젯 페이지가 동영상 비율을 알려준다(사진이면 None). 편집 중이 아니면 바로 창을 맞춘다.
#[tauri::command]
pub async fn set_media_ratio(app: AppHandle, id: String, ratio: Option<f64>) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut ratios = state.media_ratio.lock().unwrap();
        match ratio.filter(|r| r.is_finite() && *r > 0.0) {
            Some(r) => ratios.insert(id.clone(), r),
            None => ratios.remove(&id),
        };
    }
    if !is_edit_mode(&app) {
        snap_to_ratio(&app, &id);
        persist(&app)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_edit_mode(app: AppHandle) -> bool {
    is_edit_mode(&app)
}

#[tauri::command]
pub async fn set_edit_mode(app: AppHandle, on: bool) -> Result<(), String> {
    apply_edit_mode(&app, on)
}
