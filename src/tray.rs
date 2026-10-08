//! tray.rs — system tray icon, tooltip/balloon notifications and the
//! right-click context menu (rebuilt fresh on every popup so monitor lists
//! stay current).

use crate::config::*;
use crate::win32::*;
use crate::APP;
use std::ffi::c_void;

pub const TRAY_UID: u32 = 1;

// menu command ids
pub const ID_TOGGLE: u32 = 2001;
pub const ID_TEX_FIRST: u32 = 2101;
pub const ID_TEX_CUSTOM: u32 = ID_TEX_FIRST + TEXTURE_KINDS.len() as u32 - 1;
pub const ID_OPACITY_FIRST: u32 = 2201; // 10%,20%,...,100% -> 2201..2210
pub const ID_MON_FIRST: u32 = 2300; // + index
pub const ID_SETTINGS: u32 = 2401;
pub const ID_AUTOSTART: u32 = 2402;
pub const ID_THEME_FIRST: u32 = 2420;
pub const ID_ABOUT: u32 = 2404;
pub const ID_EXIT: u32 = 2405;
pub const ID_LANG_ZH: u32 = 2410;
pub const ID_LANG_EN: u32 = 2411;
pub const ID_FILTER_TOGGLE: u32 = 2450;
pub const ID_FILTER_FIRST: u32 = 2460;
pub const ID_FILTER_DEPTH_FIRST: u32 = 2470;

// ---------------------------------------------------------------------------
// Icon add / modify / remove
// ---------------------------------------------------------------------------
unsafe fn fill_nid(nid: &mut NOTIFYICONDATAW, hwnd: HWND, icon: HICON) {
    *nid = NOTIFYICONDATAW {
        cbSize: 0, hWnd: hwnd, uID: TRAY_UID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON, hIcon: icon,
        szTip: [0u16; 128], dwState: 0, dwStateMask: 0,
        szInfo: [0u16; 256], uTimeoutOrVersion: 0, szInfoTitle: [0u16; 64],
        dwInfoFlags: 0,
        guidItem: GUID { data1: 0, data2: 0, data3: 0, data4: [0; 8] },
        hBalloonIcon: core::ptr::null_mut(),
    };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
}

unsafe fn tray_tip_text() -> String {
    let app = APP().get();
    let state = if app.cfg.enabled { app.cfg.text("已开启", "ON") } else { app.cfg.text("已关闭", "OFF") };
    let filter = if app.cfg.filter_active() { app.cfg.text("开", "ON") } else { app.cfg.text("关", "OFF") };
    let status = format!("{} {} | {} {}", app.cfg.text("纸纹", "Paper"), state, app.cfg.text("滤镜", "Tint"), filter);
    let mut t = format!("PaperGrain - {} ({})", status, app.cfg.hotkey_display());
    if !app.hotkey_registered {
        t = format!("PaperGrain - {} ({})", status, app.cfg.text("快捷键不可用", "hotkey unavailable"));
    }
    if t.chars().count() > 60 {
        t = app.cfg.text("PaperGrain - 屏幕纸纹", "PaperGrain - paper texture overlay").into();
    }
    t
}

pub unsafe fn tray_add(hwnd: HWND, icon: HICON) -> bool {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    fill_nid(&mut nid, hwnd, icon);
    copy_into_buf(&mut nid.szTip, &tray_tip_text());
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

pub unsafe fn tray_update_tip(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    fill_nid(&mut nid, hwnd, core::ptr::null_mut());
    copy_into_buf(&mut nid.szTip, &tray_tip_text());
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

pub unsafe fn tray_remove(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    fill_nid(&mut nid, hwnd, core::ptr::null_mut());
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

pub unsafe fn tray_balloon(hwnd: HWND, title: &str, text: &str) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    fill_nid(&mut nid, hwnd, core::ptr::null_mut());
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_INFO;
    nid.uTimeoutOrVersion = 8000;
    copy_into_buf(&mut nid.szInfoTitle, title);
    copy_into_buf(&mut nid.szInfo, text);
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

// ---------------------------------------------------------------------------
// Context menu
// ---------------------------------------------------------------------------
unsafe fn append(menu: HMENU, flags: u32, id: usize, text: &str) {
    let w = wide(text);
    AppendMenuW(menu, flags, id, w.as_ptr());
}

#[allow(clippy::missing_safety_doc)]
#[link(name = "user32")]
unsafe extern "system" {
    fn CreatePopupMenu() -> HMENU;
    fn AppendMenuW(hMenu: HMENU, uFlags: u32, uIDNewItem: usize,
                   lpNewItem: *const u16) -> BOOL;
    fn DestroyMenu(hMenu: HMENU) -> BOOL;
    fn TrackPopupMenuEx(hMenu: HMENU, uFlags: u32, x: i32, y: i32, hWnd: HWND,
                        lptpm: *const c_void) -> i32;
    fn CheckMenuItem(hMenu: HMENU, uIDCheckItem: u32, uCheck: u32) -> u32;
    fn CheckMenuRadioItem(hMenu: HMENU, first: u32, last: u32, check: u32,
                          flags: u32) -> BOOL;
}

/// Build the full tray context menu reflecting current state.
pub unsafe fn build_tray_menu() -> HMENU {
    let app = APP().get();
    let m = CreatePopupMenu();
    let tex_labels = &app.cfg.texture_labels()[..TEXTURE_KINDS.len() - 1];

    append(m, MF_STRING | if app.cfg.enabled { MF_CHECKED } else { 0 },
           ID_TOGGLE as usize, app.cfg.text("启用纸纹", "Enable paper texture"));
    append(m, MF_STRING | if app.cfg.filter_enabled { MF_CHECKED } else { 0 },
           ID_FILTER_TOGGLE as usize, app.cfg.text("启用色彩滤镜", "Enable color filter"));
    append(m, MF_SEPARATOR, 0, "");

    // Texture submenu
    let tex = CreatePopupMenu();
    let mut sel_id = ID_TEX_FIRST;
    for (i, label) in tex_labels.iter().enumerate() {
        append(tex, MF_STRING, (ID_TEX_FIRST + i as u32) as usize, label);
    }
    append(tex, MF_STRING, ID_TEX_CUSTOM as usize, app.cfg.text("自定义图片...", "Custom..."));
    if app.cfg.texture == "custom" {
        sel_id = ID_TEX_CUSTOM;
    } else {
        for (i, _) in tex_labels.iter().enumerate() {
            let kind = TEXTURE_KINDS[i];
            if app.cfg.texture == kind { sel_id = ID_TEX_FIRST + i as u32; }
        }
    }
    CheckMenuRadioItem(tex, ID_TEX_FIRST, ID_TEX_CUSTOM, sel_id, MF_BYCOMMAND);
    append(m, MF_POPUP, tex as usize, app.cfg.text("纸张纹理", "Texture"));

    let filter = CreatePopupMenu();
    for (index, label) in app.cfg.filter_labels().iter().enumerate() {
        append(filter, MF_STRING, (ID_FILTER_FIRST + index as u32) as usize, label);
    }
    let selected = FILTER_KINDS.iter().position(|kind| *kind == app.cfg.filter_kind).unwrap_or(0) as u32;
    CheckMenuRadioItem(filter, ID_FILTER_FIRST, ID_FILTER_FIRST + 3, ID_FILTER_FIRST + selected, MF_BYCOMMAND);
    append(filter, MF_SEPARATOR, 0, "");
    let depth = CreatePopupMenu();
    append(depth, MF_STRING | MF_GRAYED, 0,
           &format!("{}: {}%", app.cfg.text("当前", "Current"), app.cfg.filter_depth));
    for index in 0..5 {
        append(depth, MF_STRING, (ID_FILTER_DEPTH_FIRST + index) as usize, &format!("{}%", index * 25));
    }
    if app.cfg.filter_depth % 25 == 0 {
        CheckMenuRadioItem(depth, ID_FILTER_DEPTH_FIRST, ID_FILTER_DEPTH_FIRST + 4,
                          ID_FILTER_DEPTH_FIRST + app.cfg.filter_depth / 25, MF_BYCOMMAND);
    }
    append(filter, MF_POPUP, depth as usize, app.cfg.text("颜色深度", "Color depth"));
    append(m, MF_POPUP, filter as usize, app.cfg.text("色彩滤镜", "Color filter"));

    // Opacity submenu
    let op = CreatePopupMenu();
    append(op, MF_STRING | MF_GRAYED, 0,
           &format!("{}: {}%", app.cfg.text("当前", "Current"), app.cfg.opacity));
    append(op, MF_SEPARATOR, 0, "");
    for i in 0..10u32 {
        let v = (i + 1) * 10;
        append(op, MF_STRING, (ID_OPACITY_FIRST + i) as usize, &format!("{}%", v));
    }
    let exact = app.cfg.opacity % 10 == 0 && (10..=100).contains(&app.cfg.opacity);
    if exact {
        let idx = app.cfg.opacity / 10 - 1;
        CheckMenuRadioItem(op, ID_OPACITY_FIRST, ID_OPACITY_FIRST + 9,
                           ID_OPACITY_FIRST + idx, MF_BYCOMMAND);
    }
    append(m, MF_POPUP, op as usize, app.cfg.text("不透明度", "Opacity"));

    // Monitors submenu
    let mon = CreatePopupMenu();
    let monitors = crate::overlay::enum_monitors();
    if monitors.is_empty() {
        append(mon, MF_STRING | MF_GRAYED, 0, app.cfg.text("（未检测到显示器）", "(no monitors)"));
    } else {
        for (i, mi) in monitors.iter().enumerate().take(16) {
            let label = if mi.primary {
                format!("{}{}", short_name(&mi.device, &app.cfg), app.cfg.text("（主显示器）", "  (primary)"))
            } else {
                short_name(&mi.device, &app.cfg)
            };
            let flags = MF_STRING
                | if app.cfg.monitor_enabled(&mi.device) { MF_CHECKED } else { 0 };
            append(mon, flags, (ID_MON_FIRST + i as u32) as usize, &label);
        }
    }
    append(m, MF_POPUP, mon as usize, app.cfg.text("显示器", "Monitors"));

    append(m, MF_SEPARATOR, 0, "");
    append(m, MF_STRING, ID_SETTINGS as usize, app.cfg.text("设置...", "Settings..."));
    let language = CreatePopupMenu();
    append(language, MF_STRING, ID_LANG_ZH as usize, LANGUAGE_LABELS[0]);
    append(language, MF_STRING, ID_LANG_EN as usize, LANGUAGE_LABELS[1]);
    let selected = if app.cfg.language == "en" { ID_LANG_EN } else { ID_LANG_ZH };
    CheckMenuRadioItem(language, ID_LANG_ZH, ID_LANG_EN, selected, MF_BYCOMMAND);
    append(m, MF_POPUP, language as usize, app.cfg.text("语言", "Language"));
    append(m, MF_STRING | if app.cfg.auto_start { MF_CHECKED } else { 0 },
           ID_AUTOSTART as usize, app.cfg.text("开机自动启动", "Run at startup"));
    let theme = CreatePopupMenu();
    for (index, label) in app.cfg.theme_labels().iter().enumerate() {
        append(theme, MF_STRING, (ID_THEME_FIRST + index as u32) as usize, label);
    }
    let selected = THEME_KINDS.iter().position(|id| *id == app.cfg.theme).unwrap_or(0) as u32;
    CheckMenuRadioItem(theme, ID_THEME_FIRST, ID_THEME_FIRST + 2, ID_THEME_FIRST + selected, MF_BYCOMMAND);
    append(m, MF_POPUP, theme as usize, app.cfg.text("外观", "Appearance"));
    append(m, MF_SEPARATOR, 0, "");
    append(m, MF_STRING, ID_ABOUT as usize, app.cfg.text("关于 PaperGrain", "About PaperGrain"));
    append(m, MF_STRING, ID_EXIT as usize, app.cfg.text("退出", "Exit"));
    m
}

fn short_name(device: &str, cfg: &Config) -> String {
    // "\\.\DISPLAY1" -> "Display 1"
    let d = device.trim_start_matches("\\\\.\\");
    let num: String = d.chars().filter(|c| c.is_ascii_digit()).collect();
    let _ = d;
    if num.is_empty() { device.to_string() } else { format!("{} {}", cfg.text("显示器", "Display"), num) }
}

/// Show the context menu at the cursor; returns the selected command id
/// (0 if dismissed).
pub unsafe fn popup_tray_menu(hwnd: HWND) -> u32 {
    let m = build_tray_menu();
    if m == core::ptr::null_mut() { return 0; }
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenuEx(
        m,
        TPM_RIGHTBUTTON | TPM_RETURNCMD,
        pt.x, pt.y, hwnd,
        core::ptr::null(),
    );
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(m);
    if cmd <= 0 { 0 } else { cmd as u32 }
}
