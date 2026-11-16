use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum WidgetKind {
    Web,
    /// 움직이는 사진 또는 동영상 파일 하나
    Image,
    /// 폴더 안의 사진을 차례로 보여준다. source는 폴더 경로.
    Slideshow,
    Clock,
    /// source가 메모 내용이다.
    Memo,
}

fn default_zoom() -> f64 {
    1.0
}

fn default_slide_seconds() -> u32 {
    10
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// 바탕화면에 고정: 다른 창 아래에 깔리고 Win+D에도 남는다.
    Desktop,
    /// 항상 위
    Top,
}

/// 위치와 크기는 가상 화면 기준 물리 픽셀(여러 모니터 공통 좌표)이다.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub kind: WidgetKind,
    pub source: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub opacity: f64,
    pub layer: Layer,
    pub locked: bool,
    pub click_through: bool,
    pub visible: bool,
    /// 웹 위젯을 모바일 브라우저로 위장해 모바일 페이지를 받는다.
    #[serde(default)]
    pub mobile: bool,
    /// 웹 위젯 확대 배율(1.0 = 100%).
    #[serde(default = "default_zoom")]
    pub zoom: f64,
    /// 웹 위젯 잘라 보기: 페이지를 이 폭(페이지 px)으로 그린다. 0이면 위젯 폭.
    #[serde(default)]
    pub page_width: u32,
    /// 웹 위젯 잘라 보기: 페이지의 이 지점(페이지 px)이 위젯 왼쪽 위에 오게 한다.
    #[serde(default)]
    pub offset_x: i32,
    #[serde(default)]
    pub offset_y: i32,
    /// 웹 위젯 자동 새로고침 간격(분). 0이면 끔.
    #[serde(default)]
    pub refresh_minutes: u32,
    /// 모서리 둥글기(물리 픽셀). 0이면 각진 모서리.
    #[serde(default)]
    pub radius: u32,
    /// 슬라이드쇼 사진 전환 간격(초).
    #[serde(default = "default_slide_seconds")]
    pub slide_seconds: u32,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub widgets: Vec<Widget>,
    pub autostart_initialized: bool,
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("widgets.json"))
        .map_err(|e| e.to_string())
}

pub fn load(app: &AppHandle) -> Config {
    path(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 임시 파일에 쓴 뒤 교체해서, 저장 도중 꺼져도 설정 파일이 깨지지 않게 한다.
pub fn save(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
