//! overlay.rs — transparent, click-through, always-on-top overlay windows.
//!
//! One layered window per monitor (`WS_EX_LAYERED|TRANSPARENT|TOOLWINDOW|
//! TOPMOST|NOACTIVATE`), painted via `UpdateLayeredWindow` with a
//! per-pixel-alpha premultiplied DIB. Static content + a low-frequency
//! topmost re-assert timer keeps idle CPU at 0%.

use crate::config::*;
use crate::textures::{self, CustomImage};
use crate::win32::*;
use crate::APP;
use std::ffi::c_void;
pub const OVERLAY_CLASS: &str = "PaperGrainOverlayWnd";

pub struct MonInfo {
    pub device: String,
    pub rect: RECT,
    pub primary: bool,
}

pub struct Overlay {
    pub hwnd: HWND,
    pub device: String,
    pub rect: RECT,
    pub w: usize,
    pub h: usize,
    pub dpi: u32,
    pub master: Vec<u8>,      // premultiplied BGRA at full opacity
    pub master_dirty: bool,
    pub mem_dc: HDC,
    pub dib: HBITMAP,
    pub bits: *mut u8,        // DIB bits (BGRA)
    pub primary: bool,
}

// ---------------------------------------------------------------------------
// Monitor enumeration
// ---------------------------------------------------------------------------
unsafe extern "system" fn mon_enum_cb(
    hmon: *mut c_void, _hdc: HDC, _rect: *const RECT, lparam: LPARAM,
) -> BOOL {
    let list = &mut *(lparam as *mut Vec<MonInfo>);
    let mut mi = MONITORINFOEXW {
        cbSize: 0, rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        dwFlags: 0, szDevice: [0u16; 32],
    };
    mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmon, &mut mi) != 0 {
        list.push(MonInfo {
            device: from_utf16(&mi.szDevice),
            rect: mi.rcMonitor,
            primary: mi.dwFlags & 1 != 0,
        });
    }
    1 // continue
}

pub fn enum_monitors() -> Vec<MonInfo> {
    let mut list: Vec<MonInfo> = Vec::new();
    unsafe {
        let lp = &mut list as *mut Vec<MonInfo> as LPARAM;
        EnumDisplayMonitors(core::ptr::null_mut(), core::ptr::null(),
                            Some(mon_enum_cb), lp);
    }
    list
}

// ---------------------------------------------------------------------------
// Overlay window proc
// ---------------------------------------------------------------------------
pub unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DPICHANGED => {
            let dpi = HIWORD(wparam as usize);
            let app = APP().get();
            for o in app.overlays.iter_mut() {
                if o.hwnd == hwnd && dpi != 0 {
                    o.dpi = dpi;
                    o.master_dirty = true;
                }
            }
            // adopt the suggested window rect
            if let Some(rc) = (lparam as *const RECT).as_ref() {
                SetWindowPos(hwnd, core::ptr::null_mut(), rc.left, rc.top,
                             rc.width(), rc.height(),
                             SWP_NOZORDER | SWP_NOACTIVATE);
            }
            APP().get().refresh_overlay_geometry(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// Creation / destruction
// ---------------------------------------------------------------------------
pub unsafe fn create_overlay(mon: &MonInfo) -> Overlay {
    let inst = GetModuleHandleW(core::ptr::null());
    let cls = wide(OVERLAY_CLASS);
    let w = mon.rect.width().max(1) as usize;
    let h = mon.rect.height().max(1) as usize;
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
            | WS_EX_NOACTIVATE,
        cls.as_ptr(),
        core::ptr::null(), // no title
        WS_POPUP,
        mon.rect.left, mon.rect.top, w as i32, h as i32,
        core::ptr::null_mut(), core::ptr::null_mut(), inst,
        core::ptr::null(),
    );
    let screen_dc = GetDC(core::ptr::null_mut());
    let mem_dc = CreateCompatibleDC(screen_dc);
    ReleaseDC(core::ptr::null_mut(), screen_dc);

    let mut o = Overlay {
        hwnd,
        device: mon.device.clone(),
        rect: mon.rect,
        w,
        h,
        dpi: 96,
        master: Vec::new(),
        master_dirty: true,
        mem_dc,
        dib: core::ptr::null_mut(),
        bits: core::ptr::null_mut(),
        primary: mon.primary,
    };
    o.dpi = overlay_dpi(hwnd);
    ensure_dib(&mut o, w, h);
    o
}

fn overlay_dpi(hwnd: HWND) -> u32 {
    unsafe {
        let d = GetDpiForWindow(hwnd);
        if d != 0 { return d; }
        let hdc = GetDC(hwnd);
        let d = GetDeviceCaps(hdc, 88); // LOGPIXELSX
        ReleaseDC(hwnd, hdc);
        if d > 0 { d as u32 } else { 96 }
    }
}

pub unsafe fn ensure_dib(o: &mut Overlay, w: usize, h: usize) -> bool {
    if o.dib != core::ptr::null_mut() && o.w == w && o.h == h {
        return true;
    }
    if o.dib != core::ptr::null_mut() {
        // deselect then delete old dib
        SelectObject(o.mem_dc, core::ptr::null_mut() as HGDIOBJ);
        DeleteObject(o.dib as HGDIOBJ);
        o.dib = core::ptr::null_mut();
        o.bits = core::ptr::null_mut();
    }
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0, biXPelsPerMeter: 0, biYPelsPerMeter: 0,
            biClrUsed: 0, biClrImportant: 0,
        },
        bmiColors: [0],
    };
    bmi.bmiHeader.biSizeImage = (w * h * 4) as u32;
    let mut bits: *mut c_void = core::ptr::null_mut();
    let hb = CreateDIBSection(core::ptr::null_mut(), &bmi, DIB_RGB_COLORS,
                              &mut bits, core::ptr::null_mut(), 0);
    if hb == core::ptr::null_mut() || bits == core::ptr::null_mut() {
        return false;
    }
    SelectObject(o.mem_dc, hb as HGDIOBJ);
    o.dib = hb;
    o.bits = bits as *mut u8;
    o.w = w;
    o.h = h;
    true
}

pub unsafe fn destroy_overlay(o: &mut Overlay) {
    if o.hwnd != core::ptr::null_mut() {
        DestroyWindow(o.hwnd);
        o.hwnd = core::ptr::null_mut();
    }
    if o.mem_dc != core::ptr::null_mut() {
        if o.dib != core::ptr::null_mut() {
            SelectObject(o.mem_dc, core::ptr::null_mut() as HGDIOBJ);
            DeleteObject(o.dib as HGDIOBJ);
        }
        DeleteDC(o.mem_dc);
        o.mem_dc = core::ptr::null_mut();
    }
}

// ---------------------------------------------------------------------------
// Master generation
// ---------------------------------------------------------------------------
pub unsafe fn build_master(o: &Overlay) -> Vec<u8> {
    let app = APP().get();
    let cfg = &app.cfg;
    if cfg.texture == "custom" {
        if let Some(img) = &app.custom {
            return textures::custom_master(img, o.w, o.h, cfg.intensity);
        }
    }
    textures::generate(&cfg.texture, o.w, o.h, o.dpi, cfg.intensity)
}

// ---------------------------------------------------------------------------
// Watermark stamp ("CookieFilled", 5% opacity, bottom-right)
// ---------------------------------------------------------------------------
pub struct Stamp {
    pub dpi: u32,
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>, // BGRA where alpha > 0 marks a set pixel
}

pub unsafe fn make_stamp(dpi: u32) -> Option<Stamp> {
    let scale = (dpi as f32 / 96.0).clamp(1.0, 3.2);
    let font_h = -(12.0 * scale).round() as i32;
    let mut lf: LOGFONTW = std::mem::zeroed();
    lf.lfHeight = font_h;
    lf.lfWeight = 400;
    lf.lfQuality = 5; // CLEARTYPE_QUALITY
    crate::win32::copy_into_buf(&mut lf.lfFaceName, "Segoe UI");
    let font = CreateFontIndirectW(&lf);
    if font == core::ptr::null_mut() { return None; }

    let screen_dc = GetDC(core::ptr::null_mut());
    let hdc = CreateCompatibleDC(screen_dc);
    ReleaseDC(core::ptr::null_mut(), screen_dc);
    let old_font = SelectObject(hdc, font as HGDIOBJ);

    // measure
    let text = wide("CookieFilled");
    let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    DrawTextW(hdc, text.as_ptr(), -1, &mut rc, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    let tw = rc.width().max(1);
    let th = rc.height().max(1);

    // draw white text on black into a temp DIB
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: tw, biHeight: -th, biPlanes: 1, biBitCount: 32,
            biCompression: BI_RGB, biSizeImage: 0, biXPelsPerMeter: 0,
            biYPelsPerMeter: 0, biClrUsed: 0, biClrImportant: 0,
        },
        bmiColors: [0],
    };
    bmi.bmiHeader.biSizeImage = (tw as usize * th as usize * 4) as u32;
    let mut bits: *mut c_void = core::ptr::null_mut();
    let dib = CreateDIBSection(core::ptr::null_mut(), &bmi, DIB_RGB_COLORS,
                               &mut bits, core::ptr::null_mut(), 0);
    if dib == core::ptr::null_mut() || bits == core::ptr::null_mut() {
        SelectObject(hdc, old_font);
        DeleteObject(font as HGDIOBJ);
        DeleteDC(hdc);
        return None;
    }
    let tdc = CreateCompatibleDC(hdc);
    let old_bmp = SelectObject(tdc, dib as HGDIOBJ);
    let old_tfont = SelectObject(tdc, font as HGDIOBJ);
    let mut black = RECT { left: 0, top: 0, right: tw, bottom: th };
    let black_brush = CreateSolidBrush(0);
    FillRect(tdc, &black, black_brush);
    DeleteObject(black_brush as HGDIOBJ);
    SetTextColor(tdc, 0xFFFFFF);
    SetBkColor(tdc, 0x000000);
    SetBkMode(tdc, OPAQUE_BK);
    DrawTextW(tdc, text.as_ptr(), -1, &mut black, DT_SINGLELINE | DT_NOPREFIX);

    // read luminance -> stamp mask
    let len = tw as usize * th as usize * 4;
    let src = std::slice::from_raw_parts(bits as *const u8, len);
    let mut data = vec![0u8; len];
    let mut any = false;
    for i in 0..(len / 4) {
        let b = src[i * 4] as u32;
        let g = src[i * 4 + 1] as u32;
        let r = src[i * 4 + 2] as u32;
        if r + g + b > 120 { // threshold on white text
            data[i * 4] = 13; data[i * 4 + 1] = 13;
            data[i * 4 + 2] = 13; data[i * 4 + 3] = 13; // 5% white
            any = true;
        }
    }
    SelectObject(tdc, old_bmp);
    SelectObject(tdc, old_tfont);
    SelectObject(hdc, old_font);
    DeleteObject(dib as HGDIOBJ);
    DeleteDC(tdc);
    DeleteObject(font as HGDIOBJ);
    DeleteDC(hdc);
    if any {
        Some(Stamp { dpi, w: tw as usize, h: th as usize, data })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Rendering / presentation
// ---------------------------------------------------------------------------
/// Blend master (full opacity) into the DIB at `opacity` percent and push it
/// to the screen with UpdateLayeredWindow. Also stamps the watermark when the
/// overlay is the primary monitor.
pub unsafe fn render_overlay(idx: usize) {
    {
        let app = APP().get();
        if idx >= app.overlays.len() { return; }
    }
    let (hwnd, mem_dc, w, h) = {
        let app = APP().get();
        let o = &app.overlays[idx];
        (o.hwnd, o.mem_dc, o.w, o.h)
    };
    if hwnd == core::ptr::null_mut() || w == 0 || h == 0 { return; }

    // (re)build master if needed
    {
        let app = APP().get();
        let o = &mut app.overlays[idx];
        if o.master.len() != w * h * 4 || o.master_dirty {
            o.master = build_master(o);
            o.master_dirty = false;
        }
    }
    APP().get().last_render_ms = GetTickCount64();

    // blend into the DIB
    let dib_len = w * h * 4;
    let (bits, rect, primary, opacity, show_wm) = {
        let app = APP().get();
        let o = &app.overlays[idx];
        (o.bits, o.rect, o.primary, app.cfg.opacity as f32 / 100.0,
         app.cfg.show_watermark && o.primary)
    };
    if bits == core::ptr::null_mut() { return; }
    let dst = std::slice::from_raw_parts_mut(bits, dib_len);
    {
        let app = APP().get();
        let master = &app.overlays[idx].master;
        if master.len() != dib_len { return; }
        for i in 0..dib_len {
            let v = master[i] as f32 * opacity + 0.5;
            dst[i] = if v > 255.0 { 255 } else { v as u8 };
        }

        // watermark on primary monitor only
        if show_wm {
            let dpi = app.overlays[idx].dpi;
            let need_stamp = match &app.stamp {
                Some(s) => s.dpi != dpi,
                None => true,
            };
            if need_stamp {
                app.stamp = make_stamp(dpi);
            }
            if let Some(st) = &app.stamp {
                let margin = (16.0 * (dpi as f32 / 96.0).clamp(1.0, 3.2))
                    .round() as usize;
                if w > st.w + margin && h > st.h + margin {
                    let y0 = h - margin - st.h;
                    let x0 = w - margin - st.w;
                    for sy in 0..st.h {
                        let dy = y0 + sy;
                        if dy >= h { break; }
                        for sx in 0..st.w {
                            let dx = x0 + sx;
                            if dx >= w { break; }
                            let sidx = (sy * st.w + sx) * 4;
                            if st.data[sidx + 3] > 0 {
                                let didx = (dy * w + dx) * 4;
                                dst[didx] = 13; dst[didx + 1] = 13;
                                dst[didx + 2] = 13; dst[didx + 3] = 13;
                            }
                        }
                    }
                }
            }
        }
    }

    // present
    let screen_dc = APP().get().screen_dc;
    let mut dstpt = POINT { x: rect.left, y: rect.top };
    let size = SIZE { cx: w as i32, cy: h as i32 };
    let srcpt = POINT { x: 0, y: 0 };
    let bf = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER, BlendFlags: 0,
        SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA,
    };
    UpdateLayeredWindow(hwnd, screen_dc, &mut dstpt, &size, mem_dc,
                        &srcpt, 0, &bf, ULW_ALPHA);
    APP().get().last_render_ms = GetTickCount64();
}

/// Load a custom texture file with GDI+ (PNG/JPG/BMP/GIF).
pub unsafe fn load_custom_image(path: &str) -> Option<CustomImage> {
    let p = wide(path);
    let mut bmp: *mut c_void = core::ptr::null_mut();
    if GdipCreateBitmapFromFile(p.as_ptr(), &mut bmp) != 0 {
        return None;
    }
    let mut w = 0u32;
    let mut h = 0u32;
    if GdipGetImageWidth(bmp, &mut w) != 0 || GdipGetImageHeight(bmp, &mut h) != 0
        || w == 0 || h == 0 || w > 16384 || h > 16384 {
        GdipDisposeImage(bmp);
        return None;
    }
    let rect = GdipRect { X: 0, Y: 0, Width: w as i32, Height: h as i32 };
    let lock_fn = match GdipLockBits_dyn() {
        Some(f) => f,
        None => { GdipDisposeImage(bmp); return None; }
    };
    let unlock_fn = match GdipUnlockBits_dyn() {
        Some(f) => f,
        None => { GdipDisposeImage(bmp); return None; }
    };
    let mut bd = BitmapData {
        Width: 0, Height: 0, Stride: 0, PixelFormat: 0,
        Scan0: core::ptr::null_mut(), Reserved: 0,
    };
    if lock_fn(bmp, &rect, IMAGE_LOCK_MODE_READ, PIXELFORMAT_32BPP_PARGB,
               &mut bd) != 0 {
        GdipDisposeImage(bmp);
        return None;
    }
    let stride = if bd.Stride < 0 { -(bd.Stride as i64) as usize } else { bd.Stride as usize };
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    if !bd.Scan0.is_null() && stride >= (w as usize) * 4 {
        for y in 0..(h as usize) {
            let src_row = (bd.Scan0 as *const u8).add(y * stride);
            let dst_row = &mut out[y * (w as usize) * 4..(y + 1) * (w as usize) * 4];
            dst_row.copy_from_slice(std::slice::from_raw_parts(src_row, (w as usize) * 4));
        }
    }
    unlock_fn(bmp, &bd);
    GdipDisposeImage(bmp);
    Some(CustomImage { w: w as usize, h: h as usize, pixels: out })
}
