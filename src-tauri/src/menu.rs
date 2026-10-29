//! 위젯 우클릭 메뉴. 사진 위젯과 편집 오버레이는 Tauri 메뉴를 띄우고,
//! 웹 위젯은 WebView2 기본 메뉴 아래에 같은 항목을 덧붙인다.

use crate::config::{Layer, WidgetKind};
use crate::widgets;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, WebviewWindow, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// 메뉴 항목 id는 `w|<위젯id>|<동작>`. 트레이 메뉴 id와 겹치지 않게 접두어를 둔다.
const PREFIX: &str = "w|";
const OPACITY_STEPS: [u32; 5] = [100, 80, 60, 40, 20];

pub enum Entry {
    Item {
        action: String,
        label: String,
        checked: Option<bool>,
    },
    Separator,
    Submenu {
        label: String,
        items: Vec<Entry>,
    },
}

fn item(action: &str, label: &str) -> Entry {
    Entry::Item { action: action.into(), label: label.into(), checked: None }
}

fn check(action: &str, label: &str, checked: bool) -> Entry {
    Entry::Item { action: action.into(), label: label.into(), checked: Some(checked) }
}

fn entries(app: &AppHandle, id: &str) -> Option<Vec<Entry>> {
    let w = widgets::find(app, id)?;
    let mut list = vec![
        check("edit", "편집 모드", widgets::is_edit_mode(app)),
        check("lock", "위치 잠금", w.locked),
        check("top", "항상 위에 표시", w.layer == Layer::Top),
        check("click", "클릭 통과 (해제는 설정창에서)", w.click_through),
    ];
    if w.kind == WidgetKind::Web {
        list.push(check("mobile", "모바일 페이지", w.mobile));
    }
    let percent = (w.opacity * 100.0).round() as u32;
    list.push(Entry::Submenu {
        label: "불투명도".into(),
        items: OPACITY_STEPS
            .iter()
            .map(|p| check(&format!("opacity:{p}"), &format!("{p}%"), *p == percent))
            .collect(),
    });
    list.extend([
        Entry::Separator,
        item("reload", "새로고침"),
        item("hide", "위젯 숨기기"),
        item("settings", "설정 열기"),
        Entry::Separator,
        item("delete", "위젯 삭제"),
    ]);
    Some(list)
}

fn command_id(id: &str, action: &str) -> String {
    format!("{PREFIX}{id}|{action}")
}

/// 메뉴 항목 선택 처리. 트레이 등 다른 메뉴의 항목은 무시한다.
pub fn on_menu_event(app: &AppHandle, command: &str) {
    let Some((id, action)) = command.strip_prefix(PREFIX).and_then(|rest| rest.split_once('|')) else {
        return;
    };
    let (app, id, action) = (app.clone(), id.to_string(), action.to_string());
    // 편집 모드 등은 창을 만들기 때문에 메인 스레드 밖에서 실행한다.
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run(&app, &id, &action) {
            eprintln!("메뉴 동작 '{action}' 실패: {e}");
        }
    });
}

fn run(app: &AppHandle, id: &str, action: &str) -> Result<(), String> {
    match action {
        "edit" => return widgets::apply_edit_mode(app, !widgets::is_edit_mode(app)),
        "settings" => return widgets::open_settings(app),
        "reload" => {
            if let Some(win) = app.get_webview_window(&widgets::widget_label(id)) {
                win.eval("location.reload()").map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        "delete" => {
            let w = widgets::find(app, id).ok_or("위젯을 찾을 수 없습니다.")?;
            // 메인 스레드 밖(run은 별도 작업에서 실행)이라 확인 창을 기다려도 된다.
            let confirmed = app
                .dialog()
                .message(format!("'{}' 위젯을 삭제할까요?\n삭제하면 되돌릴 수 없습니다.", w.name))
                .title("위젯 삭제")
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom("삭제".into(), "취소".into()))
                .blocking_show();
            return if confirmed { widgets::delete(app, id) } else { Ok(()) };
        }
        _ => {}
    }
    let mut w = widgets::find(app, id).ok_or("위젯을 찾을 수 없습니다.")?;
    match action {
        "lock" => w.locked = !w.locked,
        "top" => w.layer = if w.layer == Layer::Top { Layer::Desktop } else { Layer::Top },
        "click" => w.click_through = !w.click_through,
        "mobile" => w.mobile = !w.mobile,
        "hide" => w.visible = false,
        other => match other.strip_prefix("opacity:").and_then(|p| p.parse::<f64>().ok()) {
            Some(percent) => w.opacity = percent / 100.0,
            None => return Ok(()),
        },
    }
    widgets::apply_update(app, w)?;
    widgets::notify_changed(app);
    Ok(())
}

fn build_items(app: &AppHandle, id: &str, entries: &[Entry]) -> tauri::Result<Vec<Box<dyn IsMenuItem<Wry>>>> {
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    for entry in entries {
        match entry {
            Entry::Item { action, label, checked: None } => {
                items.push(Box::new(MenuItem::with_id(app, command_id(id, action), label, true, None::<&str>)?))
            }
            Entry::Item { action, label, checked: Some(checked) } => items.push(Box::new(
                CheckMenuItem::with_id(app, command_id(id, action), label, true, *checked, None::<&str>)?,
            )),
            Entry::Separator => items.push(Box::new(PredefinedMenuItem::separator(app)?)),
            Entry::Submenu { label, items: children } => {
                let children = build_items(app, id, children)?;
                let refs: Vec<&dyn IsMenuItem<Wry>> = children.iter().map(|b| b.as_ref()).collect();
                items.push(Box::new(Submenu::with_items(app, label, true, &refs)?));
            }
        }
    }
    Ok(items)
}

/// 사진 위젯·편집 오버레이에서 우클릭하면 호출된다. 메뉴는 호출한 창의 커서 위치에 뜬다.
#[tauri::command]
pub async fn show_widget_menu(app: AppHandle, window: WebviewWindow, id: String) -> Result<(), String> {
    let entries = entries(&app, &id).ok_or("위젯을 찾을 수 없습니다.")?;
    let items = build_items(&app, &id, &entries).map_err(|e| e.to_string())?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|b| b.as_ref()).collect();
    let menu = Menu::with_items(&app, &refs).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

/// 웹 위젯: WebView2가 기본 메뉴를 띄우기 직전에 위젯 항목을 맨 아래에 붙인다.
#[cfg(windows)]
pub fn attach_web_menu(win: &WebviewWindow, id: String) {
    use webview2_com::ContextMenuRequestedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment9, ICoreWebView2_11, ICoreWebView2_2,
    };
    use windows::core::Interface;

    let app = win.app_handle().clone();
    let _ = win.with_webview(move |webview| unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else {
            return;
        };
        let (Ok(core11), Ok(env)) = (
            core.cast::<ICoreWebView2_11>(),
            core.cast::<ICoreWebView2_2>()
                .and_then(|c| c.Environment())
                .and_then(|e| e.cast::<ICoreWebView2Environment9>()),
        ) else {
            return;
        };
        let handler = ContextMenuRequestedEventHandler::create(Box::new(move |_, args| {
            let (Some(args), Some(list)) = (args, entries(&app, &id)) else {
                return Ok(());
            };
            let items = args.MenuItems()?;
            let mut count = 0;
            items.Count(&mut count)?;
            let mut all = vec![Entry::Separator];
            all.extend(list);
            append(&env, &items, count, &all, &app, &id)
        }));
        let mut token = 0;
        let _ = core11.add_ContextMenuRequested(&handler, &mut token);
    });
}

#[cfg(windows)]
unsafe fn append(
    env: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment9,
    collection: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2ContextMenuItemCollection,
    mut index: u32,
    entries: &[Entry],
    app: &AppHandle,
    id: &str,
) -> windows::core::Result<()> {
    use webview2_com::CustomItemSelectedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_CHECK_BOX, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_COMMAND,
        COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SUBMENU,
    };
    use windows::core::HSTRING;
    use windows::Win32::System::Com::IStream;

    for entry in entries {
        let item = match entry {
            Entry::Separator => env.CreateContextMenuItem(
                &HSTRING::new(),
                None::<&IStream>,
                COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR,
            )?,
            Entry::Item { action, label, checked } => {
                let kind = if checked.is_some() {
                    COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_CHECK_BOX
                } else {
                    COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_COMMAND
                };
                let item = env.CreateContextMenuItem(&HSTRING::from(label.as_str()), None::<&IStream>, kind)?;
                if let Some(checked) = checked {
                    item.SetIsChecked(*checked)?;
                }
                let (app, command) = (app.clone(), command_id(id, action));
                let handler = CustomItemSelectedEventHandler::create(Box::new(move |_, _| {
                    on_menu_event(&app, &command);
                    Ok(())
                }));
                let mut token = 0;
                item.add_CustomItemSelected(&handler, &mut token)?;
                item
            }
            Entry::Submenu { label, items } => {
                let item = env.CreateContextMenuItem(
                    &HSTRING::from(label.as_str()),
                    None::<&IStream>,
                    COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SUBMENU,
                )?;
                append(env, &item.Children()?, 0, items, app, id)?;
                item
            }
        };
        collection.InsertValueAtIndex(index, &item)?;
        index += 1;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn attach_web_menu(_win: &WebviewWindow, _id: String) {}
