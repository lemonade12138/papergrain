//! panel.rs — compact floating settings panel (native Win32 controls,
//! dark/light themed). The panel is created lazily and hidden (not destroyed)
//! on close so control state survives.

use crate::config::*;
use crate::win32::*;
use crate::APP;
use std::ffi::c_void;

pub const PANEL_CLASS: &str = "PaperGrainPanelWnd";

// control ids
const IDC_COMBO_TEX: u32 = 3001;
const IDC_BTN_BROWSE: u32 = 3002;
const IDC_TRACK_OP: u32 = 3003;
const IDC_TRACK_INT: u32 = 3004;
const IDC_CHK_AUTOSTART: u32 = 3005;
const IDC_CHK_DARK: u32 = 3006;
const IDC_CHK_WATERMARK: u32 = 3007;
const IDC_BTN_HOTKEY: u32 = 3020;
const IDC_BTN_RESET: u32 = 3021;
pub const IDC_MON_FIRST: u32 = 3100; // + index

const GWL_ID: i32 = -12;
const TBM_SETRANGE: u32 = 0x0406;
const TBM_SETPOS: u32 = 0x0405;
const TBM_GETPOS: u32 = 0x0400;
const BM_SETCHECK: u32 = 0x00F1;
const BM_GETCHECK: u32 = 0x00F0;
const CBS_DROPDOWNLIST: u32 = 3;
const BS_AUTOCHECKBOX: u32 = 2;
const TBS_AUTOTICKS: u32 = 1;

// colors
fn theme_colors(dark: bool) -> (u32, u32) {
    if dark {
        (RGB(24, 26, 30), RGB(232, 232, 238)) // bg, fg
    } else {
        (RGB(250, 250, 248), RGB(28, 28, 32))
    }
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------
unsafe fn create_ctl(class: &str, text: &str, style: u32, ex: u32,
                     x: i32, y: i32, w: i32, h: i32, id: u32,
                     parent: HWND, font: HFONT) -> HWND {
    let inst = GetModuleHandleW(core::ptr::null_mut());
    let c = wide(class);
    let t = wide(text);
    let hwnd = CreateWindowExW(
        ex, c.as_ptr(), t.as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x, y, w, h, parent, id as HMENU, inst, core::ptr::null(),
    );
    if hwnd != core::ptr::null_mut() && font != core::ptr::null_mut() {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
    hwnd
}

/// (Re)build every child control of the panel. Destroys old ones first.
pub unsafe fn panel_rebuild() {
    let app = APP().get();
    let panel = app.hwnd_panel;
    if panel == core::ptr::null_mut() { return; }
    let scale = app.panel_scale();
    let s = |v: i32| -> i32 { (v as f32 * scale).round() as i32 };
    let font = app.msg_font;

    // destroy previous children (tracked list)
    for c in app.panel_ctls.drain(..) {
        if c != core::ptr::null_mut() {
            DestroyWindow(c);
        }
    }
    app.panel_mon_ctls.clear();
    let mut ctls: Vec<HWND> = Vec::new();

    let x0 = s(14);
    let w_client = s(340);
    let mut y = s(12);

    // ---- TEXTURE section ----
    ctls.push(create_ctl("STATIC", "TEXTURE", 0, 0, x0, y, s(200), s(18), 0,
                         panel, font));
    y += s(22);
    let combo = create_ctl("COMBOBOX", "", CBS_DROPDOWNLIST | WS_TABSTOP, 0,
                           x0, y, s(206), s(200), IDC_COMBO_TEX, panel, font);
    let tex_names = ["Fine paper grain", "Coarse craft paper",
                     "Notebook paper lines", "Parchment / aged paper",
                     "Custom texture"];
    for name in tex_names.iter() {
        let w = wide(name);
        SendMessageW(combo, CB_ADDSTRING, 0, w.as_ptr() as LPARAM);
    }
    let sel = TEXTURE_KINDS.iter().position(|k| *k == app.cfg.texture)
        .unwrap_or(0);
    SendMessageW(combo, CB_SETCURSEL, sel as WPARAM, 0);
    ctls.push(combo);

    let browse = create_ctl("BUTTON", "Browse...", WS_TABSTOP, 0,
                            x0 + s(216), y - s(2), s(96), s(28),
                            IDC_BTN_BROWSE, panel, font);
    ctls.push(browse);
    y += s(32);

    let file_txt = custom_file_label(&APP().get().cfg);
    let file_lbl = create_ctl("STATIC", &file_txt, 0, 0, x0, y, s(312), s(18),
                              0, panel, font);
    app.pnl_file_lbl = file_lbl;
    ctls.push(file_lbl);
    y += s(30);

    // ---- Opacity ----
    ctls.push(create_ctl("STATIC", "Opacity", 0, 0, x0, y, s(120), s(18), 0,
                         panel, font));
    let op_lbl = create_ctl("STATIC", &format!("{}%", app.cfg.opacity), 0, 0,
                            x0 + s(240), y, s(72), s(18), 0, panel, font);
    app.pnl_op_lbl = op_lbl;
    ctls.push(op_lbl);
    y += s(20);
    let track_op = create_ctl("msctls_trackbar32", "",
                              TBS_AUTOTICKS | WS_TABSTOP, 0,
                              x0, y, s(312), s(28), IDC_TRACK_OP, panel, font);
    SendMessageW(track_op, TBM_SETRANGE, 1, MAKELPARAM(10, 90));
    SendMessageW(track_op, TBM_SETPOS, 1, app.cfg.opacity as LPARAM);
    ctls.push(track_op);
    y += s(34);

    // ---- Intensity ----
    ctls.push(create_ctl("STATIC", "Grain intensity", 0, 0, x0, y, s(160),
                         s(18), 0, panel, font));
    let int_lbl = create_ctl("STATIC", &format!("{}%", app.cfg.intensity), 0, 0,
                             x0 + s(240), y, s(72), s(18), 0, panel, font);
    app.pnl_int_lbl = int_lbl;
    ctls.push(int_lbl);
    y += s(20);
    let track_int = create_ctl("msctls_trackbar32", "",
                               TBS_AUTOTICKS | WS_TABSTOP, 0,
                               x0, y, s(312), s(28), IDC_TRACK_INT, panel, font);
    SendMessageW(track_int, TBM_SETRANGE, 1, MAKELPARAM(10, 100));
    SendMessageW(track_int, TBM_SETPOS, 1, app.cfg.intensity as LPARAM);
    ctls.push(track_int);
    y += s(36);

    // ---- Monitors ----
    ctls.push(create_ctl("STATIC", "MONITORS", 0, 0, x0, y, s(200), s(18), 0,
                         panel, font));
    y += s(20);
    let monitors = crate::overlay::enum_monitors();
    for (i, mi) in monitors.iter().enumerate().take(16) {
        let label = if mi.primary {
            format!("Display {}  (primary)", i + 1)
        } else {
            format!("Display {}", i + 1)
        };
        let chk = create_ctl("BUTTON", &label, BS_AUTOCHECKBOX | WS_TABSTOP,
                             0, x0, y, s(312), s(22),
                             IDC_MON_FIRST + i as u32, panel, font);
        let on = app.cfg.monitor_enabled(&mi.device);
        SendMessageW(chk, BM_SETCHECK, if on { 1 } else { 0 }, 0);
        app.panel_mon_ctls.push(chk);
        ctls.push(chk);
        y += s(24);
    }
    y += s(8);

    // ---- Options ----
    ctls.push(create_ctl("STATIC", "OPTIONS", 0, 0, x0, y, s(200), s(18), 0,
                         panel, font));
    y += s(20);
    let chk_auto = create_ctl("BUTTON", "Run at startup",
                              BS_AUTOCHECKBOX | WS_TABSTOP, 0, x0, y, s(312),
                              s(22), IDC_CHK_AUTOSTART, panel, font);
    SendMessageW(chk_auto, BM_SETCHECK, if app.cfg.auto_start { 1 } else { 0 }, 0);
    ctls.push(chk_auto);
    y += s(24);
    let chk_wm = create_ctl("BUTTON", "Show \"CookieFilled\" watermark",
                            BS_AUTOCHECKBOX | WS_TABSTOP, 0, x0, y, s(312),
                            s(22), IDC_CHK_WATERMARK, panel, font);
    SendMessageW(chk_wm, BM_SETCHECK, if app.cfg.show_watermark { 1 } else { 0 }, 0);
    ctls.push(chk_wm);
    y += s(24);
    let chk_dark = create_ctl("BUTTON", "Dark settings theme",
                              BS_AUTOCHECKBOX | WS_TABSTOP, 0, x0, y, s(312),
                              s(22), IDC_CHK_DARK, panel, font);
    SendMessageW(chk_dark, BM_SETCHECK, if app.cfg.dark_mode { 1 } else { 0 }, 0);
    ctls.push(chk_dark);
    y += s(34);

    // ---- Hotkey ----
    ctls.push(create_ctl("STATIC", "Hotkey", 0, 0, x0, y, s(60), s(18), 0,
                         panel, font));
    let hk_lbl = create_ctl("STATIC", &app.cfg.hotkey_display(), 0, 0,
                            x0 + s(70), y, s(140), s(18), 0, panel, font);
    app.pnl_hotkey_lbl = hk_lbl;
    ctls.push(hk_lbl);
    let hk_btn = create_ctl("BUTTON", "Change...", WS_TABSTOP, 0,
                            x0 + s(216), y - s(2), s(96), s(28),
                            IDC_BTN_HOTKEY, panel, font);
    app.pnl_hotkey_btn = hk_btn;
    ctls.push(hk_btn);
    y += s(36);

    // ---- Reset + footer ----
    ctls.push(create_ctl("BUTTON", "Reset defaults", WS_TABSTOP, 0,
                         x0, y, s(130), s(28), IDC_BTN_RESET, panel, font));
    y += s(38);
    ctls.push(create_ctl("STATIC",
                         "PaperGrain 1.0.0  -  MIT License  -  CookieFilled",
                         0, 0, x0, y, s(312), s(18), 0, panel, font));
    y += s(28);

    app.panel_ctls = ctls;
    let client_h = y.max(s(120));
    let total_w = w_client + s(28);
    SetWindowPos(panel, core::ptr::null_mut(), 0, 0, total_w, client_h,
                 SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    panel_apply_theme();
    InvalidateRect(panel, core::ptr::null(), 1);
}

fn custom_file_label(cfg: &Config) -> String {
    if cfg.texture == "custom" {
        if cfg.custom_texture.is_empty() {
            "No custom texture loaded".to_string()
        } else {
            cfg.custom_texture.clone()
        }
    } else {
        "Procedural texture (no file needed)".to_string()
    }
}

// ---------------------------------------------------------------------------
// Theme / DWM dark title bar
// ---------------------------------------------------------------------------
pub unsafe fn panel_apply_theme() {
    let app = APP().get();
    let panel = app.hwnd_panel;
    if panel == core::ptr::null_mut() { return; }
    let on: i32 = if app.cfg.dark_mode { 1 } else { 0 };
    let mut r = DwmSetWindowAttribute(panel, DWMWA_USE_IMMERSIVE_DARK_MODE,
                                      &on as *const i32 as *const c_void, 4);
    if r != 0 {
        r = DwmSetWindowAttribute(panel, DWMWA_USE_IMMERSIVE_DARK_MODE_1809,
                                  &on as *const i32 as *const c_void, 4);
    }
    let (bg, _fg) = theme_colors(app.cfg.dark_mode);
    if app.brush_panel != core::ptr::null_mut() {
        DeleteObject(app.brush_panel as HGDIOBJ);
    }
    app.brush_panel = CreateSolidBrush(bg);
    InvalidateRect(panel, core::ptr::null(), 1);
}

// ---------------------------------------------------------------------------
// Show / hide
// ---------------------------------------------------------------------------
pub unsafe fn panel_show() {
    let app = APP().get();
    if app.hwnd_panel == core::ptr::null_mut() {
        let inst = GetModuleHandleW(core::ptr::null_mut());
        let cls = wide(PANEL_CLASS);
        let title = wide("PaperGrain Settings");
        app.hwnd_panel = CreateWindowExW(
            WS_EX_APPWINDOW,
            cls.as_ptr(), title.as_ptr(),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT, CW_USEDEFAULT, 356, 520,
            core::ptr::null_mut(), core::ptr::null_mut(), inst,
            core::ptr::null(),
        );
        if app.hwnd_panel == core::ptr::null_mut() { return; }
    }
    panel_rebuild();

    // position: stored location or centered on primary work area
    let app = APP().get();
    let (pw, ph) = {
        let mut r = RECT { left: 0, top: 0, right: 356, bottom: 520 };
        GetWindowRect(app.hwnd_panel, &mut r);
        (r.width(), r.height())
    };
    let mut x = app.cfg.panel_x;
    let mut y = app.cfg.panel_y;
    if x < 0 || y < 0 {
        let mut wa = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        SystemParametersInfoW(SPI_GETWORKAREA, 0,
                              &mut wa as *mut RECT as *mut c_void, 0);
        x = wa.left + (wa.width() - pw).max(0) / 2;
        y = wa.top + (wa.height() - ph).max(0) / 2;
    } else {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(pw);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(ph);
        x = x.clamp(vx, (vx + vw - pw).max(vx));
        y = y.clamp(vy, (vy + vh - ph).max(vy));
    }
    SetWindowPos(app.hwnd_panel, core::ptr::null_mut(), x, y, 0, 0,
                 SWP_NOSIZE | SWP_NOZORDER);
    ShowWindow(app.hwnd_panel, SW_SHOW);
    SetForegroundWindow(app.hwnd_panel);
}

// ---------------------------------------------------------------------------
// Panel window procedure
// ---------------------------------------------------------------------------
pub unsafe extern "system" fn panel_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            let app = APP().get();
            if app.brush_panel == core::ptr::null_mut() { return 1; }
            let hdc = wparam as HDC;
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rc);
            FillRect(hdc, &rc, app.brush_panel);
            1
        }
        WM_CTLCOLORSTATIC => {
            let app = APP().get();
            let hdc = wparam as HDC;
            let (bg, fg) = theme_colors(app.cfg.dark_mode);
            SetTextColor(hdc, fg);
            SetBkColor(hdc, bg);
            if app.brush_panel != core::ptr::null_mut() {
                return app.brush_panel as LRESULT;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_MOVE => {
            let x = (lparam as u32 & 0xFFFF) as u16 as i32;
            let y = ((lparam as u32 >> 16) & 0xFFFF) as u16 as i32;
            let app = APP().get();
            if app.hwnd_panel == hwnd {
                app.cfg.panel_x = x;
                app.cfg.panel_y = y;
                app.config_dirty = true;
            }
            0
        }
        WM_DPICHANGED => {
            panel_rebuild();
            0
        }
        WM_HSCROLL => {
            let ctrl = lparam as HWND;
            if ctrl == core::ptr::null_mut() { return 0; }
            let app = APP().get();
            let id = GetWindowLongPtrW(ctrl, GWL_ID) as u32;
            let pos = SendMessageW(ctrl, TBM_GETPOS, 0, 0) as i32;
            if id == IDC_TRACK_OP {
                let v = pos.clamp(10, 90) as u32;
                app.set_opacity(v);
                let t = wide(&format!("{}%", v));
                SetWindowTextW(app.pnl_op_lbl, t.as_ptr());
            } else if id == IDC_TRACK_INT {
                let v = pos.clamp(10, 100) as u32;
                app.set_intensity(v);
                let t = wide(&format!("{}%", v));
                SetWindowTextW(app.pnl_int_lbl, t.as_ptr());
            }
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let app = APP().get();
            if app.capture_hotkey {
                handle_hotkey_capture(wparam);
                0
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_COMMAND => {
            let notif = HIWORD(wparam as usize);
            let id = LOWORD(wparam as usize);
            let ctrl = lparam as HWND;
            if id > 0 {
                handle_command(id, notif, ctrl);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn handle_command(id: u32, notif: u32, ctrl: HWND) {
    let app = APP().get();
    match id {
        IDC_COMBO_TEX if notif == CBN_SELCHANGE => {
            let sel = SendMessageW(ctrl, CB_GETCURSEL, 0, 0) as i32;
            if sel >= 0 && (sel as usize) < TEXTURE_KINDS.len() {
                let kind = TEXTURE_KINDS[sel as usize];
                if kind == "custom" && app.cfg.custom_texture.is_empty() {
                    // no file yet — trigger the browse dialog
                    app.browse_custom_texture();
                } else {
                    app.set_texture(kind);
                }
                panel_refresh_texture();
            }
        }
        IDC_BTN_BROWSE => {
            app.browse_custom_texture();
        }
        IDC_BTN_RESET => {
            app.reset_defaults();
            panel_rebuild();
        }
        IDC_BTN_HOTKEY if notif == BN_CLICKED => {
            app.capture_hotkey = true;
            let t = wide("Press keys... (Esc cancels)");
            SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
        }
        IDC_CHK_AUTOSTART if notif == BN_CLICKED => {
            let on = SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0;
            app.set_autostart(on);
        }
        IDC_CHK_WATERMARK if notif == BN_CLICKED => {
            let on = SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0;
            app.set_watermark(on);
        }
        IDC_CHK_DARK if notif == BN_CLICKED => {
            let on = SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0;
            app.cfg.dark_mode = on;
            app.config_dirty = true;
            panel_apply_theme();
        }
        _ => {
            // monitor checkboxes
            if (IDC_MON_FIRST..IDC_MON_FIRST + 16).contains(&id)
                && notif == BN_CLICKED {
                let idx = (id - IDC_MON_FIRST) as usize;
                let monitors = crate::overlay::enum_monitors();
                if idx < monitors.len() && idx < app.panel_mon_ctls.len() {
                    let on = SendMessageW(app.panel_mon_ctls[idx],
                                          BM_GETCHECK, 0, 0) != 0;
                    app.set_monitor(&monitors[idx].device, on);
                }
            }
        }
    }
}

unsafe fn handle_hotkey_capture(wparam: WPARAM) {
    let app = APP().get();
    let vk = wparam as u32;
    if vk == VK_ESCAPE {
        app.capture_hotkey = false;
        let t = wide("Change...");
        SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
        return;
    }
    if vk == VK_SHIFT || vk == VK_CONTROL || vk == VK_MENU || vk == VK_LWIN
        || vk == VK_RWIN {
        let mods = live_mods();
        let mut s = String::new();
        if mods & MOD_CONTROL != 0 { s.push_str("Ctrl+"); }
        if mods & MOD_SHIFT != 0 { s.push_str("Shift+"); }
        if mods & MOD_ALT != 0 { s.push_str("Alt+"); }
        if mods & MOD_WIN != 0 { s.push_str("Win+"); }
        let t = wide(&format!("{}...", s));
        SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
        return;
    }
    let mods = live_mods();
    if mods == 0 {
        let t = wide("Add Ctrl / Alt / Shift...");
        SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
        return;
    }
    app.capture_hotkey = false;
    let t = wide("Change...");
    SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
    app.try_set_hotkey(mods, vk);
}

fn live_mods() -> u32 {
    unsafe {
        let mut m = 0u32;
        if GetKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0 { m |= MOD_CONTROL; }
        if GetKeyState(VK_SHIFT as i32) as u16 & 0x8000 != 0 { m |= MOD_SHIFT; }
        if GetKeyState(VK_MENU as i32) as u16 & 0x8000 != 0 { m |= MOD_ALT; }
        if GetKeyState(VK_LWIN as i32) as u16 & 0x8000 != 0 { m |= MOD_WIN; }
        if GetKeyState(VK_RWIN as i32) as u16 & 0x8000 != 0 { m |= MOD_WIN; }
        m
    }
}

/// Update combo selection + file label without rebuilding everything.
pub unsafe fn panel_refresh_texture() {
    let app = APP().get();
    for c in &app.panel_ctls {
        if *c == core::ptr::null_mut() { continue; }
        if GetWindowLongPtrW(*c, GWL_ID) as u32 == IDC_COMBO_TEX {
            let sel = TEXTURE_KINDS.iter().position(|k| *k == app.cfg.texture)
                .unwrap_or(0);
            SendMessageW(*c, CB_SETCURSEL, sel as WPARAM, 0);
        }
    }
    let file_txt = custom_file_label(&APP().get().cfg);
    let t = wide(&file_txt);
    SetWindowTextW(app.pnl_file_lbl, t.as_ptr());
}
