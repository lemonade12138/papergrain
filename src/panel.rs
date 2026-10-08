//! panel.rs — compact floating settings panel (native Win32 controls,
//! dark/light themed). The panel is created lazily and hidden (not destroyed)
//! on close so control state survives.

use crate::config::*;
use crate::win32::*;
use crate::APP;
use std::ffi::c_void;

pub const PANEL_CLASS: &str = "PaperGrainPanelWnd";
pub const WM_LANGUAGE_CHANGED: u32 = WM_APP + 3;
pub const WM_APPEARANCE_CHANGED: u32 = WM_APP + 4;
const WM_PANEL_PAGE: u32 = WM_APP + 5;

// control ids
const IDC_COMBO_TEX: u32 = 3001;
const IDC_BTN_BROWSE: u32 = 3002;
const IDC_TRACK_OP: u32 = 3003;
const IDC_TRACK_INT: u32 = 3004;
const IDC_CHK_AUTOSTART: u32 = 3005;
const IDC_COMBO_THEME: u32 = 3006;
const IDC_COMBO_LANG: u32 = 3008;
const IDC_CHK_PAPER: u32 = 3009;
const IDC_CHK_FILTER: u32 = 3010;
const IDC_TRACK_FILTER: u32 = 3011;
const IDC_FILTER_FIRST: u32 = 3030;
const IDC_BTN_HOTKEY: u32 = 3020;
const IDC_BTN_RESET: u32 = 3021;
const IDC_BTN_MORE: u32 = 3022;
const IDC_DIVIDER: u32 = 3050;
pub const IDC_MON_FIRST: u32 = 3100; // + index

const GWL_ID: i32 = -12;
const TBM_SETRANGE: u32 = 0x0406;
const TBM_SETPOS: u32 = 0x0405;
const TBM_GETPOS: u32 = 0x0400;
const BM_SETCHECK: u32 = 0x00F1;
const BM_GETCHECK: u32 = 0x00F0;
const CBS_DROPDOWNLIST: u32 = 3;
const CBS_OWNERDRAWFIXED: u32 = 0x10;
const CBS_HASSTRINGS: u32 = 0x200;
const BS_AUTOCHECKBOX: u32 = 3;
const TBS_NOTICKS: u32 = 0x10;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PanelTheme {
    pub dark: bool,
    pub high_contrast: bool,
    pub bg: COLORREF,
    pub fg: COLORREF,
    pub control: COLORREF,
    pub border: COLORREF,
    pub disabled: COLORREF,
    pub highlight: COLORREF,
    pub highlight_text: COLORREF,
}

impl PanelTheme {
    fn colors(dark: bool) -> Self {
        Self {
            dark, high_contrast: false,
            bg: if dark { RGB(28, 28, 30) } else { RGB(250, 250, 252) },
            fg: if dark { RGB(242, 242, 247) } else { RGB(29, 29, 31) },
            control: if dark { RGB(44, 44, 46) } else { RGB(255, 255, 255) },
            border: if dark { RGB(76, 76, 80) } else { RGB(218, 218, 223) },
            disabled: if dark { RGB(178, 178, 185) } else { RGB(104, 104, 112) },
            highlight: RGB(0, 102, 204), highlight_text: RGB(255, 255, 255),
        }
    }

    pub unsafe fn from_config(cfg: &Config) -> Self {
        let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let value = wide("AppsUseLightTheme");
        let mut light = 1u32;
        let mut size = 4u32;
        let ok = RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), 0x10,
                             core::ptr::null_mut(), &mut light as *mut u32 as *mut c_void, &mut size);
        let mut theme = Self::colors(cfg.theme_is_dark(ok == 0 && light == 0));
        let mut hc: HIGHCONTRASTW = std::mem::zeroed();
        hc.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
        if SystemParametersInfoW(0x42, hc.cbSize, &mut hc as *mut _ as *mut c_void, 0) != 0
            && hc.dwFlags & 1 != 0 {
            theme.high_contrast = true;
            theme.dark = false;
            theme.bg = GetSysColor(5);
            theme.fg = GetSysColor(8);
            theme.control = GetSysColor(5);
            theme.border = GetSysColor(6);
            theme.disabled = GetSysColor(17);
            theme.highlight = GetSysColor(13);
            theme.highlight_text = GetSysColor(14);
        }
        theme
    }
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------
unsafe fn create_ctl(class: &str, text: &str, style: u32, ex: u32,
                     x: i32, y: i32, w: i32, h: i32, id: u32,
                     parent: HWND, font: HFONT) -> HWND {
    let inst = GetModuleHandleW(core::ptr::null_mut());
    let style = if class == "COMBOBOX" { style | CBS_OWNERDRAWFIXED | CBS_HASSTRINGS } else { style };
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

unsafe fn slider_row(ctls: &mut Vec<HWND>, panel: HWND, font: HFONT, scale: f32,
                     y: i32, label: &str, id: u32, value: u32, min: u32, max: u32) -> HWND {
    let s = |v: i32| (v as f32 * scale).round() as i32;
    ctls.push(create_ctl("STATIC", label, 0x200, 0, s(24), y, s(92), s(32), 0, panel, font)); // SS_CENTERIMAGE
    let track = create_ctl("msctls_trackbar32", "", TBS_NOTICKS | 0x40 | WS_TABSTOP, 0,
                           s(120), y, s(176), s(32), id, panel, font);
    SendMessageW(track, TBM_SETRANGE, 1, MAKELPARAM(min, max));
    SendMessageW(track, TBM_SETPOS, 1, value as LPARAM);
    SendMessageW(track, 0x41B, s(20) as WPARAM, 0); // TBM_SETTHUMBLENGTH / TBS_FIXEDLENGTH
    SetWindowSubclass(track, slider_wndproc, 1, 0);
    ctls.push(track);
    let label = create_ctl("STATIC", &format!("{}%", value), 2 | 0x200, 0,
                           s(302), y, s(42), s(32), 0, panel, font);
    ctls.push(label);
    label
}

/// Rebuild the current page, keeping effects and preferences independent of navigation.
pub unsafe fn panel_rebuild() {
    let app = APP().get();
    let panel = app.hwnd_panel;
    if panel.is_null() { return; }
    let scale = app.panel_scale();
    let s = |v: i32| (v as f32 * scale).round() as i32;
    SetWindowTextW(panel, wide(app.cfg.text("PaperGrain 设置", "PaperGrain Settings")).as_ptr());
    for control in app.panel_ctls.drain(..) {
        if !control.is_null() { DestroyWindow(control); }
    }
    for font in app.panel_fonts {
        if !font.is_null() { DeleteObject(font as HGDIOBJ); }
    }
    for (index, (size, weight)) in [(13, 400), (16, 600), (11, 400)].iter().enumerate() {
        let mut face: LOGFONTW = std::mem::zeroed();
        face.lfHeight = -s(*size);
        face.lfWeight = *weight;
        face.lfQuality = 5;
        copy_into_buf(&mut face.lfFaceName, "Segoe UI");
        app.panel_fonts[index] = CreateFontIndirectW(&face);
    }
    let mut icons: LOGFONTW = std::mem::zeroed();
    icons.lfHeight = -s(16);
    copy_into_buf(&mut icons.lfFaceName, "Segoe MDL2 Assets");
    app.panel_fonts[3] = CreateFontIndirectW(&icons);
    let font = if app.panel_fonts[0].is_null() { app.msg_font } else { app.panel_fonts[0] };
    let heading = if app.panel_fonts[1].is_null() { font } else { app.panel_fonts[1] };
    let caption = if app.panel_fonts[2].is_null() { font } else { app.panel_fonts[2] };
    app.panel_mon_ctls.clear();
    app.pnl_file_lbl = core::ptr::null_mut();
    app.pnl_op_lbl = core::ptr::null_mut();
    app.pnl_int_lbl = core::ptr::null_mut();
    app.pnl_filter_lbl = core::ptr::null_mut();
    app.pnl_hotkey_lbl = core::ptr::null_mut();
    app.pnl_hotkey_btn = core::ptr::null_mut();
    let mut ctls = Vec::new();
    let x = s(24);
    let w = s(320);
    let mut y = s(18);

    if !app.panel_more {
        let paper = create_ctl("BUTTON", app.cfg.text("纸张", "Paper"),
                               BS_AUTOCHECKBOX | WS_TABSTOP, 0, x, y, w, s(32),
                               IDC_CHK_PAPER, panel, heading);
        SendMessageW(paper, BM_SETCHECK, app.cfg.enabled as WPARAM, 0);
        ctls.push(paper);
        y += s(44);
        let combo = create_ctl("COMBOBOX", "", CBS_DROPDOWNLIST | WS_TABSTOP, 0,
                               x, y, w, s(248), IDC_COMBO_TEX, panel, font);
        for label in app.cfg.texture_labels() {
            SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as LPARAM);
        }
        let selected = TEXTURE_KINDS.iter().position(|kind| *kind == app.cfg.texture).unwrap_or(0);
        SendMessageW(combo, CB_SETCURSEL, selected, 0);
        ctls.push(combo);
        y += s(46);
        if app.cfg.texture == "custom" {
            ctls.push(create_ctl("BUTTON", app.cfg.text("选择图片...", "Choose image..."), WS_TABSTOP, 0,
                                 x, y, w, s(30), IDC_BTN_BROWSE, panel, font));
            y += s(36);
            let label = create_ctl("STATIC", &custom_file_label(&app.cfg), 0x4000, 0,
                                   x, y, w, s(18), 0, panel, caption); // SS_PATHELLIPSIS
            app.pnl_file_lbl = label;
            ctls.push(label);
            y += s(28);
        }
        app.pnl_op_lbl = slider_row(&mut ctls, panel, font, scale, y,
                                   app.cfg.text("不透明度", "Opacity"), IDC_TRACK_OP, app.cfg.opacity, 10, 100);
        y += s(40);
        app.pnl_int_lbl = slider_row(&mut ctls, panel, font, scale, y,
                                    app.cfg.text("纹理强度", "Intensity"), IDC_TRACK_INT, app.cfg.intensity, 10, 100);
        y += s(48);
        ctls.push(create_ctl("STATIC", "", 0, 0, x, y, w, s(1).max(1), IDC_DIVIDER, panel, font));
        y += s(18);
        let filter = create_ctl("BUTTON", app.cfg.text("颜色", "Color"),
                                BS_AUTOCHECKBOX | WS_TABSTOP, 0, x, y, w, s(32),
                                IDC_CHK_FILTER, panel, heading);
        SendMessageW(filter, BM_SETCHECK, app.cfg.filter_enabled as WPARAM, 0);
        ctls.push(filter);
        y += s(42);
        for (index, label) in app.cfg.filter_labels().iter().enumerate() {
            ctls.push(create_ctl("BUTTON", label, WS_TABSTOP, 0,
                                 x + s(index as i32 * 82), y, s(74), s(58),
                                 IDC_FILTER_FIRST + index as u32, panel, font));
        }
        y += s(70);
        app.pnl_filter_lbl = slider_row(&mut ctls, panel, font, scale, y,
                                       app.cfg.text("颜色深度", "Depth"), IDC_TRACK_FILTER, app.cfg.filter_depth, 0, 100);
        y += s(56);
        ctls.push(create_ctl("BUTTON", app.cfg.text("更多设置", "More settings"), WS_TABSTOP, 0,
                             x, y, w, s(34), IDC_BTN_MORE, panel, font));
        y += s(54);
    } else {
        ctls.push(create_ctl("BUTTON", app.cfg.text("返回", "Back"), WS_TABSTOP, 0,
                             x, y, s(72), s(32), IDC_BTN_MORE, panel, font));
        ctls.push(create_ctl("STATIC", app.cfg.text("更多设置", "More settings"), 0, 0,
                             x + s(96), y + s(5), s(224), s(24), 0, panel, heading));
        y += s(50);
        for (label, id, labels, selected) in [
            (app.cfg.text("语言", "Language"), IDC_COMBO_LANG, &LANGUAGE_LABELS[..],
             LANGUAGE_KINDS.iter().position(|kind| *kind == app.cfg.language).unwrap_or(0)),
            (app.cfg.text("外观", "Appearance"), IDC_COMBO_THEME, app.cfg.theme_labels(),
             THEME_KINDS.iter().position(|kind| *kind == app.cfg.theme).unwrap_or(0)),
        ] {
            ctls.push(create_ctl("STATIC", label, 0, 0, x, y + s(5), s(86), s(22), 0, panel, font));
            let combo = create_ctl("COMBOBOX", "", CBS_DROPDOWNLIST | WS_TABSTOP, 0,
                                   x + s(104), y, s(216), s(128), id, panel, font);
            for label in labels { SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as LPARAM); }
            SendMessageW(combo, CB_SETCURSEL, selected, 0);
            ctls.push(combo);
            y += s(44);
        }
        let auto = create_ctl("BUTTON", app.cfg.text("开机启动", "Run at startup"),
                              BS_AUTOCHECKBOX | WS_TABSTOP, 0, x, y, w, s(34),
                              IDC_CHK_AUTOSTART, panel, font);
        SendMessageW(auto, BM_SETCHECK, app.cfg.auto_start as WPARAM, 0);
        ctls.push(auto);
        y += s(48);
        ctls.push(create_ctl("STATIC", "", 0, 0, x, y, w, s(1).max(1), IDC_DIVIDER, panel, font));
        y += s(18);
        ctls.push(create_ctl("STATIC", app.cfg.text("显示器", "Displays"), 0, 0,
                             x, y, w, s(24), 0, panel, heading));
        y += s(30);
        for (index, monitor) in crate::overlay::enum_monitors().iter().enumerate().take(16) {
            let label = format!("{} {}{}", app.cfg.text("显示器", "Display"), index + 1,
                                if monitor.primary { app.cfg.text("（主屏幕）", " (primary)") } else { "" });
            let control = create_ctl("BUTTON", &label, BS_AUTOCHECKBOX | WS_TABSTOP, 0,
                                     x, y, w, s(34), IDC_MON_FIRST + index as u32, panel, font);
            SendMessageW(control, BM_SETCHECK, app.cfg.monitor_enabled(&monitor.device) as WPARAM, 0);
            app.panel_mon_ctls.push(control);
            ctls.push(control);
            y += s(38);
        }
        y += s(12);
        ctls.push(create_ctl("STATIC", app.cfg.text("快捷键", "Hotkey"), 0, 0,
                             x, y, w, s(20), 0, panel, font));
        y += s(26);
        app.pnl_hotkey_lbl = create_ctl("STATIC", &app.cfg.hotkey_display(), 0, 0,
                                        x, y + s(5), s(204), s(22), 0, panel, font);
        ctls.push(app.pnl_hotkey_lbl);
        app.pnl_hotkey_btn = create_ctl("BUTTON", app.cfg.text("修改...", "Change..."), WS_TABSTOP, 0,
                                        x + s(216), y, s(104), s(30), IDC_BTN_HOTKEY, panel, font);
        ctls.push(app.pnl_hotkey_btn);
        y += s(48);
        ctls.push(create_ctl("BUTTON", app.cfg.text("恢复默认", "Reset defaults"), WS_TABSTOP, 0,
                             x, y, w, s(32), IDC_BTN_RESET, panel, font));
        y += s(48);
        ctls.push(create_ctl("STATIC", "PaperGrain 1.0.0  |  MIT  |  CookieFilled", 1, 0,
                             x, y, w, s(20), 0, panel, caption));
        y += s(34);
    }
    app.panel_ctls = ctls;
    let mut window: RECT = std::mem::zeroed();
    let mut client: RECT = std::mem::zeroed();
    GetWindowRect(panel, &mut window);
    GetClientRect(panel, &mut client);
    let frame_h = (window.height() - client.height()).max(0);
    let mut monitor: MONITORINFOEXW = std::mem::zeroed();
    monitor.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let limit = if GetMonitorInfoW(MonitorFromWindow(panel, 2), &mut monitor) != 0 {
        (monitor.rcWork.height() - frame_h - s(24)).max(s(200))
    } else { y };
    let viewport = y.min(limit);
    app.panel_scroll = 0;
    app.panel_content_height = y;
    let info = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: 7, nMin: 0, nMax: y - 1, nPage: viewport as u32, nPos: 0, nTrackPos: 0 };
    SetScrollInfo(panel, 1, &info, 1);
    GetWindowRect(panel, &mut window);
    GetClientRect(panel, &mut client);
    let frame_w = (window.width() - client.width()).max(0);
    SetWindowPos(panel, core::ptr::null_mut(), 0, 0, s(368) + frame_w, viewport + frame_h,
                 SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    panel_apply_theme();
    panel_raise();
    InvalidateRect(panel, core::ptr::null(), 1);
}

fn custom_file_label(cfg: &Config) -> String {
    if cfg.texture == "custom" {
        if cfg.custom_texture.is_empty() {
            cfg.text("尚未选择自定义图片", "No custom texture loaded").to_string()
        } else {
            cfg.custom_texture.clone()
        }
    } else {
        cfg.text("内置纸纹，无需图片文件", "Procedural texture (no file needed)").to_string()
    }
}

// ---------------------------------------------------------------------------
// Theme / DWM dark title bar
// ---------------------------------------------------------------------------
pub unsafe fn panel_apply_theme() {
    let theme = PanelTheme::from_config(&APP().get().cfg);
    APP().get().panel_theme = theme;
    let panel = APP().get().hwnd_panel;
    if panel == core::ptr::null_mut() { return; }
    let on: i32 = if theme.dark { 1 } else { 0 };
    let r = DwmSetWindowAttribute(panel, DWMWA_USE_IMMERSIVE_DARK_MODE,
                                      &on as *const i32 as *const c_void, 4);
    if r != 0 {
        DwmSetWindowAttribute(panel, DWMWA_USE_IMMERSIVE_DARK_MODE_1809,
                                  &on as *const i32 as *const c_void, 4);
    }
    let app = APP().get();
    if app.brush_panel != core::ptr::null_mut() {
        DeleteObject(app.brush_panel as HGDIOBJ);
    }
    if app.brush_control != core::ptr::null_mut() {
        DeleteObject(app.brush_control as HGDIOBJ);
    }
    app.brush_panel = CreateSolidBrush(theme.bg);
    app.brush_control = CreateSolidBrush(theme.control);
    let controls = app.panel_ctls.clone();
    let selected = THEME_KINDS.iter().position(|id| *id == app.cfg.theme).unwrap_or(0);
    for (index, &control) in controls.iter().enumerate() {
        let id = GetWindowLongPtrW(control, GWL_ID) as u32;
        let combo = [IDC_COMBO_LANG, IDC_COMBO_TEX, IDC_COMBO_THEME].contains(&id);
        let name = wide(if combo { "DarkMode_CFD" } else { "DarkMode_Explorer" });
        SetWindowTheme(control, if theme.dark { name.as_ptr() } else { core::ptr::null() },
                       core::ptr::null());
        if id == IDC_COMBO_THEME { SendMessageW(control, CB_SETCURSEL, selected, 0); }
        InvalidateRect(control, core::ptr::null(), 1);
        if [IDC_TRACK_OP, IDC_TRACK_INT, IDC_TRACK_FILTER].contains(&id) {
            // Move the native hit target with the drawing, using its final themed geometry.
            UpdateWindow(control);
            let mut thumb: RECT = std::mem::zeroed();
            let mut label: RECT = std::mem::zeroed();
            let mut bounds: RECT = std::mem::zeroed();
            SendMessageW(control, 0x419, 0, &mut thumb as *mut RECT as LPARAM);
            GetWindowRect(controls[index - 1], &mut label);
            GetWindowRect(control, &mut bounds);
            let mut origin = POINT { x: 0, y: 0 };
            ClientToScreen(panel, &mut origin);
            let top = (label.top + label.bottom) / 2 - (thumb.top + thumb.bottom) / 2;
            SetWindowPos(control, core::ptr::null_mut(), bounds.left - origin.x, top - origin.y, 0, 0,
                         SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    InvalidateRect(panel, core::ptr::null(), 1);
}

pub unsafe fn panel_refresh_system_theme() {
    let theme = PanelTheme::from_config(&APP().get().cfg);
    if theme != APP().get().panel_theme {
        APP().get().panel_theme = theme;
        if APP().get().hwnd_panel != core::ptr::null_mut() {
            PostMessageW(APP().get().hwnd_panel, WM_APPEARANCE_CHANGED, 0, 0);
        }
    }
}

unsafe fn fill_shape(dc: HDC, rect: &RECT, color: COLORREF, radius: i32, circle: bool) {
    let mut graphics = core::ptr::null_mut();
    if APP().get().gdiplus_token != 0 && GdipCreateFromHDC(dc, &mut graphics) == 0 {
        let mut brush = core::ptr::null_mut();
        let argb = 0xFF000000 | ((color & 0xFF) << 16) | (color & 0xFF00) | ((color >> 16) & 0xFF);
        let status = if GdipCreateSolidFill(argb, &mut brush) == 0 {
            GdipSetSmoothingMode(graphics, 5); // SmoothingModeAntiAlias8x8
            GdipSetPixelOffsetMode(graphics, 4); // PixelOffsetModeHalf
            let x = rect.left as f32;
            let y = rect.top as f32;
            let w = rect.width() as f32;
            let h = rect.height() as f32;
            let status = if circle {
                // Independent DPI rounding must never turn a circle into an ellipse.
                let diameter = w.min(h);
                GdipFillEllipse(graphics, brush, x + (w - diameter) / 2.0,
                                y + (h - diameter) / 2.0, diameter, diameter)
            } else {
                let mut path = core::ptr::null_mut();
                if GdipCreatePath(0, &mut path) == 0 {
                    let d = (radius as f32).min(w).min(h).max(1.0);
                    for (left, top, start) in [(x, y, 180.0), (x + w - d, y, 270.0),
                                               (x + w - d, y + h - d, 0.0), (x, y + h - d, 90.0)] {
                        GdipAddPathArc(path, left, top, d, d, start, 90.0);
                    }
                    GdipClosePathFigure(path);
                    let status = GdipFillPath(graphics, brush, path);
                    GdipDeletePath(path);
                    status
                } else { -1 }
            };
            GdipDeleteBrush(brush);
            status
        } else { -1 };
        GdipDeleteGraphics(graphics);
        if status == 0 { return; }
    }
    let brush = CreateSolidBrush(color);
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, GetStockObject(8)); // NULL_PEN
    if circle { Ellipse(dc, rect.left, rect.top, rect.right, rect.bottom); }
    else { RoundRect(dc, rect.left, rect.top, rect.right, rect.bottom, radius, radius); }
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    DeleteObject(brush as HGDIOBJ);
}

#[derive(Default)]
struct SliderPaintProbe { paints: usize, clipped: usize }

unsafe extern "system" fn slider_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM,
                                          lparam: LPARAM, id: usize, data: usize) -> LRESULT {
    if msg == WM_PAINT && !APP().get().panel_theme.high_contrast {
        // Expand before the native BeginPaint clips the DC to its narrower thumb rectangle.
        InvalidateRect(hwnd, core::ptr::null(), 0);
        if APP().get().smoke && data != 0 {
            let probe = &mut *(data as *mut SliderPaintProbe);
            let mut client: RECT = std::mem::zeroed();
            let mut update: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client);
            GetUpdateRect(hwnd, &mut update, 0);
            probe.paints += 1;
            if update.left != client.left || update.top != client.top
                || update.right != client.right || update.bottom != client.bottom { probe.clipped += 1; }
        }
    }
    if msg == 0x82 { RemoveWindowSubclass(hwnd, slider_wndproc, id); } // WM_NCDESTROY
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

unsafe fn draw_slider(draw: &NMCUSTOMDRAW) -> LRESULT {
    if APP().get().panel_theme.high_contrast { return 0; }
    // Native layout must finish first so painting agrees with the real thumb hit target.
    if draw.dwDrawStage == 1 { return 16; } // CDRF_NOTIFYPOSTPAINT
    if draw.dwDrawStage != 2 { return 0; } // CDDS_POSTPAINT
    let dc = draw.hdc;
    let scale = APP().get().panel_scale();
    let s = |v: i32| (v as f32 * scale).round() as i32;
    let theme = APP().get().panel_theme;
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(draw.hdr.hwndFrom, &mut client);
    FillRect(dc, &client, APP().get().brush_panel);
    let mut thumb: RECT = std::mem::zeroed();
    SendMessageW(draw.hdr.hwndFrom, 0x419, 0, &mut thumb as *mut RECT as LPARAM);
    let center = (thumb.left + thumb.right) / 2;
    let middle = (thumb.top + thumb.bottom) / 2;
    let mut channel: RECT = std::mem::zeroed();
    SendMessageW(draw.hdr.hwndFrom, 0x41A, 0, &mut channel as *mut RECT as LPARAM);
    let track = RECT { left: channel.left, top: middle - s(2), right: channel.right, bottom: middle + s(2) };
    fill_shape(dc, &track, theme.border, s(4), false);
    let filled = RECT { right: center.min(track.right), ..track };
    if filled.right > filled.left { fill_shape(dc, &filled, theme.highlight, s(4), false); }
    let outer = RECT { left: center - s(9), top: middle - s(9), right: center + s(9), bottom: middle + s(9) };
    fill_shape(dc, &outer, theme.border, 0, true);
    let inner = RECT { left: outer.left + s(1), top: outer.top + s(1), right: outer.right - s(1), bottom: outer.bottom - s(1) };
    fill_shape(dc, &inner, RGB(255, 255, 255), 0, true);
    if GetFocus() == draw.hdr.hwndFrom && SendMessageW(draw.hdr.hwndFrom, 0x129, 0, 0) & 1 == 0 {
        DrawFocusRect(dc, &client);
    }
    0
}

unsafe fn draw_button(draw: &NMCUSTOMDRAW) -> LRESULT {
    let control = draw.hdr.hwndFrom;
    let style = GetWindowLongPtrW(control, -16) as u32;
    let checkbox = style & 0xF == BS_AUTOCHECKBOX;
    let id = draw.hdr.idFrom as u32;
    let swatch = (IDC_FILTER_FIRST..IDC_FILTER_FIRST + FILTER_KINDS.len() as u32).contains(&id);
    if !checkbox && !swatch && ![IDC_BTN_BROWSE, IDC_BTN_HOTKEY, IDC_BTN_RESET, IDC_BTN_MORE].contains(&id) { return 0; }
    let theme = APP().get().panel_theme;
    if (theme.high_contrast && !swatch) || draw.dwDrawStage != 1 { return 0; }
    let dc = draw.hdc;
    let saved = SaveDC(dc);
    let scale = APP().get().panel_scale();
    let s = |v: i32| (v as f32 * scale).round() as i32;
    let mut rect = draw.rc;
    let disabled = draw.uItemState & 4 != 0;
    let pressed = draw.uItemState & 1 != 0;
    let hot = draw.uItemState & 0x40 != 0;
    FillRect(dc, &rect, APP().get().brush_panel);
    if checkbox {
        let checked = SendMessageW(control, BM_GETCHECK, 0, 0) != 0;
        let top = rect.top + (rect.height() - s(24)) / 2;
        let rail = RECT { left: rect.right - s(42), top, right: rect.right, bottom: top + s(24) };
        let color = if disabled { theme.border } else if checked { RGB(35, 166, 77) } else { theme.border };
        fill_shape(dc, &rail, color, s(24), false);
        let left = if checked { rail.right - s(22) } else { rail.left + s(2) };
        let knob = RECT { left, top: rail.top + s(2), right: left + s(20), bottom: rail.bottom - s(2) };
        fill_shape(dc, &knob, RGB(255, 255, 255), 0, true);
        rect.right = rail.left - s(12);
    } else if swatch {
        if hot || pressed { fill_shape(dc, &rect, theme.control, s(8), false); }
        let index = (id - IDC_FILTER_FIRST) as usize;
        let selected = APP().get().cfg.filter_kind == FILTER_KINDS[index];
        let color = FILTER_COLORS.get(index).copied().unwrap_or(APP().get().cfg.filter_custom_color);
        let center = (rect.left + rect.right) / 2;
        let circle = RECT { left: center - s(15), top: rect.top + s(3), right: center + s(15), bottom: rect.top + s(33) };
        fill_shape(dc, &circle, if selected { theme.highlight } else { theme.border }, 0, true);
        let gap = if selected { 3 } else { 1 };
        let inside = RECT { left: circle.left + s(gap), top: circle.top + s(gap),
                            right: circle.right - s(gap), bottom: circle.bottom - s(gap) };
        if selected {
            fill_shape(dc, &inside, theme.bg, 0, true);
            let fill = RECT { left: inside.left + s(2), top: inside.top + s(2), right: inside.right - s(2), bottom: inside.bottom - s(2) };
            fill_shape(dc, &fill, color, 0, true);
        } else { fill_shape(dc, &inside, color, 0, true); }
        rect.top += s(36);
    } else {
        let button_bg = if pressed { theme.border } else if hot { theme.control } else { theme.bg };
        fill_shape(dc, &rect, button_bg, s(8), false);
        let glyph = match id {
            IDC_BTN_BROWSE => 0xE8B7,
            IDC_BTN_HOTKEY => 0xE70F,
            IDC_BTN_RESET => 0xE777,
            _ if APP().get().panel_more => 0xE72B,
            _ => 0xE713,
        };
        let mut icon_rect = RECT { left: rect.left + s(4), right: rect.left + s(24), ..rect };
        SelectObject(dc, APP().get().panel_fonts[3] as HGDIOBJ);
        SetBkMode(dc, TRANSPARENT_BK);
        SetTextColor(dc, theme.disabled);
        DrawTextW(dc, [glyph, 0].as_ptr(), 1, &mut icon_rect, DT_SINGLELINE | 4 | 1);
        rect.left += s(32);
        rect.right -= s(4);
        if id == IDC_BTN_MORE && !APP().get().panel_more {
            let mut chevron = RECT { left: rect.right - s(18), ..rect };
            SelectObject(dc, APP().get().panel_fonts[0] as HGDIOBJ);
            DrawTextW(dc, wide(">").as_ptr(), 1, &mut chevron, DT_SINGLELINE | 4 | 1);
            rect.right -= s(24);
        }
    }
    let mut label = [0u16; 256];
    let len = GetWindowTextW(control, label.as_mut_ptr(), label.len() as i32);
    let font = SendMessageW(control, 0x31, 0, 0); // WM_GETFONT
    SelectObject(dc, font as HGDIOBJ);
    SetBkMode(dc, TRANSPARENT_BK);
    SetTextColor(dc, if disabled { theme.disabled } else { theme.fg });
    DrawTextW(dc, label.as_ptr(), len, &mut rect, DT_SINGLELINE | DT_NOPREFIX | 4 | if swatch { 1 } else { 0 });
    if draw.uItemState & 0x10 != 0 {
        let mut focus = draw.rc;
        focus.top += s(2); focus.bottom -= s(2);
        DrawFocusRect(dc, &focus);
    }
    RestoreDC(dc, saved);
    4
}

unsafe fn draw_combo_item(item: &DRAWITEMSTRUCT) {
    if item.itemID == u32::MAX { return; }
    let theme = APP().get().panel_theme;
    let selected = item.itemState & 1 != 0;
    let saved = SaveDC(item.hDC);
    let brush = CreateSolidBrush(if selected { theme.highlight } else { theme.control });
    FillRect(item.hDC, &item.rcItem, brush);
    DeleteObject(brush as HGDIOBJ);
    let text = match item.CtlID {
        IDC_COMBO_LANG => LANGUAGE_LABELS.get(item.itemID as usize).copied(),
        IDC_COMBO_TEX => APP().get().cfg.texture_labels().get(item.itemID as usize).copied(),
        IDC_COMBO_THEME => APP().get().cfg.theme_labels().get(item.itemID as usize).copied(),
        _ => None,
    };
    if let Some(text) = text {
        let mut rect = item.rcItem;
        rect.left += (5.0 * APP().get().panel_scale()).round() as i32;
        SelectObject(item.hDC, SendMessageW(item.hwndItem, 0x31, 0, 0) as HGDIOBJ);
        SetBkMode(item.hDC, TRANSPARENT_BK);
        SetTextColor(item.hDC, if item.itemState & 4 != 0 { theme.disabled }
                     else if selected { theme.highlight_text } else { theme.fg });
        DrawTextW(item.hDC, wide(text).as_ptr(), -1, &mut rect, DT_SINGLELINE | DT_NOPREFIX | 4);
        if item.itemState & 0x10 != 0 { DrawFocusRect(item.hDC, &item.rcItem); }
    }
    RestoreDC(item.hDC, saved);
}

// ---------------------------------------------------------------------------
// Show / hide
// ---------------------------------------------------------------------------
pub unsafe fn panel_raise() {
    let panel = APP().get().hwnd_panel;
    if !panel.is_null() && IsWindowVisible(panel) != 0 {
        SetWindowPos(panel, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

pub unsafe fn panel_keep_focus_visible() {
    let app = APP().get();
    let focused = GetFocus();
    if !app.panel_ctls.contains(&focused) { return; }
    let mut control: RECT = std::mem::zeroed();
    let mut client: RECT = std::mem::zeroed();
    GetWindowRect(focused, &mut control);
    GetClientRect(app.hwnd_panel, &mut client);
    let mut origin = POINT { x: 0, y: 0 };
    ClientToScreen(app.hwnd_panel, &mut origin);
    let top = origin.y;
    let bottom = top + client.height();
    let target = if control.top < top { app.panel_scroll + control.top - top }
                 else if control.bottom > bottom { app.panel_scroll + control.bottom - bottom }
                 else { return; };
    scroll_panel_to(target);
}

unsafe fn scroll_panel_to(target: i32) {
    let app = APP().get();
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(app.hwnd_panel, &mut client);
    let next = target.clamp(0, (app.panel_content_height - client.height()).max(0));
    let old = app.panel_scroll;
    if next == old { return; }
    app.panel_scroll = next;
    let info = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: 4, nMin: 0, nMax: 0, nPage: 0, nPos: next, nTrackPos: 0 };
    SetScrollInfo(app.hwnd_panel, 1, &info, 1);
    ScrollWindowEx(app.hwnd_panel, 0, old - next, core::ptr::null(), core::ptr::null(),
                   core::ptr::null_mut(), core::ptr::null_mut(), 1 | 2 | 4);
}

unsafe fn panel_create() {
    let app = APP().get();
    if app.hwnd_panel == core::ptr::null_mut() {
        let inst = GetModuleHandleW(core::ptr::null_mut());
        let cls = wide(PANEL_CLASS);
        let title = wide(app.cfg.text("PaperGrain 设置", "PaperGrain Settings"));
        app.hwnd_panel = CreateWindowExW(
            WS_EX_APPWINDOW,
            cls.as_ptr(), title.as_ptr(),
            WS_CAPTION | WS_SYSMENU | 0x200000, // WS_VSCROLL (hidden when not needed)
            CW_USEDEFAULT, CW_USEDEFAULT, 356, 520,
            core::ptr::null_mut(), core::ptr::null_mut(), inst,
            core::ptr::null(),
        );
        if app.hwnd_panel == core::ptr::null_mut() { return; }
    }
    panel_rebuild();
}

pub unsafe fn panel_show() {
    panel_create();
    if APP().get().hwnd_panel.is_null() { return; }

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
    panel_raise();
    SetForegroundWindow(app.hwnd_panel);
}

// ---------------------------------------------------------------------------
// Panel window procedure
// ---------------------------------------------------------------------------
pub unsafe extern "system" fn panel_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        0x115 | 0x20A => { // WM_VSCROLL / WM_MOUSEWHEEL
            let app = APP().get();
            let mut info: SCROLLINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
            info.fMask = 0x17;
            GetScrollInfo(hwnd, 1, &mut info);
            let old = app.panel_scroll;
            let step = (32.0 * app.panel_scale()).round() as i32;
            let next = if msg == 0x20A {
                old - (HIWORD(wparam) as u16 as i16 as i32) * step / 120
            } else {
                match LOWORD(wparam) {
                    0 => old - step, 1 => old + step,
                    2 => old - info.nPage as i32, 3 => old + info.nPage as i32,
                    4 | 5 => info.nTrackPos, 6 => 0, 7 => info.nMax,
                    _ => old,
                }
            }.clamp(0, (info.nMax - info.nPage as i32 + 1).max(0));
            scroll_panel_to(next);
            0
        }
        WM_PANEL_PAGE => {
            panel_rebuild();
            if let Some(control) = APP().get().panel_ctls.iter().find(|c| GetWindowLongPtrW(**c, GWL_ID) as u32 == IDC_BTN_MORE) {
                SetFocus(*control);
            }
            0
        }
        WM_LANGUAGE_CHANGED => {
            panel_rebuild();
            0
        }
        WM_APPEARANCE_CHANGED => {
            panel_apply_theme();
            0
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE => {
            panel_refresh_system_theme();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NOTIFY if lparam != 0 => {
            let hdr = &*(lparam as *const NMHDR);
            if hdr.code == (-12i32) as u32 { // NM_CUSTOMDRAW
                if [IDC_TRACK_OP, IDC_TRACK_INT, IDC_TRACK_FILTER].contains(&(hdr.idFrom as u32)) {
                    return draw_slider(&*(lparam as *const NMCUSTOMDRAW));
                }
                return draw_button(&*(lparam as *const NMCUSTOMDRAW));
            }
            0
        }
        WM_MEASUREITEM if lparam != 0 => {
            let item = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            if item.CtlType == 3 {
                item.itemHeight = (22.0 * APP().get().panel_scale()).round() as u32;
                return 1;
            }
            0
        }
        WM_DRAWITEM if lparam != 0 => {
            let item = &*(lparam as *const DRAWITEMSTRUCT);
            if item.CtlType == 3 { draw_combo_item(item); return 1; }
            0
        }
        WM_ERASEBKGND => {
            let app = APP().get();
            if app.brush_panel == core::ptr::null_mut() { return 1; }
            let hdc = wparam as HDC;
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rc);
            FillRect(hdc, &rc, app.brush_panel);
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let app = APP().get();
            let hdc = wparam as HDC;
            let theme = app.panel_theme;
            let input = msg == WM_CTLCOLOREDIT || msg == WM_CTLCOLORLISTBOX;
            if GetWindowLongPtrW(lparam as HWND, GWL_ID) as u32 == IDC_DIVIDER {
                SetDCBrushColor(hdc, theme.border);
                return GetStockObject(18) as LRESULT; // DC_BRUSH
            }
            SetTextColor(hdc, theme.fg);
            SetBkColor(hdc, if input { theme.control } else { theme.bg });
            if input && !app.brush_control.is_null() { return app.brush_control as LRESULT; }
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
                let v = pos.clamp(10, 100) as u32;
                app.set_opacity(v);
                let t = wide(&format!("{}%", v));
                SetWindowTextW(app.pnl_op_lbl, t.as_ptr());
            } else if id == IDC_TRACK_INT {
                let v = pos.clamp(10, 100) as u32;
                app.set_intensity(v);
                let t = wide(&format!("{}%", v));
                SetWindowTextW(app.pnl_int_lbl, t.as_ptr());
            } else if id == IDC_TRACK_FILTER {
                app.set_filter_depth(pos.clamp(0, 100) as u32);
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
        IDC_BTN_MORE if notif == BN_CLICKED => {
            app.capture_hotkey = false;
            app.panel_more = !app.panel_more;
            PostMessageW(app.hwnd_panel, WM_PANEL_PAGE, 0, 0);
        }
        IDC_COMBO_LANG if notif == CBN_SELCHANGE => {
            let selected = SendMessageW(ctrl, CB_GETCURSEL, 0, 0) as usize;
            if let Some(language) = LANGUAGE_KINDS.get(selected) {
                app.set_language(language);
            }
        }
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
            app.capture_hotkey = false;
            PostMessageW(app.hwnd_panel, WM_PANEL_PAGE, 0, 0);
        }
        IDC_BTN_HOTKEY if notif == BN_CLICKED => {
            app.capture_hotkey = true;
            let t = wide(app.cfg.text("按下按键...", "Press keys..."));
            SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
            let t = wide(app.cfg.text("Esc 取消", "Esc cancels"));
            SetWindowTextW(app.pnl_hotkey_lbl, t.as_ptr());
        }
        IDC_CHK_AUTOSTART if notif == BN_CLICKED => {
            let on = SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0;
            app.set_autostart(on);
        }
        IDC_CHK_PAPER if notif == BN_CLICKED => {
            app.set_enabled(SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0);
        }
        IDC_CHK_FILTER if notif == BN_CLICKED => {
            app.set_filter_enabled(SendMessageW(ctrl, BM_GETCHECK, 0, 0) != 0);
        }
        IDC_COMBO_THEME if notif == CBN_SELCHANGE => {
            let selected = SendMessageW(ctrl, CB_GETCURSEL, 0, 0) as usize;
            if let Some(theme) = THEME_KINDS.get(selected) { app.set_theme(theme); }
        }
        _ => {
            if (IDC_FILTER_FIRST..IDC_FILTER_FIRST + FILTER_KINDS.len() as u32).contains(&id)
                && notif == BN_CLICKED {
                let kind = FILTER_KINDS[(id - IDC_FILTER_FIRST) as usize];
                if kind == "custom" { choose_filter_color(); } else { app.set_filter_kind(kind); }
                return;
            }
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
        let t = wide(app.cfg.text("修改...", "Change..."));
        SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
        let t = wide(&app.cfg.hotkey_display());
        SetWindowTextW(app.pnl_hotkey_lbl, t.as_ptr());
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
        SetWindowTextW(app.pnl_hotkey_lbl, t.as_ptr());
        return;
    }
    let mods = live_mods();
    if mods == 0 {
        let t = wide(app.cfg.text("需要 Ctrl/Alt/Shift", "Use Ctrl/Alt/Shift"));
        SetWindowTextW(app.pnl_hotkey_lbl, t.as_ptr());
        return;
    }
    app.capture_hotkey = false;
    let t = wide(app.cfg.text("修改...", "Change..."));
    SetWindowTextW(app.pnl_hotkey_btn, t.as_ptr());
    app.try_set_hotkey(mods, vk);
    let t = wide(&app.cfg.hotkey_display());
    SetWindowTextW(app.pnl_hotkey_lbl, t.as_ptr());
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
    if !app.pnl_file_lbl.is_null() { SetWindowTextW(app.pnl_file_lbl, t.as_ptr()); }
    if !app.panel_more && !app.hwnd_panel.is_null()
        && (!app.pnl_file_lbl.is_null()) != (app.cfg.texture == "custom") {
        PostMessageW(app.hwnd_panel, WM_PANEL_PAGE, 0, 0);
    }
}

pub unsafe fn panel_refresh_effects() {
    let controls = APP().get().panel_ctls.clone();
    let monitors = if APP().get().panel_more { crate::overlay::enum_monitors() } else { Vec::new() };
    for control in controls {
        match GetWindowLongPtrW(control, GWL_ID) as u32 {
            IDC_CHK_PAPER => { SendMessageW(control, BM_SETCHECK, APP().get().cfg.enabled as WPARAM, 0); }
            IDC_CHK_FILTER => { SendMessageW(control, BM_SETCHECK, APP().get().cfg.filter_enabled as WPARAM, 0); }
            IDC_TRACK_OP => { SendMessageW(control, TBM_SETPOS, 1, APP().get().cfg.opacity as LPARAM); }
            IDC_TRACK_FILTER => { SendMessageW(control, TBM_SETPOS, 1, APP().get().cfg.filter_depth as LPARAM); }
            IDC_CHK_AUTOSTART => { SendMessageW(control, BM_SETCHECK, APP().get().cfg.auto_start as WPARAM, 0); }
            id if (IDC_MON_FIRST..IDC_MON_FIRST + 16).contains(&id) => {
                if let Some(monitor) = monitors.get((id - IDC_MON_FIRST) as usize) {
                    SendMessageW(control, BM_SETCHECK, APP().get().cfg.monitor_enabled(&monitor.device) as WPARAM, 0);
                }
            }
            id if (IDC_FILTER_FIRST..IDC_FILTER_FIRST + FILTER_KINDS.len() as u32).contains(&id) => {
                InvalidateRect(control, core::ptr::null(), 1);
            }
            _ => {}
        }
    }
    let label = APP().get().pnl_filter_lbl;
    if !label.is_null() { SetWindowTextW(label, wide(&format!("{}%", APP().get().cfg.filter_depth)).as_ptr()); }
    let label = APP().get().pnl_op_lbl;
    if !label.is_null() { SetWindowTextW(label, wide(&format!("{}%", APP().get().cfg.opacity)).as_ptr()); }
}

pub unsafe fn choose_filter_color() {
    let mut colors = APP().get().filter_dialog_colors;
    let mut dialog: CHOOSECOLORW = std::mem::zeroed();
    dialog.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
    dialog.hwndOwner = if APP().get().hwnd_panel.is_null() { APP().get().hwnd_main } else { APP().get().hwnd_panel };
    dialog.rgbResult = APP().get().cfg.filter_color();
    dialog.lpCustColors = colors.as_mut_ptr();
    dialog.Flags = 1 | 2 | 0x100; // CC_RGBINIT | CC_FULLOPEN | CC_ANYCOLOR
    let selection = if ChooseColorW(&mut dialog) != 0 {
        APP().get().filter_dialog_colors = colors;
        Some(dialog.rgbResult)
    } else { None };
    APP().get().set_custom_filter_color(selection);
}

unsafe fn smoke_check_effects(panel: HWND, controls: &[HWND]) -> Result<(), String> {
    let control = |id| controls.iter().copied().find(|c| GetWindowLongPtrW(*c, GWL_ID) as u32 == id)
        .ok_or_else(|| format!("Effect control {} missing", id));
    let paper = control(IDC_CHK_PAPER)?;
    let filter = control(IDC_CHK_FILTER)?;
    let depth = control(IDC_TRACK_FILTER)?;
    let original = APP().get().cfg.clone();
    let verify_pixels = || -> Result<(), String> {
        let app = APP().get();
        if app.overlays.is_empty() { return Err("No test overlay allocated".into()); }
        for overlay in &app.overlays {
            if overlay.bits.is_null() || IsWindowVisible(overlay.hwnd) != 0 {
                return Err("Test overlay missing or unexpectedly visible".into());
            }
            let count = overlay.w * overlay.h;
            let pixels = std::slice::from_raw_parts(overlay.bits, count * 4);
            let mut expected = vec![0; count * 4];
            crate::color_filter::compose(&mut expected, &overlay.master, &app.cfg);
            if pixels != expected {
                return Err(format!("Rendered overlay differs from paper and tint on {}", overlay.device));
            }
        }
        if SendMessageW(paper, BM_GETCHECK, 0, 0) != app.cfg.enabled as LRESULT
            || SendMessageW(filter, BM_GETCHECK, 0, 0) != app.cfg.filter_enabled as LRESULT
            || SendMessageW(depth, TBM_GETPOS, 0, 0) != app.cfg.filter_depth as LRESULT {
            return Err("Effect controls lost synchronization".into());
        }
        Ok(())
    };
    let result = (|| {
        let opacity = control(IDC_TRACK_OP)?;
        SendMessageW(opacity, WM_KEYDOWN, 0x23, 0); // VK_END
        SendMessageW(opacity, 0x101, 0x23, 0);
        if APP().get().cfg.opacity != 100 || SendMessageW(opacity, TBM_GETPOS, 0, 0) != 100 {
            return Err("Opacity slider cannot reach 100%".into());
        }
        let saved = Config::from_json_text(&APP().get().cfg.to_json_text()).unwrap();
        if saved.opacity != 100 { return Err("100% opacity did not survive reload".into()); }
        for (index, expected) in [(8, 90), (9, 100)] {
            crate::dispatch_tray_command(crate::tray::ID_OPACITY_FIRST + index);
            if APP().get().cfg.opacity != expected || SendMessageW(opacity, TBM_GETPOS, 0, 0) != expected as LRESULT {
                return Err("Tray opacity preset and slider did not agree".into());
            }
        }
        APP().get().set_opacity(original.opacity);
        APP().get().cfg.enabled = false;
        APP().get().cfg.filter_enabled = false;
        APP().get().cfg.filter_depth = 45;
        APP().get().render_all();
        APP().get().apply_visibility();
        panel_refresh_effects();
        verify_pixels()?;

        SendMessageW(filter, 0xF5, 0, 0);
        if !APP().get().cfg.filter_enabled || APP().get().cfg.enabled {
            return Err("Filter-only checkbox mode failed".into());
        }
        for (index, kind) in FILTER_KINDS[..3].iter().enumerate() {
            SendMessageW(control(IDC_FILTER_FIRST + index as u32)?, 0xF5, 0, 0);
            if APP().get().cfg.filter_kind != *kind { return Err("Color swatch selection failed".into()); }
            verify_pixels()?;
        }
        for value in [0, 100, 37] {
            SendMessageW(depth, TBM_SETPOS, 1, value);
            SendMessageW(panel, WM_HSCROLL, 5, depth as LPARAM);
            if APP().get().cfg.filter_depth != value as u32 {
                return Err("Color depth slider did not update configuration".into());
            }
            verify_pixels()?;
            if value == 0 && APP().get().cfg.filter_active() {
                return Err("Zero depth remained active".into());
            }
        }
        SendMessageW(depth, WM_KEYDOWN, 0x27, 0); // VK_RIGHT
        SendMessageW(depth, 0x101, 0x27, 0);
        if APP().get().cfg.filter_depth != 38 { return Err("Slider keyboard step failed".into()); }
        SendMessageW(depth, WM_KEYDOWN, 0x24, 0); // VK_HOME
        SendMessageW(depth, 0x101, 0x24, 0);
        if APP().get().cfg.filter_depth != 0 { return Err("Slider keyboard minimum failed".into()); }
        UpdateWindow(depth);
        let mut thumb: RECT = std::mem::zeroed();
        let mut channel: RECT = std::mem::zeroed();
        SendMessageW(depth, 0x419, 0, &mut thumb as *mut RECT as LPARAM);
        SendMessageW(depth, 0x41A, 0, &mut channel as *mut RECT as LPARAM);
        let middle = ((thumb.top + thumb.bottom) / 2) as u32;
        SendMessageW(depth, 0x201, 1, MAKELPARAM(((thumb.left + thumb.right) / 2) as u32, middle));
        SendMessageW(depth, 0x200, 1, MAKELPARAM(channel.right as u32, middle));
        SendMessageW(depth, 0x202, 0, MAKELPARAM(channel.right as u32, middle));
        if APP().get().cfg.filter_depth != 100 { return Err("Slider thumb drag missed its visible hit target".into()); }
        verify_pixels()?;
        APP().get().set_custom_filter_color(Some(RGB(198, 141, 164)));
        let selected = APP().get().cfg.to_json_text();
        APP().get().set_custom_filter_color(None);
        if APP().get().cfg.to_json_text() != selected || APP().get().cfg.filter_kind != "custom" {
            return Err("Custom color cancellation changed settings".into());
        }
        verify_pixels()?;

        SendMessageW(paper, 0xF5, 0, 0);
        if !APP().get().cfg.enabled || !APP().get().cfg.filter_enabled {
            return Err("Paper-plus-filter mode failed".into());
        }
        verify_pixels()?;
        SendMessageW(filter, 0xF5, 0, 0);
        if !APP().get().cfg.enabled || APP().get().cfg.filter_enabled {
            return Err("Paper-only mode failed".into());
        }
        verify_pixels()?;
        SendMessageW(paper, 0xF5, 0, 0);
        crate::dispatch_tray_command(crate::tray::ID_FILTER_TOGGLE);
        crate::dispatch_tray_command(crate::tray::ID_FILTER_FIRST + 1);
        crate::dispatch_tray_command(crate::tray::ID_FILTER_DEPTH_FIRST + 2);
        APP().get().set_texture("xuan-paper");
        if !APP().get().cfg.filter_enabled || APP().get().cfg.enabled
            || APP().get().cfg.filter_kind != "green" || APP().get().cfg.filter_depth != 50
            || APP().get().cfg.opacity != original.opacity || APP().get().cfg.intensity != original.intensity {
            return Err("Tray commands or texture changes affected independent filter settings".into());
        }
        verify_pixels()
    })();
    APP().get().cfg = original;
    APP().get().mark_all_masters_dirty();
    APP().get().render_all();
    APP().get().apply_visibility();
    APP().get().save_config_soon();
    panel_refresh_texture();
    panel_refresh_effects();
    result
}

unsafe fn smoke_check_slider_repaints(controls: &[HWND]) -> Result<(), String> {
    if APP().get().panel_theme.high_contrast { return Ok(()); }
    let original_focus = GetFocus();
    for (id, min, max) in [(IDC_TRACK_OP, 10, 100), (IDC_TRACK_INT, 10, 100), (IDC_TRACK_FILTER, 0, 100)] {
        let control = controls.iter().copied().find(|c| GetWindowLongPtrW(*c, GWL_ID) as u32 == id)
            .ok_or_else(|| format!("Slider {} missing", id))?;
        SetFocus(control);
        panel_keep_focus_visible();
        let original = SendMessageW(control, TBM_GETPOS, 0, 0);
        let mut probe = SliderPaintProbe::default();
        if SetWindowSubclass(control, slider_wndproc, 1, &mut probe as *mut _ as usize) == 0 {
            return Err("Slider repaint test hook failed".into());
        }
        for value in (min..=max).step_by(2).chain((min..=max).rev().step_by(2)) {
            SendMessageW(control, TBM_SETPOS, 1, value);
            UpdateWindow(control);
        }
        // An exposed sliver must clear the old round thumb too, without a full WM_PRINT masking it.
        ValidateRect(control, core::ptr::null());
        let sliver = RECT { left: 1, top: 1, right: 2, bottom: 2 };
        InvalidateRect(control, &sliver, 0);
        UpdateWindow(control);
        SendMessageW(control, TBM_SETPOS, 1, original);
        UpdateWindow(control);
        SetWindowSubclass(control, slider_wndproc, 1, 0);
        if probe.paints < 20 || probe.clipped != 0 {
            return Err(format!("Slider {} left clipped repaint regions during continuous movement: {} / {}",
                               id, probe.clipped, probe.paints));
        }
    }
    SetFocus(original_focus);
    scroll_panel_to(0);
    Ok(())
}

/// Render only this app's isolated offscreen window, never the desktop.
unsafe fn smoke_render_page(path: &str) -> Result<usize, String> {
    let app = APP().get();
    let panel = app.hwnd_panel;
    let controls = app.panel_ctls.clone();
    let mut rect: RECT = std::mem::zeroed();
    GetWindowRect(panel, &mut rect);
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = 40;
    info.bmiHeader.biWidth = rect.width();
    info.bmiHeader.biHeight = -rect.height();
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    let mut bits: *mut c_void = core::ptr::null_mut();
    let dc = CreateCompatibleDC(core::ptr::null_mut());
    let bitmap = CreateDIBSection(dc, &info, 0, &mut bits, core::ptr::null_mut(), 0);
    if dc.is_null() || bitmap.is_null() || bits.is_null() {
        if !bitmap.is_null() { DeleteObject(bitmap as HGDIOBJ); }
        if !dc.is_null() { DeleteDC(dc); }
        return Err("Panel test bitmap allocation failed".into());
    }
    let old = SelectObject(dc, bitmap as HGDIOBJ);
    SendMessageW(panel, WM_PRINT, dc as WPARAM, 2 | 4 | 8 | 16);
    GdiFlush();
    let pixels = std::slice::from_raw_parts(bits as *const u8, (rect.width() * rect.height() * 4) as usize).to_vec();
    SelectObject(dc, old);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(dc);

    let fg = APP().get().panel_theme.fg;
    let expected = [(fg >> 16) as u8, (fg >> 8) as u8, fg as u8];
    let scale = APP().get().panel_scale();
    let s = |value: i32| (value as f32 * scale).round() as i32;
    let mut readable_pixels = 0;
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(panel, &mut client);
    let mut origin = POINT { x: 0, y: 0 };
    ClientToScreen(panel, &mut origin);
    let measure_dc = CreateCompatibleDC(core::ptr::null_mut());
    let validation = (|| -> Result<(), String> {
        for (index, &control) in controls.iter().enumerate() {
            let id = GetWindowLongPtrW(control, GWL_ID) as u32;
            if id == 3007 { return Err("Obsolete watermark control still present".into()); }
            let mut item: RECT = std::mem::zeroed();
            GetWindowRect(control, &mut item);
            if item.left < origin.x || item.right > origin.x + client.width() {
                return Err(format!("Control {} extends horizontally outside panel", id));
            }
            if [IDC_TRACK_OP, IDC_TRACK_INT, IDC_TRACK_FILTER].contains(&id) && !APP().get().panel_theme.high_contrast
                && item.top >= origin.y && item.bottom <= origin.y + client.height() {
                let mut thumb: RECT = std::mem::zeroed();
                SendMessageW(control, 0x419, 0, &mut thumb as *mut RECT as LPARAM);
                let center_y = item.top + (thumb.top + thumb.bottom) / 2;
                for label in [controls[index - 1], controls[index + 1]] {
                    let mut label_rect: RECT = std::mem::zeroed();
                    GetWindowRect(label, &mut label_rect);
                    let label_y = (label_rect.top + label_rect.bottom) / 2;
                    if (center_y - label_y).abs() > 1 {
                        return Err(format!("Slider {} is not aligned with its text: {} vs {}", id, center_y, label_y));
                    }
                }
                let mut knob_pixels = 0;
                for y in (item.top - rect.top).max(0)..(item.bottom - rect.top).min(rect.height()) {
                    for x in (item.left - rect.left).max(0)..(item.right - rect.left).min(rect.width()) {
                        let offset = ((y * rect.width() + x) * 4) as usize;
                        if pixels[offset..offset + 3] == [255, 255, 255] { knob_pixels += 1; }
                    }
                }
                if knob_pixels < (120.0 * scale * scale) as usize || knob_pixels > (270.0 * scale * scale) as usize {
                    return Err(format!("Slider {} thumb is clipped or has ghost pixels: {}", id, knob_pixels));
                }
            }
            if (IDC_FILTER_FIRST..IDC_FILTER_FIRST + 4).contains(&id) && !APP().get().panel_theme.high_contrast
                && item.top >= origin.y && item.bottom <= origin.y + client.height() {
                let color = FILTER_COLORS.get((id - IDC_FILTER_FIRST) as usize).copied()
                    .unwrap_or(APP().get().cfg.filter_custom_color);
                let theme = APP().get().panel_theme;
                let solid = [theme.bg, theme.control, theme.border, theme.highlight, color]
                    .map(|c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]);
                let center = (item.left + item.right) / 2 - rect.left;
                let mut blended = 0;
                let mut extent = RECT { left: i32::MAX, top: i32::MAX, right: 0, bottom: 0 };
                for y in item.top - rect.top + s(3)..item.top - rect.top + s(33) {
                    for x in center - s(15)..center + s(15) {
                        let offset = ((y * rect.width() + x) * 4) as usize;
                        if !solid.iter().any(|c| pixels[offset..offset + 3] == *c) { blended += 1; }
                        if (0..3).any(|c| pixels[offset + c].abs_diff(solid[0][c]) > 20) {
                            extent.left = extent.left.min(x); extent.right = extent.right.max(x + 1);
                            extent.top = extent.top.min(y); extent.bottom = extent.bottom.max(y + 1);
                        }
                    }
                }
                if blended < (8.0 * scale) as usize {
                    return Err(format!("Color swatch {} has no antialiased edge pixels", id));
                }
                if extent.left == i32::MAX || (extent.width() - extent.height()).abs() > 2 {
                    return Err(format!("Color swatch {} is not round", id));
                }
            }
            let mut text = [0u16; 512];
            let len = GetWindowTextW(control, text.as_mut_ptr(), text.len() as i32);
            if len == 0 { continue; }
            let style = GetWindowLongPtrW(control, -16) as u32;
            let checkbox = style & 0xF == BS_AUTOCHECKBOX && id != 0;
            let swatch = (IDC_FILTER_FIRST..IDC_FILTER_FIRST + 4).contains(&id);
            let combo = [IDC_COMBO_LANG, IDC_COMBO_TEX, IDC_COMBO_THEME].contains(&id);
            let command = [IDC_BTN_MORE, IDC_BTN_BROWSE, IDC_BTN_RESET, IDC_BTN_HOTKEY].contains(&id);
            let allowance = if checkbox { s(54) } else if combo { s(30) }
                            else if command { s(if id == IDC_BTN_MORE && !APP().get().panel_more { 60 } else { 36 }) }
                            else { 0 };
            SelectObject(measure_dc, SendMessageW(control, 0x31, 0, 0) as HGDIOBJ);
            let mut measured: RECT = std::mem::zeroed();
            DrawTextW(measure_dc, text.as_ptr(), len, &mut measured, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
            // Paths intentionally ellipsize; all other labels must fit without truncation.
            if style & 0xC000 != 0x4000 && measured.width() > item.width() - allowance {
                return Err(format!("Label does not fit: {} / {} ({} > {})", id,
                                   String::from_utf16_lossy(&text[..len as usize]), measured.width(), item.width() - allowance));
            }
            let y0 = item.top - rect.top + if swatch { s(36) } else { 0 };
            let y1 = (item.bottom - rect.top).min(origin.y + client.height() - rect.top);
            let x0 = item.left - rect.left + if command { s(32) } else { 0 };
            let x1 = item.right - rect.left - if checkbox { s(54) } else if combo { s(22) } else { 0 };
            let mut count = 0;
            for y in y0.max(origin.y - rect.top)..y1 {
                for x in x0.max(0)..x1.min(rect.width()) {
                    let offset = ((y * rect.width() + x) * 4) as usize;
                    if (0..3).all(|channel| pixels[offset + channel].abs_diff(expected[channel]) < 25) { count += 1; }
                }
            }
            if item.top >= origin.y && item.bottom <= origin.y + client.height() && count < 10 {
                return Err(format!("Label is missing or unreadable: {} / {}", id, String::from_utf16_lossy(&text[..len as usize])));
            }
            readable_pixels += count;
        }
        Ok(())
    })();
    DeleteDC(measure_dc);
    let mut bmp = Vec::with_capacity(54 + pixels.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((54 + pixels.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&rect.width().to_le_bytes());
    bmp.extend_from_slice(&(-rect.height()).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&[0; 24]);
    bmp.extend_from_slice(&pixels);
    std::fs::write(path, bmp).map_err(|e| e.to_string())?;
    validation?;
    Ok(readable_pixels)
}

pub unsafe fn panel_smoke_check(path: &str) -> Result<(), String> {
    panel_create();
    let panel = APP().get().hwnd_panel;
    if panel.is_null() { return Err("Settings panel creation failed".into()); }
    SetWindowPos(panel, core::ptr::null_mut(), -30000, -30000, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    ShowWindow(panel, SW_SHOWNOACTIVATE);
    let original = APP().get().cfg.clone();
    let controls = APP().get().panel_ctls.clone();
    if controls.len() >= 24 { return Err("Main page has too many controls".into()); }
    smoke_check_slider_repaints(&controls)?;
    smoke_check_effects(panel, &controls)?;
    let readable_pixels = smoke_render_page(path)?;
    let find = |id| APP().get().panel_ctls.iter().copied()
        .find(|control| GetWindowLongPtrW(*control, GWL_ID) as u32 == id)
        .ok_or_else(|| format!("Control {} missing from current page", id));
    SetFocus(find(IDC_BTN_MORE)?);
    panel_keep_focus_visible();
    if APP().get().panel_scroll > 0 {
        smoke_render_page(&format!("{}.main-bottom.bmp", path))?;
        scroll_panel_to(0);
    }
    let navigate = || -> Result<(), String> {
        SendMessageW(find(IDC_BTN_MORE)?, 0xF5, 0, 0);
        SendMessageW(panel, WM_PANEL_PAGE, 0, 0);
        Ok(())
    };
    navigate()?;
    if !APP().get().panel_more { return Err("More settings navigation failed".into()); }
    for id in [IDC_COMBO_LANG, IDC_COMBO_THEME, IDC_CHK_AUTOSTART, IDC_BTN_HOTKEY, IDC_BTN_RESET] {
        find(id)?;
    }
    for (index, theme) in THEME_KINDS.iter().enumerate() {
        let combo = find(IDC_COMBO_THEME)?;
        SendMessageW(combo, CB_SETCURSEL, index, 0);
        SendMessageW(panel, WM_COMMAND, MAKELPARAM(IDC_COMBO_THEME, CBN_SELCHANGE) as WPARAM, combo as LPARAM);
        SendMessageW(panel, WM_APPEARANCE_CHANGED, 0, 0);
        if APP().get().cfg.theme != *theme || APP().get().panel_theme != PanelTheme::from_config(&APP().get().cfg) {
            return Err(format!("Appearance selection failed: {}", theme));
        }
        SendMessageW(APP().get().hwnd_main, WM_SETTINGCHANGE, 0, 0);
        if APP().get().cfg.theme != *theme { return Err("System notification changed manual preference".into()); }
    }
    APP().get().set_theme(&original.theme);
    SendMessageW(panel, WM_APPEARANCE_CHANGED, 0, 0);
    for (index, language) in LANGUAGE_KINDS.iter().enumerate() {
        let combo = find(IDC_COMBO_LANG)?;
        SendMessageW(combo, CB_SETCURSEL, index, 0);
        SendMessageW(panel, WM_COMMAND, MAKELPARAM(IDC_COMBO_LANG, CBN_SELCHANGE) as WPARAM, combo as LPARAM);
        SendMessageW(panel, WM_LANGUAGE_CHANGED, 0, 0);
        if APP().get().cfg.language != *language || !APP().get().panel_more {
            return Err("Language switch lost the current page".into());
        }
    }
    APP().get().set_language(&original.language);
    SendMessageW(panel, WM_LANGUAGE_CHANGED, 0, 0);
    let auto = find(IDC_CHK_AUTOSTART)?;
    SendMessageW(auto, 0xF5, 0, 0);
    if APP().get().cfg.auto_start == original.auto_start { return Err("Startup switch failed".into()); }
    SendMessageW(auto, 0xF5, 0, 0);
    for (index, monitor) in crate::overlay::enum_monitors().iter().enumerate().take(16) {
        let control = find(IDC_MON_FIRST + index as u32)?;
        let on = APP().get().cfg.monitor_enabled(&monitor.device);
        SendMessageW(control, 0xF5, 0, 0);
        if APP().get().cfg.monitor_enabled(&monitor.device) == on { return Err("Display switch failed".into()); }
        SendMessageW(control, 0xF5, 0, 0);
    }
    SendMessageW(find(IDC_BTN_HOTKEY)?, 0xF5, 0, 0);
    if !APP().get().capture_hotkey { return Err("Hotkey capture failed".into()); }
    SendMessageW(panel, WM_KEYDOWN, VK_ESCAPE as WPARAM, 0);
    if APP().get().capture_hotkey { return Err("Hotkey cancellation failed".into()); }
    smoke_render_page(&format!("{}.more.bmp", path))?;
    // Exercise a short viewport even on large desktop monitors.
    let mut small_window: RECT = std::mem::zeroed();
    let mut small_client: RECT = std::mem::zeroed();
    GetWindowRect(panel, &mut small_window);
    GetClientRect(panel, &mut small_client);
    let page_height = (280.0 * APP().get().panel_scale()).round() as i32;
    if small_client.height() > page_height {
        let frame = small_window.height() - small_client.height();
        SetWindowPos(panel, core::ptr::null_mut(), 0, 0, small_window.width(), page_height + frame,
                     SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
        let info = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: 3, nMin: 0, nMax: APP().get().panel_content_height - 1,
            nPage: page_height as u32, nPos: 0, nTrackPos: 0 };
        SetScrollInfo(panel, 1, &info, 1);
        let mut updated_client: RECT = std::mem::zeroed();
        GetClientRect(panel, &mut updated_client);
        SetWindowPos(panel, core::ptr::null_mut(), 0, 0,
                     small_window.width() + small_client.width() - updated_client.width(), page_height + frame,
                     SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
    if APP().get().panel_content_height > {
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(panel, &mut client);
        client.height()
    } {
        SendMessageW(panel, 0x115, 7, 0);
        if APP().get().panel_scroll == 0 { return Err("Settings scrollbar failed".into()); }
        smoke_render_page(&format!("{}.more-bottom.bmp", path))?;
        SendMessageW(panel, 0x115, 6, 0);
        SetFocus(find(IDC_BTN_RESET)?);
        panel_keep_focus_visible();
        if APP().get().panel_scroll == 0 { return Err("Keyboard focus did not scroll into view".into()); }
    }
    SendMessageW(find(IDC_BTN_RESET)?, 0xF5, 0, 0);
    SendMessageW(panel, WM_PANEL_PAGE, 0, 0);
    if APP().get().cfg.texture != "fine-grain" || APP().get().cfg.filter_enabled
        || APP().get().cfg.filter_depth != 25 || APP().get().cfg.theme != "system" {
        return Err("Reset defaults failed".into());
    }
    APP().get().cfg = original.clone();
    panel_rebuild();
    navigate()?;
    if APP().get().panel_more { return Err("Back navigation failed".into()); }
    APP().get().cfg.texture = "custom".into();
    APP().get().cfg.custom_texture = r"C:\Pictures\sample-paper-texture-for-reading.png".into();
    panel_rebuild();
    find(IDC_BTN_BROWSE)?;
    smoke_render_page(&format!("{}.custom.bmp", path))?;
    APP().get().cfg = original;
    APP().get().mark_all_masters_dirty();
    APP().get().render_all();
    APP().get().apply_visibility();
    APP().get().save_config_soon();
    panel_rebuild();
    let mut title_dark = 0i32;
    let got_title = DwmGetWindowAttribute(panel, DWMWA_USE_IMMERSIVE_DARK_MODE,
                      &mut title_dark as *mut i32 as *mut c_void, 4) == 0;
    if got_title && (title_dark != 0) != APP().get().panel_theme.dark {
        return Err("Native title bar appearance did not match panel".into());
    }
    let theme = APP().get().panel_theme;
    std::fs::write(format!("{}.result.json", path), format!(
        "{{\"dark\":{},\"highContrast\":{},\"textPixels\":{},\"independentFilter\":true,\"compactPages\":true}}",
        theme.dark, theme.high_contrast, readable_pixels
    )).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_color_dialog_layout_matches_windows_abi() {
        assert_eq!(std::mem::size_of::<CHOOSECOLORW>(), if cfg!(target_pointer_width = "64") { 72 } else { 36 });
    }

    #[test]
    fn light_and_dark_text_have_readable_contrast() {
        fn luminance(color: COLORREF) -> f64 {
            let component = |shift: u32| {
                let value = ((color >> shift) & 255u32) as f64 / 255.0;
                if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * component(0) + 0.7152 * component(8) + 0.0722 * component(16)
        }
        fn contrast(a: COLORREF, b: COLORREF) -> f64 {
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }
        for dark in [false, true] {
            let colors = PanelTheme::colors(dark);
            assert!(contrast(colors.fg, colors.bg) >= 7.0);
            assert!(contrast(colors.fg, colors.control) >= 7.0);
            assert!(contrast(colors.disabled, colors.control) >= 4.5);
            assert!(contrast(colors.highlight_text, colors.highlight) >= 4.5);
        }
    }
}
