#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod desktop;
mod launcher;
mod media_session;
mod menu;
mod system_stats;
mod updater;
mod widgets;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use widgets::AppState;

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "설정 열기", true, None::<&str>)?;
    let edit = CheckMenuItem::with_id(app, "edit", "편집 모드", true, false, None::<&str>)?;
    let gather = MenuItem::with_id(app, "gather", "위젯을 화면 안으로 모으기", true, None::<&str>)?;
    let hide_all = MenuItem::with_id(app, "hide_all", "모든 위젯 숨기기/보이기", true, Some("Ctrl+Alt+H"))?;
    let click_all = MenuItem::with_id(app, "click_all", "클릭 통과 켜기/끄기", true, Some("Ctrl+Alt+C"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    let update = MenuItem::with_id(app, "update", "업데이트 확인", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open, &edit, &gather, &hide_all, &click_all, &separator, &update, &quit],
    )?;
    *app.state::<AppState>().tray_edit.lock().unwrap() = Some(edit);

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("앱 아이콘이 없습니다"))
        .tooltip(format!("DeskWidget {}", app.package_info().version))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                let _ = widgets::open_settings(app);
            }
            "edit" => {
                let on = !*app.state::<AppState>().edit_mode.lock().unwrap();
                let _ = widgets::apply_edit_mode(app, on);
            }
            "gather" => widgets::gather(app),
            "hide_all" => widgets::toggle_hidden_all(app),
            "click_all" => widgets::toggle_click_through_all(app),
            "update" => updater::check(app.clone(), true),
            "quit" => {
                let _ = widgets::persist(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = widgets::open_settings(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = widgets::open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_menu_event(|app, event| menu::on_menu_event(app, event.id().as_ref()))
        .invoke_handler(tauri::generate_handler![
            menu::show_widget_menu,
            widgets::get_widgets,
            widgets::get_widget,
            widgets::add_widget,
            widgets::update_widget,
            widgets::remove_widget,
            widgets::set_media_ratio,
            widgets::set_memo_text,
            widgets::set_widget_data,
            widgets::list_slides,
            widgets::get_edit_mode,
            widgets::set_edit_mode,
            system_stats::get_system_stats,
            launcher::open_target,
            media_session::get_media_info,
            media_session::media_control,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let mut config = config::load(&handle);
            // 첫 실행 때 한 번만 부팅 시 실행을 켠다. 이후에는 설정창의 선택을 따른다.
            // 개발 빌드는 target\debug 실행 파일이 부팅 항목에 등록되지 않도록 제외한다.
            if !cfg!(debug_assertions) && !config.autostart_initialized {
                let _ = handle.autolaunch().enable();
                config.autostart_initialized = true;
                let _ = config::save(&handle, &config);
            }
            app.manage(AppState::new(config));
            build_tray(&handle)?;
            // 위젯이 없어도 설정창을 띄우지 않고 트레이에서만 시작한다.
            widgets::restore_all(&handle);

            let shortcuts: [(&str, fn(&AppHandle)); 2] = [
                ("ctrl+alt+h", widgets::toggle_hidden_all),
                ("ctrl+alt+c", widgets::toggle_click_through_all),
            ];
            for (keys, action) in shortcuts {
                let result = handle.global_shortcut().on_shortcut(keys, move |app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        action(app);
                    }
                });
                // 다른 프로그램이 이미 쓰는 조합이면 등록되지 않는다. 트레이 메뉴로는 계속 쓸 수 있다.
                if let Err(e) = result {
                    eprintln!("단축키 {keys} 등록 실패: {e}");
                }
            }

            // 전체화면 앱이 뜨고 사라질 때 위젯 페이지에 알려 재생을 멈추거나 잇게 한다.
            let watcher = handle.clone();
            std::thread::spawn(move || {
                let mut paused = false;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let now = desktop::fullscreen_active();
                    if now != paused {
                        paused = now;
                        let _ = watcher.emit("playback-paused", now);
                    }
                }
            });
            // 개발 빌드는 정식 버전으로 덮어쓰지 않도록 시작 시 확인을 건너뛴다.
            if !cfg!(debug_assertions) {
                updater::check(handle.clone(), false);
            }
            Ok(())
        })
        .on_window_event(widgets::on_window_event)
        .build(tauri::generate_context!())
        .expect("앱을 시작하지 못했습니다")
        .run(|_app, event| {
            // 창을 모두 닫아도 트레이에 남아 있게 한다. 트레이의 '종료'만 앱을 끝낸다.
            if let RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
