//! 자동 업데이트. GitHub Releases의 latest.json에서 새 버전을 확인하고, 물어본 뒤 설치한다.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const TITLE: &str = "DeskWidget 업데이트";

/// `manual`이면 트레이에서 직접 누른 경우라 "최신 버전"·오류도 알려준다.
pub fn check(app: AppHandle, manual: bool) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run(&app, manual).await {
            if manual {
                app.dialog()
                    .message(format!("업데이트를 확인하지 못했습니다.\n\n{e}"))
                    .title(TITLE)
                    .kind(MessageDialogKind::Error)
                    .show(|_| {});
            }
        }
    });
}

async fn run(app: &AppHandle, manual: bool) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(update) = update else {
        if manual {
            app.dialog()
                .message(format!("최신 버전입니다. (현재 {})", app.package_info().version))
                .title(TITLE)
                .show(|_| {});
        }
        return Ok(());
    };

    let notes = update.body.clone().filter(|b| !b.trim().is_empty()).map(|b| format!("\n\n{b}")).unwrap_or_default();
    let install = app
        .dialog()
        .message(format!(
            "새 버전 {}이(가) 있습니다. (현재 {}){notes}\n\n지금 설치할까요? 설치 후 앱이 다시 시작됩니다.",
            update.version, update.current_version
        ))
        .title(TITLE)
        .buttons(MessageDialogButtons::OkCancelCustom("설치".into(), "나중에".into()))
        .blocking_show();
    if !install {
        return Ok(());
    }

    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    crate::widgets::persist(app)?;
    // Windows에서는 설치 프로그램을 띄운 뒤 앱이 종료되고, 설치가 끝나면 다시 실행된다.
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart();
}
