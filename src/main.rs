//! PaperGrain — ultra-lightweight paper texture overlay for Windows 10/11.
//! Native Rust + Win32 implementation (no WebView, no runtime dependencies).
//! MIT License (c) 2026 CookieFilled.

#![windows_subsystem = "windows"]
#![allow(static_mut_refs)]

mod config;
mod noise;
mod overlay;
mod panel;
mod textures;
mod tray;
mod win32;

use config::Config;
use overlay::Overlay;
use std::ffi::c_void;
use win32::*;

pub const APP_VERSION: &str = "1.0.0";
const MAIN_CLASS: &str = "PaperGrainMainWnd";

// timer ids
const TIMER_TOPMOST: usize = 1;
const TIMER_SAVECFG: usize = 2;
const TIMER_FREE_MASTERS: usize = 3;
const TIMER_SMOKE: usize = 4;

// ---------------------------------------------------------------------------
// Global app state (single GUI thread; reentrancy-safe by design)
// ---------------------------------------------------------------------------
pub struct App {
    pub cfg: Config,
    pub config_path: String,
    pub tex_dir: String,
    pub config_dirty: bool,
    pub smoke: bool,

    pub hwnd_main: HWND,
    pub hwnd_panel: HWND,
    pub overlays: Vec<Overlay>,
    pub custom: Option<textures::CustomImage>,
    pub stamp: Option<overlay::Stamp>,

    pub screen_dc: HDC,
    pub msg_font: HFONT,
    pub brush_panel: HBRUSH,
    pub tray_icon: HICON,
    pub app_icon: HICON,
    pub mutex: HANDLE,
    pub gdiplus_token: usize,

    pub hotkey_registered: bool,
    pub capture_hotkey: bool,
    pub last_render_ms: u64,

    // panel controls (tracked for rebuild/update)
    pub panel_ctls: Vec<HWND>,
    pub panel_mon_ctls: Vec<HWND>,
    pub pnl_file_lbl: HWND,
    pub pnl_op_lbl: HWND,
    pub pnl_int_lbl: HWND,
    pub pnl_hotkey_lbl: HWND,
    pub pnl_hotkey_btn: HWND,
}

pub static mut APP_STATE: Option<App> = None;

/// Accessor for the global app state. Returns a fresh `&'static mut` view;
/// the app is strictly single-threaded (one GUI thread), so this is safe by
/// construction. Never store the returned reference across reentrant calls.
pub fn APP() -> AppRef {
    AppRef
}

pub struct AppRef;

impl AppRef {
    #[inline(always)]
    pub unsafe fn get(&self) -> &'static mut App {
        let ptr = &raw mut APP_STATE;
        match (*ptr).as_mut() {
            Some(a) => a,
            None => std::hint::unreachable_unchecked(),
        }
    }
}

impl App {
    pub fn panel_scale(&self) -> f32 {
        let dpi = if self.hwnd_panel != core::ptr::null_mut() {
            unsafe { GetDpiForWindow(self.hwnd_panel) }
        } else { 0 };
        let d = if dpi != 0 { dpi } else { self.primary_dpi() };
        (d as f32 / 96.0).clamp(1.0, 3.2)
    }

    fn primary_dpi(&self) -> u32 {
        unsafe {
            let hdc = GetDC(core::ptr::null_mut());
            let d = GetDeviceCaps(hdc, 88); // LOGPIXELSX
            ReleaseDC(core::ptr::null_mut(), hdc);
            if d > 0 { d as u32 } else { 96 }
        }
    }

    // -- state mutations (called from tray menu + panel) ---------------------

    pub fn save_config_soon(&mut self) {
        self.config_dirty = true;
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.cfg.enabled = on;
        self.save_config_soon();
        unsafe { self.apply_visibility() };
        unsafe { tray::tray_update_tip(self.hwnd_main) };
    }

    pub fn toggle(&mut self) {
        self.set_enabled(!self.cfg.enabled);
    }

    pub unsafe fn apply_visibility(&mut self) {
        let enabled = self.cfg.enabled;
        for o in self.overlays.iter_mut() {
            let mon_on = self.cfg.monitor_enabled(&o.device);
            let show = enabled && mon_on;
            if show {
                ShowWindow(o.hwnd, SW_SHOWNOACTIVATE);
            } else {
                ShowWindow(o.hwnd, SW_HIDE);
            }
        }
    }

    pub unsafe fn set_texture(&mut self, kind: &str) {
        if !config::TEXTURE_KINDS.contains(&kind) { return; }
        self.cfg.texture = kind.to_string();
        self.save_config_soon();
        self.mark_all_masters_dirty();
        self.render_all();
    }

    pub unsafe fn set_language(&mut self, language: &str) {
        if !self.cfg.set_language(language) { return; }
        self.capture_hotkey = false;
        self.save_config_soon();
        tray::tray_update_tip(self.hwnd_main);
        if self.hwnd_panel != core::ptr::null_mut() {
            // Rebuild after the dropdown's selection notification has returned.
            PostMessageW(self.hwnd_panel, panel::WM_LANGUAGE_CHANGED, 0, 0);
        }
    }

    pub unsafe fn set_opacity(&mut self, v: u32) {
        let v = v.clamp(10, 90);
        if v == self.cfg.opacity { return; }
        self.cfg.opacity = v;
        self.save_config_soon();
        self.render_all();
    }

    pub unsafe fn set_intensity(&mut self, v: u32) {
        let v = v.clamp(10, 100);
        if v == self.cfg.intensity { return; }
        self.cfg.intensity = v;
        self.save_config_soon();
        self.mark_all_masters_dirty();
        self.render_all();
    }

    pub unsafe fn set_monitor(&mut self, device: &str, on: bool) {
        self.cfg.set_monitor(device, on);
        self.save_config_soon();
        self.apply_visibility();
    }

    pub unsafe fn set_watermark(&mut self, on: bool) {
        self.cfg.show_watermark = on;
        self.save_config_soon();
        self.render_all();
    }

    pub unsafe fn set_autostart(&mut self, on: bool) {
        self.cfg.auto_start = on;
        self.save_config_soon();
        apply_registry_autostart(on);
    }

    pub unsafe fn mark_all_masters_dirty(&mut self) {
        for o in self.overlays.iter_mut() {
            o.master_dirty = true;
        }
    }

    pub unsafe fn render_all(&mut self) {
        for i in 0..self.overlays.len() {
            overlay::render_overlay(i);
        }
    }

    /// Refresh overlay geometry after DPI / display changes.
    pub unsafe fn refresh_overlay_geometry(&mut self, hwnd: HWND) {
        let monitors = overlay::enum_monitors();
        for o in self.overlays.iter_mut() {
            if o.hwnd != hwnd { continue; }
            if let Some(m) = monitors.iter().find(|m| m.device == o.device) {
                o.rect = m.rect;
                let w = m.rect.width().max(1) as usize;
                let h = m.rect.height().max(1) as usize;
                if overlay::ensure_dib(o, w, h) {
                    o.master_dirty = true;
                    SetWindowPos(o.hwnd, core::ptr::null_mut(), m.rect.left,
                                 m.rect.top, w as i32, h as i32,
                                 SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
        let idx = self.overlays.iter().position(|o| o.hwnd == hwnd);
        if let Some(i) = idx {
            overlay::render_overlay(i);
        }
    }

    /// Re-create overlay windows for the current monitor topology.
    pub unsafe fn reinit_monitors(&mut self) {
        for o in self.overlays.iter_mut() {
            overlay::destroy_overlay(o);
        }
        self.overlays.clear();
        let monitors = overlay::enum_monitors();
        for m in &monitors {
            let o = overlay::create_overlay(m);
            self.overlays.push(o);
        }
        self.render_all();
        self.apply_visibility();
        // refresh panel monitor checkboxes if the panel is open
        if self.hwnd_panel != core::ptr::null_mut()
            && IsWindowVisible(self.hwnd_panel) != 0 {
            panel::panel_rebuild();
        }
    }

    /// Try to register a new global hotkey combo; revert on failure.
    pub unsafe fn try_set_hotkey(&mut self, mods: u32, vk: u32) {
        let old_mods = self.cfg.hotkey_mods;
        let old_vk = self.cfg.hotkey_vk;
        UnregisterHotKey(self.hwnd_main, 1);
        if RegisterHotKey(self.hwnd_main, 1, mods | MOD_NOREPEAT, vk) != 0 {
            self.cfg.hotkey_mods = mods;
            self.cfg.hotkey_vk = vk;
            self.hotkey_registered = true;
            self.save_config_soon();
            let t = wide(&self.cfg.hotkey_display());
            SetWindowTextW(self.pnl_hotkey_lbl, t.as_ptr());
            tray::tray_update_tip(self.hwnd_main);
        } else {
            // restore previous registration
            let ok = RegisterHotKey(self.hwnd_main, 1,
                                    old_mods | MOD_NOREPEAT, old_vk);
            self.hotkey_registered = ok != 0;
            self.warn("PaperGrain", self.cfg.text(
                "这个快捷键已被占用或由系统保留，请换一个组合。",
                "That key combination is reserved or already in use.\nPlease try another combination."));
        }
    }

    pub unsafe fn reset_defaults(&mut self) {
        let keep_x = self.cfg.panel_x;
        let keep_y = self.cfg.panel_y;
        let first_run = self.cfg.first_run;
        self.cfg = Config::default();
        self.cfg.panel_x = keep_x;
        self.cfg.panel_y = keep_y;
        self.cfg.first_run = first_run;
        self.save_config_soon();
        // hotkey
        UnregisterHotKey(self.hwnd_main, 1);
        self.hotkey_registered = RegisterHotKey(
            self.hwnd_main, 1,
            self.cfg.hotkey_mods | MOD_NOREPEAT, self.cfg.hotkey_vk) != 0;
        apply_registry_autostart(self.cfg.auto_start);
        self.mark_all_masters_dirty();
        self.render_all();
        self.apply_visibility();
        unsafe { tray::tray_update_tip(self.hwnd_main) };
    }

    /// Pick + load a custom texture (open file dialog, copy to appdata).
    pub unsafe fn browse_custom_texture(&mut self) {
        let filter_parts = [self.cfg.text("图片", "Images"), "*.png;*.jpg;*.jpeg;*.bmp;*.gif",
                            self.cfg.text("所有文件", "All files"), "*.*"];
        let mut filter: Vec<u16> = Vec::new();
        for p in filter_parts.iter() {
            filter.extend(p.encode_utf16());
            filter.push(0);
        }
        filter.push(0); // double null termination

        let mut file_buf = vec![0u16; 4096];
        let dlg_title = wide(self.cfg.text("选择纸张纹理图片", "Choose a paper texture"));
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = self.hwnd_panel;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = dlg_title.as_ptr();
        ofn.Flags = OFN_HIDEREADONLY | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST
            | OFN_FILEMUSTEXIST;
        if GetOpenFileNameW(&mut ofn) == 0 { return; }
        let src = from_utf16(&file_buf);
        if src.is_empty() { return; }

        // copy into %APPDATA%\PaperGrain\textures for persistence
        let fname = src.rsplit(['\\', '/']).next().unwrap_or("custom.png");
        let dst = format!("{}\\{}", self.tex_dir, fname);
        let dst_w = wide(&dst);
        let src_w = wide(&src);
        let stored = if CopyFileW(src_w.as_ptr(), dst_w.as_ptr(), 0) != 0 {
            dst
        } else {
            src
        };

        match overlay::load_custom_image(&stored) {
            Some(img) => {
                self.custom = Some(img);
                self.cfg.custom_texture = stored;
                self.cfg.texture = "custom".into();
                self.save_config_soon();
                self.mark_all_masters_dirty();
                self.render_all();
                panel::panel_refresh_texture();
            }
            None => {
                self.warn("PaperGrain", self.cfg.text(
                    "无法加载这张图片。支持 PNG、JPG、BMP 和 GIF。",
                    "Could not load that image as a texture.\nSupported: PNG, JPG, BMP, GIF."));
            }
        }
    }

    unsafe fn warn(&mut self, title: &str, text: &str) {
        if self.smoke { return; }
        let t = wide(text);
        let c = wide(title);
        MessageBoxW(core::ptr::null_mut(), t.as_ptr(), c.as_ptr(),
                    MB_OK | MB_ICONWARNING | MB_SETFOREGROUND);
    }
}

// ---------------------------------------------------------------------------
// Registry auto-start
// ---------------------------------------------------------------------------
unsafe fn apply_registry_autostart(on: bool) {
    let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let name = wide("PaperGrain");
    let mut hkey: HKEY = core::ptr::null_mut();
    if on {
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0,
                                core::ptr::null(), 0, KEY_SET_VALUE,
                                core::ptr::null(), &mut hkey,
                                core::ptr::null_mut());
        if r != 0 { return; }
        let mut path_buf = [0u16; 1024];
        let n = GetModuleFileNameW(core::ptr::null_mut(), path_buf.as_mut_ptr(),
                                   path_buf.len() as u32);
        if n == 0 || n as usize >= path_buf.len() {
            RegCloseKey(hkey);
            return;
        }
        let mut quoted = vec![0x22u16 /* " */];
        quoted.extend_from_slice(&path_buf[..n as usize]);
        quoted.push(0x22);
        quoted.push(0);
        let bytes = std::slice::from_raw_parts(
            quoted.as_ptr() as *const u8, quoted.len() * 2);
        RegSetValueW(hkey, name.as_ptr(), REG_SZ, bytes.as_ptr(),
                     bytes.len() as u32);
        RegCloseKey(hkey);
    } else {
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0,
                                core::ptr::null(), 0, KEY_SET_VALUE,
                                core::ptr::null(), &mut hkey,
                                core::ptr::null_mut());
        if r != 0 { return; }
        RegDeleteValueW(hkey, name.as_ptr());
        RegCloseKey(hkey);
    }
}

// ---------------------------------------------------------------------------
// Main window procedure
// ---------------------------------------------------------------------------
unsafe extern "system" fn main_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            if LOWORD(wparam as usize) != tray::TRAY_UID { return 0; }
            // classic (v0) tray: mouse message in the low word of lParam
            let mouse = lparam as u32 & 0xFFFF;
            match mouse {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    panel::panel_show();
                }
                WM_RBUTTONUP => {
                    let cmd = tray::popup_tray_menu(hwnd);
                    if cmd != 0 { dispatch_tray_command(cmd); }
                }
                _ => {}
            }
            0
        }
        WM_HOTKEY => {
            if wparam == 1 {
                APP().get().toggle();
            }
            0
        }
        WM_TIMER => {
            match wparam {
                TIMER_TOPMOST => {
                    let app = APP().get();
                    for o in app.overlays.iter() {
                        if o.hwnd != core::ptr::null_mut()
                            && IsWindowVisible(o.hwnd) != 0 {
                            SetWindowPos(o.hwnd, HWND_TOPMOST, 0, 0, 0, 0,
                                         SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        }
                    }
                }
                TIMER_SAVECFG => {
                    let app = APP().get();
                    if app.config_dirty {
                        config::write_file_text(&app.config_path,
                                                &app.cfg.to_json_text());
                        app.config_dirty = false;
                    }
                }
                TIMER_FREE_MASTERS => {
                    // idle memory trim: drop cached masters after 5s of
                    // inactivity (regenerated on demand in ~50-200ms)
                    let app = APP().get();
                    let now = GetTickCount64();
                    if now.saturating_sub(app.last_render_ms) > 5000 {
                        for o in app.overlays.iter_mut() {
                            if o.master.len() > 0 {
                                o.master = Vec::new();
                                o.master_dirty = true;
                            }
                        }
                    }
                }
                TIMER_SMOKE => {
                    KillTimer(hwnd, TIMER_SMOKE);
                    PostQuitMessage(0);
                }
                _ => {}
            }
            0
        }
        WM_DISPLAYCHANGE => {
            APP().get().reinit_monitors();
            0
        }
        WM_SETTINGCHANGE => {
            // explorer restarted? taskbar may have dropped our icon
            if lparam != 0 {
                let p = lparam as *const u16;
                let mut buf = [0u16; 32];
                let mut i = 0usize;
                while i < 32 {
                    let c = *p.add(i);
                    if c == 0 { break; }
                    buf[i] = c;
                    i += 1;
                }
                let s = from_utf16(&buf);
                if s.starts_with("TaskbarCreated") {
                    let app = APP().get();
                    tray::tray_add(app.hwnd_main, app.tray_icon);
                }
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_ENDSESSION if wparam != 0 => {
            let app = APP().get();
            if app.config_dirty {
                config::write_file_text(&app.config_path, &app.cfg.to_json_text());
                app.config_dirty = false;
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// Tray command dispatch
// ---------------------------------------------------------------------------
unsafe fn dispatch_tray_command(cmd: u32) {
    let app = APP().get();
    match cmd {
        tray::ID_TOGGLE => app.toggle(),
        tray::ID_SETTINGS => panel::panel_show(),
        tray::ID_LANG_ZH => app.set_language("zh-CN"),
        tray::ID_LANG_EN => app.set_language("en"),
        tray::ID_ABOUT => {
            if !app.smoke {
                let text = format!(
                    "PaperGrain {}\n{}\n\n{}\n\n{}: {}\n{}:\n{}\n\n{}",
                    APP_VERSION,
                    app.cfg.text("Windows 10/11 屏幕纸纹工具", "Paper texture overlay for Windows 10/11"),
                    app.cfg.text("MIT 许可证 (c) 2026 CookieFilled", "MIT License (c) 2026 CookieFilled"),
                    app.cfg.text("快捷键", "Hotkey"), app.cfg.hotkey_display(),
                    app.cfg.text("设置文件", "Configuration"), app.config_path,
                    app.cfg.text("纸纹不会拦截鼠标操作；屏幕分享前可先关闭纸纹。",
                                 "Overlays are click-through; disable before screen sharing."));
                let t = wide(&text);
                let c = wide(app.cfg.text("关于 PaperGrain", "About PaperGrain"));
                MessageBoxW(core::ptr::null_mut(), t.as_ptr(), c.as_ptr(),
                            MB_OK | MB_ICONINFORMATION);
            }
        }
        tray::ID_EXIT => {
            let app = APP().get();
            if app.config_dirty {
                config::write_file_text(&app.config_path, &app.cfg.to_json_text());
                app.config_dirty = false;
            }
            DestroyWindow(app.hwnd_main);
        }
        tray::ID_AUTOSTART => {
            let on = !app.cfg.auto_start;
            app.set_autostart(on);
        }
        tray::ID_WATERMARK => {
            let on = !app.cfg.show_watermark;
            app.set_watermark(on);
        }
        tray::ID_DARKMODE => {
            app.cfg.dark_mode = !app.cfg.dark_mode;
            app.save_config_soon();
            panel::panel_apply_theme();
            if app.hwnd_panel != core::ptr::null_mut()
                && IsWindowVisible(app.hwnd_panel) != 0 {
                panel::panel_rebuild();
            }
        }
        _ => {
            // textures
            if (tray::ID_TEX_FIRST..=tray::ID_TEX_CUSTOM).contains(&cmd) {
                let idx = (cmd - tray::ID_TEX_FIRST) as usize;
                if idx < config::TEXTURE_KINDS.len() {
                    let kind = config::TEXTURE_KINDS[idx];
                    if kind == "custom" && app.cfg.custom_texture.is_empty() {
                        app.browse_custom_texture();
                        if app.cfg.texture != "custom" {
                            // browse failed/cancelled — stay on preset
                        }
                    } else {
                        app.set_texture(kind);
                    }
                }
            }
            // opacity presets
            else if (tray::ID_OPACITY_FIRST..=tray::ID_OPACITY_FIRST + 8)
                .contains(&cmd) {
                let v = (cmd - tray::ID_OPACITY_FIRST + 1) * 10;
                app.set_opacity(v);
            }
            // monitor toggles
            else if cmd >= tray::ID_MON_FIRST
                && cmd < tray::ID_MON_FIRST + 16 {
                let idx = (cmd - tray::ID_MON_FIRST) as usize;
                let monitors = overlay::enum_monitors();
                if idx < monitors.len() {
                    let dev = monitors[idx].device.clone();
                    let on = !app.cfg.monitor_enabled(&dev);
                    app.set_monitor(&dev, on);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Icon loading (resource with runtime-generated fallback)
// ---------------------------------------------------------------------------
unsafe fn load_app_icon() -> HICON {
    let inst = GetModuleHandleW(core::ptr::null_mut());
    let sm = GetSystemMetrics(SM_CXSMICON).max(16);
    let smy = GetSystemMetrics(SM_CYSMICON).max(16);
    let h = LoadImageW(inst, 1usize as *const u16, 1 /*IMAGE_ICON*/,
                       sm, smy, 0 /*LR_DEFAULTCOLOR*/);
    if h != core::ptr::null_mut() {
        return h as HICON;
    }
    let h2 = LoadIconW(core::ptr::null_mut(), IDI_APPLICATION);
    h2
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------
unsafe fn register_classes(inst: HINSTANCE) -> bool {
    let cursor = LoadCursorW(core::ptr::null_mut(), IDC_ARROW);
    let icon = load_app_icon();

    let main_cls = wide(MAIN_CLASS);
    let wc1 = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(main_wndproc),
        cbClsExtra: 0, cbWndExtra: 0,
        hInstance: inst, hIcon: icon, hCursor: cursor,
        hbrBackground: core::ptr::null_mut(),
        lpszMenuName: core::ptr::null(),
        lpszClassName: main_cls.as_ptr(),
        hIconSm: icon,
    };
    if RegisterClassExW(&wc1) == 0 { return false; }

    let ov_cls = wide(overlay::OVERLAY_CLASS);
    let wc2 = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(overlay::overlay_wndproc),
        cbClsExtra: 0, cbWndExtra: 0,
        hInstance: inst, hIcon: icon, hCursor: cursor,
        hbrBackground: core::ptr::null_mut(),
        lpszMenuName: core::ptr::null(),
        lpszClassName: ov_cls.as_ptr(),
        hIconSm: icon,
    };
    if RegisterClassExW(&wc2) == 0 { return false; }

    let pnl_cls = wide(panel::PANEL_CLASS);
    let wc3 = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(panel::panel_wndproc),
        cbClsExtra: 0, cbWndExtra: 0,
        hInstance: inst, hIcon: icon, hCursor: cursor,
        hbrBackground: core::ptr::null_mut(),
        lpszMenuName: core::ptr::null(),
        lpszClassName: pnl_cls.as_ptr(),
        hIconSm: icon,
    };
    RegisterClassExW(&wc3) != 0
}

unsafe fn init_message_font(app: &mut App) {
    let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
    ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
    if SystemParametersInfoW(SPI_GETNONCLIENTMETRICS,
                             ncm.cbSize,
                             &mut ncm as *mut NONCLIENTMETRICSW as *mut c_void,
                             0) != 0 {
        app.msg_font = CreateFontIndirectW(&ncm.lfMessageFont);
    }
    if app.msg_font == core::ptr::null_mut() {
        // fallback: Segoe UI 9pt
        let mut lf: LOGFONTW = std::mem::zeroed();
        lf.lfHeight = -12;
        lf.lfWeight = 400;
        copy_into_buf(&mut lf.lfFaceName, "Segoe UI");
        app.msg_font = CreateFontIndirectW(&lf);
    }
}

fn ensure_dirs(appdata: &str) -> (String, String) {
    let base = format!("{}\\PaperGrain", appdata);
    let tex = format!("{}\\textures", base);
    let _ = std::fs::create_dir_all(&tex);
    (format!("{}\\config.json", base), tex)
}

fn main() {
    unsafe {
        let smoke = std::env::args().any(|a| a == "--smoke");

        let (config_path, tex_dir) = match config::appdata_dir() {
            Some(ad) => ensure_dirs(&ad),
            None => ("PaperGrain.json".to_string(), "textures".to_string()),
        };
        let cfg = config::read_file_text(&config_path)
            .and_then(|t| Config::from_json_text(&t))
            .unwrap_or_else(Config::default);

        // DPI awareness first (per-monitor v2, with dynamic fallback)
        if SetProcessDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            if let Some(f) = SetProcessDpiAware_dyn() {
                f();
            }
        }

        // single instance
        let mutex_name = wide("Local\\PaperGrainSingleton");
        let mutex = CreateMutexW(core::ptr::null(), 1, mutex_name.as_ptr());
        let already = mutex != core::ptr::null_mut()
            && GetLastError() == 183; // ERROR_ALREADY_EXISTS
        if already {
            if !smoke {
                let t = wide(cfg.text("PaperGrain 已在运行，请在右下角找到它的图标。",
                    "PaperGrain is already running.\nLook for the paper-grain icon in the system tray."));
                let c = wide("PaperGrain");
                MessageBoxW(core::ptr::null_mut(), t.as_ptr(), c.as_ptr(),
                            MB_OK | MB_ICONINFORMATION);
            }
            if mutex != core::ptr::null_mut() { CloseHandle(mutex); }
            return;
        }

        // common controls (trackbar) + GDI+ (custom textures)
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES | ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&icc);
        let mut token: usize = 0;
        let gp_input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: core::ptr::null_mut(),
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        let gdiplus_ok = GdiplusStartup(&mut token, &gp_input,
                                         core::ptr::null_mut()) == 0;

        // app state (before any window creation)
        let inst = GetModuleHandleW(core::ptr::null_mut());
        let app = App {
            cfg,
            config_path,
            tex_dir,
            config_dirty: true, // Persist the language default for older settings.
            smoke,
            hwnd_main: core::ptr::null_mut(),
            hwnd_panel: core::ptr::null_mut(),
            overlays: Vec::new(),
            custom: None,
            stamp: None,
            screen_dc: GetDC(core::ptr::null_mut()),
            msg_font: core::ptr::null_mut(),
            brush_panel: core::ptr::null_mut(),
            tray_icon: core::ptr::null_mut(),
            app_icon: core::ptr::null_mut(),
            mutex,
            gdiplus_token: if gdiplus_ok { token } else { 0 },
            hotkey_registered: false,
            capture_hotkey: false,
            last_render_ms: GetTickCount64(),
            panel_ctls: Vec::new(),
            panel_mon_ctls: Vec::new(),
            pnl_file_lbl: core::ptr::null_mut(),
            pnl_op_lbl: core::ptr::null_mut(),
            pnl_int_lbl: core::ptr::null_mut(),
            pnl_hotkey_lbl: core::ptr::null_mut(),
            pnl_hotkey_btn: core::ptr::null_mut(),
        };
        APP_STATE = Some(app);
        let app = APP().get();
        init_message_font(app);

        if !register_classes(inst) {
            return;
        }

        // hidden top-level main window (receives tray, hotkeys, timers)
        let main_cls = wide(MAIN_CLASS);
        let main_title = wide("PaperGrain");
        app.hwnd_main = CreateWindowExW(
            0,
            main_cls.as_ptr(), main_title.as_ptr(),
            0, // WS_OVERLAPPED, never shown
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
            core::ptr::null_mut(), core::ptr::null_mut(), inst,
            core::ptr::null(),
        );
        if app.hwnd_main == core::ptr::null_mut() { return; }

        // tray icon
        app.app_icon = load_app_icon();
        app.tray_icon = app.app_icon;
        if !smoke { tray::tray_add(app.hwnd_main, app.tray_icon); }

        // load custom texture if configured
        if app.cfg.texture == "custom" && !app.cfg.custom_texture.is_empty() {
            app.custom = overlay::load_custom_image(&app.cfg.custom_texture);
            if app.custom.is_none() {
                app.cfg.texture = "fine-grain".into();
            }
        }

        // hotkey
        if !smoke {
            app.hotkey_registered = RegisterHotKey(
                app.hwnd_main, 1,
                app.cfg.hotkey_mods | MOD_NOREPEAT, app.cfg.hotkey_vk) != 0;
        }

        // overlays for all monitors
        app.reinit_monitors();
        app.apply_visibility();

        // first-run balloon
        if app.cfg.first_run && !smoke {
            tray::tray_balloon(
                app.hwnd_main, app.cfg.text("PaperGrain 已启动", "PaperGrain is running"),
                app.cfg.text("纸纹已开启。右键点击图标可调整设置，也可用快捷键开关。",
                    "Paper texture overlay is active. Right-click the tray icon to configure, or use the hotkey to toggle."));
            app.cfg.first_run = false;
            app.save_config_soon();
        }

        // persist startup state (also writes config on first run)
        if app.cfg.auto_start {
            apply_registry_autostart(true);
        }

        // timers
        SetTimer(app.hwnd_main, TIMER_TOPMOST, 2000, None);
        SetTimer(app.hwnd_main, TIMER_SAVECFG, 500, None);
        SetTimer(app.hwnd_main, TIMER_FREE_MASTERS, 4000, None);
        if smoke {
            SetTimer(app.hwnd_main, TIMER_SMOKE, 1500, None);
        }

        // message loop
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let r = GetMessageW(&mut msg, core::ptr::null_mut(), 0, 0);
            if r <= 0 { break; }
            let app = APP().get();
            let skip_dialog = app.hwnd_panel != core::ptr::null_mut()
                && IsWindowVisible(app.hwnd_panel) != 0
                && !app.capture_hotkey;
            let handled = if skip_dialog {
                IsDialogMessageW(app.hwnd_panel, &mut msg) != 0
            } else {
                false
            };
            if !handled {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // cleanup
        let app = APP().get();
        if app.config_dirty {
            config::write_file_text(&app.config_path, &app.cfg.to_json_text());
        }
        tray::tray_remove(app.hwnd_main);
        UnregisterHotKey(app.hwnd_main, 1);
        KillTimer(app.hwnd_main, TIMER_TOPMOST);
        KillTimer(app.hwnd_main, TIMER_SAVECFG);
        KillTimer(app.hwnd_main, TIMER_FREE_MASTERS);
        for o in app.overlays.iter_mut() {
            overlay::destroy_overlay(o);
        }
        if app.hwnd_panel != core::ptr::null_mut() {
            DestroyWindow(app.hwnd_panel);
        }
        DestroyWindow(app.hwnd_main);
        if app.brush_panel != core::ptr::null_mut() {
            DeleteObject(app.brush_panel as HGDIOBJ);
        }
        if app.msg_font != core::ptr::null_mut() {
            DeleteObject(app.msg_font as HGDIOBJ);
        }
        if app.screen_dc != core::ptr::null_mut() {
            ReleaseDC(core::ptr::null_mut(), app.screen_dc);
        }
        if app.gdiplus_token != 0 {
            GdiplusShutdown(app.gdiplus_token);
        }
        if app.mutex != core::ptr::null_mut() {
            ReleaseMutex(app.mutex);
            CloseHandle(app.mutex);
        }
    }
}
