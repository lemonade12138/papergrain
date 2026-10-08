//! config.rs — configuration persistence in `%APPDATA%\PaperGrain\config.json`
//! via a tiny self-contained JSON parser/serializer (no external crates).
#![allow(dead_code)]

use crate::win32::*;

// ---------------------------------------------------------------------------
// Minimal generic JSON value + parser
// ---------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(items) = self {
            for (k, v) in items {
                if k == key { return Some(v); }
            }
        }
        None
    }
    pub fn as_f64(&self) -> Option<f64> {
        if let Json::Num(n) = self { Some(*n) } else { None }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Json::Bool(b) = self { Some(*b) } else { None }
    }
}

struct P<'a> { b: &'a [u8], i: usize }

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                b' ' | b'\t' | b'\r' | b'\n' => self.i += 1,
                _ => break,
            }
        }
    }
    fn peek(&self) -> Option<u8> {
        if self.i < self.b.len() { Some(self.b[self.i]) } else { None }
    }
    fn value(&mut self) -> Option<Json> {
        self.ws();
        match self.peek()? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => self.string().map(Json::Str),
            b't' => { self.lit(b"true")?; Some(Json::Bool(true)) }
            b'f' => { self.lit(b"false")?; Some(Json::Bool(false)) }
            b'n' => { self.lit(b"null")?; Some(Json::Null) }
            _ => self.number(),
        }
    }
    fn lit(&mut self, l: &[u8]) -> Option<()> {
        if self.i + l.len() <= self.b.len() && &self.b[self.i..self.i + l.len()] == l {
            self.i += l.len();
            Some(())
        } else { None }
    }
    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9' => self.i += 1,
                _ => break,
            }
        }
        if start == self.i { return None; }
        std::str::from_utf8(&self.b[start..self.i]).ok()?
            .parse::<f64>().ok().map(Json::Num)
    }
    fn string(&mut self) -> Option<String> {
        if self.peek()? != b'"' { return None; }
        self.i += 1;
        let mut out: Vec<u16> = Vec::new();
        loop {
            let c = *self.b.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return Some(String::from_utf16_lossy(&out)),
                b'\\' => {
                    let e = *self.b.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(0x22),
                        b'\\' => out.push(0x5C),
                        b'/' => out.push(0x2F),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'n' => out.push(0x0A),
                        b'r' => out.push(0x0D),
                        b't' => out.push(0x09),
                        b'u' => {
                            let cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) {
                                // surrogate pair?
                                if self.b.get(self.i) == Some(&b'\\')
                                    && self.b.get(self.i + 1) == Some(&b'u') {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if (0xDC00..0xE000).contains(&lo) {
                                        let c = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                        out.push(c as u16);
                                    } else {
                                        out.push(0xFFFD);
                                    }
                                } else {
                                    out.push(0xFFFD);
                                }
                            } else if (0xDC00..0xE000).contains(&cp) {
                                out.push(0xFFFD);
                            } else {
                                out.push(cp as u16);
                            }
                        }
                        _ => return None,
                    }
                }
                _ => {
                    // multi-byte UTF-8: consume the full sequence (1-4 bytes).
                    // Delimiters before this point are ASCII, so self.i - 1
                    // is always a character start boundary.
                    let start = self.i - 1;
                    let mut end = self.i;
                    while end < self.b.len() && (self.b[end] & 0xC0) == 0x80 {
                        end += 1;
                    }
                    let s = std::str::from_utf8(&self.b[start..end]).ok()?;
                    for u in s.encode_utf16() { out.push(u); }
                    self.i = end;
                }
            }
        }
    }
    fn hex4(&mut self) -> Option<u32> {
        if self.i + 4 > self.b.len() { return None; }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).ok()?;
        self.i += 4;
        u32::from_str_radix(s, 16).ok()
    }
    fn object(&mut self) -> Option<Json> {
        self.i += 1; // {
        let mut items = Vec::new();
        self.ws();
        if self.peek()? == b'}' { self.i += 1; return Some(Json::Obj(items)); }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            if self.peek()? != b':' { return None; }
            self.i += 1;
            let v = self.value()?;
            items.push((k, v));
            self.ws();
            match self.peek()? {
                b',' => { self.i += 1; }
                b'}' => { self.i += 1; return Some(Json::Obj(items)); }
                _ => return None,
            }
        }
    }
    fn array(&mut self) -> Option<Json> {
        self.i += 1; // [
        let mut items = Vec::new();
        self.ws();
        if self.peek()? == b']' { self.i += 1; return Some(Json::Arr(items)); }
        loop {
            let v = self.value()?;
            items.push(v);
            self.ws();
            match self.peek()? {
                b',' => { self.i += 1; }
                b']' => { self.i += 1; return Some(Json::Arr(items)); }
                _ => return None,
            }
        }
    }
}

pub fn parse_json(s: &str) -> Option<Json> {
    let mut p = P { b: s.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i == p.b.len() { Some(v) } else { None }
}

fn escape_json(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------
pub const TEXTURE_KINDS: [&str; 9] = [
    "fine-grain", "cotton-paper", "drawing-paper", "book-paper",
    "recycled-paper", "watercolor-paper", "xuan-paper", "offset-paper", "custom",
];
pub const TEXTURE_LABELS: [&str; 9] = [
    "Fine paper grain", "Cotton paper", "Drawing paper", "Soft book paper",
    "Recycled paper", "Fine watercolor paper", "Xuan paper", "Offset printing paper", "Custom texture",
];
pub const TEXTURE_LABELS_ZH: [&str; 9] = [
    "细纸纹", "棉质纸", "素描纸", "柔和书纸",
    "再生纸", "细纹水彩纸", "宣纸", "胶版印刷纸", "自定义图片",
];
pub const LANGUAGE_KINDS: [&str; 2] = ["zh-CN", "en"];
pub const LANGUAGE_LABELS: [&str; 2] = ["简体中文", "English"];
pub const THEME_KINDS: [&str; 3] = ["system", "light", "dark"];
pub const THEME_LABELS: [&str; 3] = ["Follow system", "Light", "Dark"];
pub const THEME_LABELS_ZH: [&str; 3] = ["跟随系统", "浅色", "深色"];
pub const FILTER_KINDS: [&str; 4] = ["yellow", "green", "amber", "custom"];
pub const FILTER_LABELS: [&str; 4] = ["Yellow", "Green", "Amber", "Custom"];
pub const FILTER_LABELS_ZH: [&str; 4] = ["柔黄", "浅绿", "琥珀", "自定义"];
pub const FILTER_COLORS: [COLORREF; 3] = [RGB(250, 223, 144), RGB(160, 218, 166), RGB(244, 179, 88)];

#[derive(Clone)]
pub struct Config {
    pub version: u32,
    pub language: String,
    pub enabled: bool,
    pub texture: String,        // one of TEXTURE_KINDS
    pub custom_texture: String, // absolute path (or empty)
    pub opacity: u32,           // 10..100 (%)
    pub intensity: u32,         // 10..100 (%)
    pub filter_enabled: bool,
    pub filter_kind: String,
    pub filter_depth: u32,      // 0..100; relative strength, not opaque coverage
    pub filter_custom_color: COLORREF,
    pub monitors: Vec<(String, bool)>,
    pub hotkey_mods: u32,       // MOD_CONTROL/MOD_SHIFT/... bits (without MOD_NOREPEAT)
    pub hotkey_vk: u32,
    pub auto_start: bool,
    pub theme: String,
    pub panel_x: i32,
    pub panel_y: i32,
    pub first_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: 1,
            language: "zh-CN".into(),
            enabled: true,
            texture: "fine-grain".into(),
            custom_texture: String::new(),
            opacity: 35,
            intensity: 60,
            filter_enabled: false,
            filter_kind: "yellow".into(),
            filter_depth: 25,
            filter_custom_color: FILTER_COLORS[0],
            monitors: Vec::new(),
            hotkey_mods: MOD_CONTROL | MOD_SHIFT,
            hotkey_vk: 'P' as u32,
            auto_start: false,
            theme: "system".into(),
            panel_x: -1,
            panel_y: -1,
            first_run: true,
        }
    }
}

impl Config {
    pub fn text(&self, chinese: &'static str, english: &'static str) -> &'static str {
        if self.language == "en" { english } else { chinese }
    }

    pub fn texture_labels(&self) -> &'static [&'static str] {
        if self.language == "en" { &TEXTURE_LABELS } else { &TEXTURE_LABELS_ZH }
    }

    pub fn set_language(&mut self, language: &str) -> bool {
        if !LANGUAGE_KINDS.contains(&language) || self.language == language { return false; }
        self.language = language.to_string();
        true
    }

    pub fn theme_labels(&self) -> &'static [&'static str] {
        if self.language == "en" { &THEME_LABELS } else { &THEME_LABELS_ZH }
    }

    pub fn set_theme(&mut self, theme: &str) -> bool {
        if !THEME_KINDS.contains(&theme) || self.theme == theme { return false; }
        self.theme = theme.to_string();
        true
    }

    pub fn theme_is_dark(&self, system_dark: bool) -> bool {
        match self.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => system_dark,
        }
    }

    pub fn filter_labels(&self) -> &'static [&'static str] {
        if self.language == "en" { &FILTER_LABELS } else { &FILTER_LABELS_ZH }
    }

    pub fn set_filter_kind(&mut self, kind: &str) -> bool {
        if !FILTER_KINDS.contains(&kind) || self.filter_kind == kind { return false; }
        self.filter_kind = kind.to_string();
        true
    }

    pub fn filter_color(&self) -> COLORREF {
        FILTER_KINDS.iter().position(|kind| *kind == self.filter_kind)
            .and_then(|index| FILTER_COLORS.get(index).copied())
            .unwrap_or(self.filter_custom_color)
    }

    pub fn filter_active(&self) -> bool { self.filter_enabled && self.filter_depth > 0 }

    pub fn overlay_enabled(&self, device: &str) -> bool {
        self.monitor_enabled(device) && (self.enabled || self.filter_active())
    }

    pub fn color_to_hex(color: COLORREF) -> String {
        format!("#{:02X}{:02X}{:02X}", color & 255, (color >> 8) & 255, (color >> 16) & 255)
    }

    fn color_from_hex(text: &str) -> Option<COLORREF> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) { return None; }
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        Some(RGB((rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255))
    }

    fn mods_to_string(mods: u32) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if mods & MOD_CONTROL != 0 { parts.push("Ctrl"); }
        if mods & MOD_SHIFT != 0 { parts.push("Shift"); }
        if mods & MOD_ALT != 0 { parts.push("Alt"); }
        if mods & MOD_WIN != 0 { parts.push("Win"); }
        parts.join("+")
    }
    fn string_to_mods(s: &str) -> u32 {
        let mut m = 0u32;
        let lower = s.to_lowercase();
        for part in lower.split('+') {
            match part.trim() {
                "ctrl" | "control" => m |= MOD_CONTROL,
                "shift" => m |= MOD_SHIFT,
                "alt" => m |= MOD_ALT,
                "win" | "super" | "meta" => m |= MOD_WIN,
                _ => {}
            }
        }
        m
    }
    fn key_to_vk(s: &str) -> Option<u32> {
        let t = s.trim();
        if t.len() == 1 {
            let c = t.chars().next().unwrap();
            let cu = c.to_ascii_uppercase();
            if cu.is_ascii_alphanumeric() { return Some(cu as u32); }
        }
        let lower = t.to_lowercase();
        match lower.as_str() {
            "space" => Some(VK_SPACE),
            _ => {
                if let Some(rest) = lower.strip_prefix('f') {
                    if let Ok(n) = rest.parse::<u32>() {
                        if (1..=12).contains(&n) { return Some(0x6F + n); } // F1 = 0x70
                    }
                }
                None
            }
        }
    }
    pub fn vk_to_key_name(vk: u32) -> String {
        if (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) {
            return (vk as u8 as char).to_string();
        }
        if (0x70..=0x7B).contains(&vk) {
            return format!("F{}", vk - 0x6F);
        }
        if vk == VK_SPACE { return "Space".into(); }
        // fall back to the OS key name (scan-code based)
        unsafe {
            let sc = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
            if sc != 0 {
                let mut buf = [0u16; 64];
                let n = GetKeyNameTextW((sc as LPARAM) << 16, buf.as_mut_ptr(), 64);
                if n > 0 { return from_utf16(&buf[..n as usize]); }
            }
        }
        format!("Key{:02X}", vk)
    }
    pub fn hotkey_display(&self) -> String {
        let mods = Self::mods_to_string(self.hotkey_mods);
        let key = Self::vk_to_key_name(self.hotkey_vk);
        if mods.is_empty() { key } else { format!("{}+{}", mods, key) }
    }

    pub fn monitor_enabled(&self, device: &str) -> bool {
        for (k, v) in &self.monitors {
            if k == device { return *v; }
        }
        true // unknown/new monitors default to enabled
    }
    pub fn set_monitor(&mut self, device: &str, enabled: bool) {
        for (k, v) in self.monitors.iter_mut() {
            if k == device { *v = enabled; return; }
        }
        self.monitors.push((device.to_string(), enabled));
    }

    // -- serialization ------------------------------------------------------
    pub fn to_json_text(&self) -> String {
        let mut monitors = String::from("{");
        for (i, (k, v)) in self.monitors.iter().enumerate() {
            if i > 0 { monitors.push(','); }
            monitors.push_str("\n    ");
            escape_json(k, &mut monitors);
            monitors.push_str(": ");
            monitors.push_str(if *v { "true" } else { "false" });
        }
        if !self.monitors.is_empty() { monitors.push('\n'); }
        monitors.push('}');

        let mut hotkey = String::from("{\n    \"mods\": ");
        escape_json(&Self::mods_to_string(self.hotkey_mods), &mut hotkey);
        hotkey.push_str(",\n    \"key\": ");
        escape_json(&Self::vk_to_key_name(self.hotkey_vk), &mut hotkey);
        hotkey.push_str("\n  }");

        let mut t = String::new();
        escape_json(&self.texture, &mut t);
        let mut c = String::new();
        escape_json(&self.custom_texture, &mut c);
        let mut language = String::new();
        escape_json(&self.language, &mut language);
        let mut theme = String::new();
        escape_json(&self.theme, &mut theme);
        let mut filter_kind = String::new();
        escape_json(&self.filter_kind, &mut filter_kind);
        let filter = format!(
            "{{\n    \"enabled\": {},\n    \"color\": {},\n    \"depth\": {},\n    \"customColor\": \"{}\"\n  }}",
            self.filter_enabled, filter_kind, self.filter_depth, Self::color_to_hex(self.filter_custom_color));
        format!(
            "{{\n  \"version\": {},\n  \"language\": {},\n  \"enabled\": {},\n  \"texture\": {},\n  \"customTexture\": {},\n  \"opacity\": {},\n  \"intensity\": {},\n  \"colorFilter\": {},\n  \"monitors\": {},\n  \"hotkey\": {},\n  \"autoStart\": {},\n  \"theme\": {},\n  \"panelX\": {},\n  \"panelY\": {},\n  \"firstRun\": {}\n}}\n",
            self.version, language, self.enabled, t, c, self.opacity, self.intensity, filter, monitors, hotkey,
            self.auto_start, theme, self.panel_x, self.panel_y,
            self.first_run
        )
    }

    pub fn from_json_text(text: &str) -> Option<Config> {
        let j = parse_json(text)?;
        let mut c = Config::default();
        if let Some(v) = j.get("version").and_then(|v| v.as_f64()) { c.version = v as u32; }
        if let Some(v) = j.get("language").and_then(|v| v.as_str()) { c.set_language(v); }
        if let Some(v) = j.get("enabled").and_then(|v| v.as_bool()) { c.enabled = v; }
        if let Some(v) = j.get("texture").and_then(|v| v.as_str()) {
            if TEXTURE_KINDS.contains(&v) { c.texture = v.to_string(); }
        }
        if let Some(v) = j.get("customTexture").and_then(|v| v.as_str()) {
            c.custom_texture = v.to_string();
        }
        if let Some(v) = j.get("opacity").and_then(|v| v.as_f64()) {
            c.opacity = (v as i64).clamp(10, 100) as u32;
        }
        if let Some(v) = j.get("intensity").and_then(|v| v.as_f64()) {
            c.intensity = (v as i64).clamp(10, 100) as u32;
        }
        if let Some(filter) = j.get("colorFilter") {
            if let Some(v) = filter.get("enabled").and_then(|v| v.as_bool()) { c.filter_enabled = v; }
            if let Some(v) = filter.get("color").and_then(|v| v.as_str()) { c.set_filter_kind(v); }
            if let Some(v) = filter.get("depth").and_then(|v| v.as_f64()) { c.filter_depth = (v as i64).clamp(0, 100) as u32; }
            if let Some(v) = filter.get("customColor").and_then(|v| v.as_str()).and_then(Self::color_from_hex) {
                c.filter_custom_color = v;
            }
        }
        if let Some(Json::Obj(items)) = j.get("monitors") {
            for (k, v) in items {
                if let Some(b) = v.as_bool() { c.monitors.push((k.clone(), b)); }
            }
        }
        if let Some(h) = j.get("hotkey") {
            let mods = h.get("mods").and_then(|v| v.as_str())
                .map(|s| Self::string_to_mods(s)).unwrap_or(MOD_CONTROL | MOD_SHIFT);
            let vk = h.get("key").and_then(|v| v.as_str())
                .and_then(|s| Self::key_to_vk(s)).unwrap_or('P' as u32);
            if mods != 0 { c.hotkey_mods = mods; }
            c.hotkey_vk = vk;
        }
        if let Some(v) = j.get("autoStart").and_then(|v| v.as_bool()) { c.auto_start = v; }
        // Old darkMode settings migrate to automatic appearance, without resetting paper settings.
        if let Some(v) = j.get("theme").and_then(|v| v.as_str()) { c.set_theme(v); }
        if let Some(v) = j.get("panelX").and_then(|v| v.as_f64()) { c.panel_x = v as i32; }
        if let Some(v) = j.get("panelY").and_then(|v| v.as_f64()) { c.panel_y = v as i32; }
        if let Some(v) = j.get("firstRun").and_then(|v| v.as_bool()) { c.first_run = v; }
        Some(c)
    }
}

// ---------------------------------------------------------------------------
// File IO (Win32, wide-char)
// ---------------------------------------------------------------------------
pub fn read_file_text(path: &str) -> Option<String> {
    unsafe {
        let p = wide(path);
        let h = CreateFileW(p.as_ptr(), GENERIC_READ, FILE_SHARE_READ,
                            core::ptr::null(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
                            core::ptr::null_mut());
        if h == INVALID_HANDLE_VALUE { return None; }
        let mut size: i64 = 0;
        if GetFileSizeEx(h, &mut size) == 0 || size <= 0 || size > 1 << 20 {
            CloseHandle(h);
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let mut read = 0u32;
        let ok = ReadFile(h, buf.as_mut_ptr(), buf.len() as u32, &mut read,
                          core::ptr::null_mut());
        CloseHandle(h);
        if ok == 0 { return None; }
        buf.truncate(read as usize);
        // strip UTF-8 BOM if present
        if buf.starts_with(&[0xEF, 0xBB, 0xBF]) { buf.drain(0..3); }
        String::from_utf8(buf).ok()
    }
}

pub fn write_file_text(path: &str, text: &str) -> bool {
    unsafe {
        let tmp = format!("{}.tmp", path);
        let p = wide(&tmp);
        let h = CreateFileW(p.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ,
                            core::ptr::null(), CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL,
                            core::ptr::null_mut());
        if h == INVALID_HANDLE_VALUE { return false; }
        let bytes = text.as_bytes();
        let mut written = 0u32;
        let ok = WriteFile(h, bytes.as_ptr(), bytes.len() as u32, &mut written,
                           core::ptr::null_mut());
        CloseHandle(h);
        if ok == 0 || written as usize != bytes.len() { return false; }
        let from = wide(&tmp);
        let to = wide(path);
        if MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING) == 0 {
            // fall back to non-atomic write
            let p2 = wide(path);
            let h2 = CreateFileW(p2.as_ptr(), GENERIC_WRITE, FILE_SHARE_READ,
                                 core::ptr::null(), CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL,
                                 core::ptr::null_mut());
            if h2 == INVALID_HANDLE_VALUE { return false; }
            let mut w2 = 0u32;
            let ok2 = WriteFile(h2, bytes.as_ptr(), bytes.len() as u32, &mut w2,
                                core::ptr::null_mut());
            CloseHandle(h2);
            return ok2 != 0 && w2 as usize == bytes.len();
        }
        true
    }
}

/// %APPDATA% path (UTF-16 env query).
pub fn appdata_dir() -> Option<String> {
    unsafe {
        let name = wide("APPDATA");
        let n = GetEnvironmentVariableW(name.as_ptr(), core::ptr::null_mut(), 0);
        if n == 0 { return None; }
        let mut buf = vec![0u16; n as usize + 1];
        let n2 = GetEnvironmentVariableW(name.as_ptr(), buf.as_mut_ptr(), buf.len() as u32);
        if n2 == 0 || n2 >= buf.len() as u32 { return None; }
        buf.truncate(n2 as usize);
        Some(String::from_utf16_lossy(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_accepts_full_strength_and_preserves_it_after_reload() {
        for (input, expected) in [(0, 10), (90, 90), (95, 95), (100, 100), (101, 100)] {
            let text = format!(r#"{{"opacity":{input},"texture":"xuan-paper","intensity":77}}"#);
            let cfg = Config::from_json_text(&text).unwrap();
            assert_eq!(cfg.opacity, expected);
            let loaded = Config::from_json_text(&cfg.to_json_text()).unwrap();
            assert_eq!(loaded.opacity, expected);
            assert_eq!(loaded.texture, "xuan-paper");
            assert_eq!(loaded.intensity, 77);
        }
    }

    #[test]
    fn obsolete_watermark_setting_is_ignored_without_resetting_preferences() {
        for enabled in [false, true] {
            let cfg = Config::from_json_text(&format!(
                r#"{{"showWatermark":{enabled},"language":"en","theme":"dark","texture":"xuan-paper","opacity":90,"intensity":100,"colorFilter":{{"enabled":true,"color":"green","depth":77}}}}"#
            )).unwrap();
            assert_eq!(cfg.language, "en");
            assert_eq!(cfg.theme, "dark");
            assert_eq!(cfg.texture, "xuan-paper");
            assert_eq!((cfg.opacity, cfg.intensity), (90, 100));
            assert!(cfg.filter_enabled);
            assert_eq!(cfg.filter_kind, "green");
            assert_eq!(cfg.filter_depth, 77);
            assert!(parse_json(&cfg.to_json_text()).unwrap().get("showWatermark").is_none());
        }
    }

    #[test]
    fn independent_filter_defaults_validates_and_preserves_settings() {
        let mut cfg = Config::from_json_text(
            r#"{"enabled":false,"texture":"offset-paper","opacity":90,"intensity":100,"language":"en","theme":"dark"}"#
        ).unwrap();
        assert!(!cfg.filter_enabled);
        assert_eq!(cfg.filter_kind, "yellow");
        assert_eq!(cfg.filter_depth, 25);
        assert!(!cfg.overlay_enabled("display"));
        cfg.filter_enabled = true;
        assert!(cfg.overlay_enabled("display"));
        cfg.set_monitor("display", false);
        assert!(!cfg.overlay_enabled("display"));
        cfg.filter_depth = 0;
        assert!(!cfg.filter_active());
        cfg.filter_depth = 83;
        cfg.filter_custom_color = RGB(128, 195, 97);
        for kind in FILTER_KINDS {
            cfg.set_filter_kind(kind);
            assert!(!cfg.set_filter_kind("invalid"));
            let loaded = Config::from_json_text(&cfg.to_json_text()).unwrap();
            assert_eq!(loaded.filter_kind, kind);
            assert_eq!(loaded.filter_color(), cfg.filter_color());
            assert_eq!(loaded.filter_custom_color, RGB(128, 195, 97));
            assert_eq!(loaded.filter_depth, 83);
            assert!(loaded.filter_enabled);
            assert!(!loaded.enabled);
            assert_eq!(loaded.texture, "offset-paper");
            assert_eq!((loaded.opacity, loaded.intensity), (90, 100));
            assert_eq!((loaded.language.as_str(), loaded.theme.as_str()), ("en", "dark"));
        }
        for invalid in ["", "#FFF", "#GG1234", "#12345678", "#中文测试", "red"] {
            assert_eq!(Config::color_from_hex(invalid), None);
        }
        for (depth, expected) in [(-1, 0), (1000, 100)] {
            let loaded = Config::from_json_text(&format!(
                r##"{{"colorFilter":{{"enabled":true,"color":"unknown","depth":{depth},"customColor":"#FFFFZZ"}}}}"##
            )).unwrap();
            assert_eq!(loaded.filter_kind, "yellow");
            assert_eq!(loaded.filter_custom_color, FILTER_COLORS[0]);
            assert_eq!(loaded.filter_depth, expected);
        }
        assert_eq!(FILTER_KINDS.len(), FILTER_LABELS.len());
        assert_eq!(FILTER_KINDS.len(), FILTER_LABELS_ZH.len());
    }

    #[test]
    fn appearance_defaults_migrates_and_resolves_system_mode() {
        for text in [r#"{}"#, r#"{"darkMode":true}"#, r#"{"darkMode":false}"#,
                     r#"{"theme":"unknown"}"#, r#"{"theme":null}"#] {
            let config = Config::from_json_text(text).unwrap();
            assert_eq!(config.theme, "system");
            assert!(!config.theme_is_dark(false));
            assert!(config.theme_is_dark(true));
        }
        let mut config = Config::from_json_text(
            r#"{"darkMode":true,"texture":"cotton-paper","opacity":90,"intensity":77}"#
        ).unwrap();
        for (theme, light_system, dark_system) in
            [("dark", true, true), ("light", false, false), ("system", false, true)] {
            assert!(config.set_theme(theme));
            assert!(!config.set_theme("invalid"));
            let loaded = Config::from_json_text(&config.to_json_text()).unwrap();
            assert_eq!(loaded.theme, theme);
            assert_eq!(loaded.theme_is_dark(false), light_system);
            assert_eq!(loaded.theme_is_dark(true), dark_system);
            assert_eq!(loaded.texture, "cotton-paper");
            assert_eq!((loaded.opacity, loaded.intensity), (90, 77));
        }
        assert_eq!(THEME_KINDS.len(), THEME_LABELS.len());
        assert_eq!(THEME_KINDS.len(), THEME_LABELS_ZH.len());
    }

    #[test]
    fn new_preset_choices_survive_config_round_trip() {
        assert_eq!(TEXTURE_KINDS.len(), TEXTURE_LABELS.len());
        assert_eq!(TEXTURE_KINDS.len(), TEXTURE_LABELS_ZH.len());
        assert_eq!(TEXTURE_KINDS.last(), Some(&"custom"));
        for (index, kind) in TEXTURE_KINDS.iter().enumerate() {
            for language in LANGUAGE_KINDS {
                let mut config = Config::default();
                config.texture = (*kind).into();
                config.set_language(language);
                config.opacity = 42;
                config.intensity = 75;
                let loaded = Config::from_json_text(&config.to_json_text()).unwrap();
                assert_eq!(loaded.texture, *kind);
                assert_eq!(loaded.language, language);
                assert_eq!(loaded.texture_labels()[index], config.texture_labels()[index]);
                assert_eq!(loaded.opacity, 42);
                assert_eq!(loaded.intensity, 75);
            }
        }
    }

    #[test]
    fn removed_presets_fall_back_without_resetting_other_settings() {
        for kind in ["craft-paper", "notebook", "parchment"] {
            let mut config = Config::default();
            config.texture = kind.into();
            config.opacity = 90;
            config.intensity = 100;
            let loaded = Config::from_json_text(&config.to_json_text()).unwrap();
            assert_eq!(loaded.texture, "fine-grain");
            assert_eq!(loaded.opacity, 90);
            assert_eq!(loaded.intensity, 100);
            assert_eq!(crate::textures::generate(kind, 64, 48, 96, 60),
                       crate::textures::generate("fine-grain", 64, 48, 96, 60));
        }
    }

    #[test]
    fn old_and_invalid_language_settings_default_to_chinese() {
        for text in [
            r#"{"texture":"cotton-paper","opacity":90,"intensity":100}"#,
            r#"{"language":"unknown","texture":"cotton-paper","opacity":90,"intensity":100}"#,
            r#"{"language":null,"texture":"cotton-paper","opacity":90,"intensity":100}"#,
        ] {
            let config = Config::from_json_text(text).unwrap();
            assert_eq!(config.language, "zh-CN");
            assert_eq!(config.texture, "cotton-paper");
            assert_eq!(config.opacity, 90);
            assert_eq!(config.intensity, 100);
            assert_eq!(config.texture_labels()[1], "棉质纸");
        }
    }

    #[test]
    fn language_switches_and_persists_without_changing_texture_settings() {
        let mut config = Config::default();
        config.texture = "drawing-paper".into();
        config.opacity = 90;
        config.intensity = 100;
        for (language, label) in [("en", "Drawing paper"), ("zh-CN", "素描纸")] {
            assert!(config.set_language(language));
            assert!(!config.set_language("unsupported"));
            assert_eq!(config.texture_labels()[2], label);
            let loaded = Config::from_json_text(&config.to_json_text()).unwrap();
            assert_eq!(loaded.language, language);
            assert_eq!(loaded.texture_labels()[2], label);
            assert_eq!(loaded.texture, "drawing-paper");
            assert_eq!(loaded.opacity, 90);
            assert_eq!(loaded.intensity, 100);
        }
    }
}
