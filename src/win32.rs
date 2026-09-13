//! win32.rs — minimal hand-written Win32 FFI layer (no external crates).
//! Only what PaperGrain needs, declared explicitly so the crate builds with
//! plain rustc/cargo and links directly against system DLLs.
#![allow(non_camel_case_types, non_snake_case, dead_code, clippy::missing_safety_doc)]

use core::ffi::c_void;

// ---------------------------------------------------------------------------
// Handle / scalar types (Win64 ABI)
// ---------------------------------------------------------------------------
pub type HWND = *mut c_void;
pub type HDC = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HICON = *mut c_void;
pub type HMENU = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HBRUSH = *mut c_void;
pub type HFONT = *mut c_void;
pub type HCURSOR = *mut c_void;
pub type HANDLE = *mut c_void;
pub type HKEY = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HGDIOBJ = *mut c_void;
pub type HHOOK = *mut c_void;

pub type LONG = i32;
pub type BOOL = i32;
pub type DWORD = u32;
pub type WORD = u16;
pub type UINT = u32;
pub type ULONG = u32;
pub type LRESULT = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type ATOM = u16;
pub type COLORREF = u32;
pub type LONG_PTR = isize;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub fn HIWORD(w: usize) -> u32 { ((w >> 16) & 0xFFFF) as u32 }
pub fn LOWORD(w: usize) -> u32 { (w & 0xFFFF) as u32 }
pub fn MAKELPARAM(lo: u32, hi: u32) -> LPARAM { (lo as usize | ((hi as usize) << 16)) as isize }
pub fn RGB(r: u32, g: u32, b: u32) -> COLORREF { r | (g << 8) | (b << 16) }

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT { pub left: LONG, pub top: LONG, pub right: LONG, pub bottom: LONG }
impl RECT {
    pub fn width(&self) -> i32 { self.right - self.left }
    pub fn height(&self) -> i32 { self.bottom - self.top }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT { pub x: LONG, pub y: LONG }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SIZE { pub cx: LONG, pub cy: LONG }

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND, pub message: UINT, pub wParam: WPARAM, pub lParam: LPARAM,
    pub time: u32, pub pt: POINT, pub lPrivate: u32,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
    pub cbClsExtra: i32, pub cbWndExtra: i32,
    pub hInstance: HINSTANCE, pub hIcon: HICON, pub hCursor: HCURSOR, pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16, pub lpszClassName: *const u16, pub hIconSm: HICON,
}

#[repr(C)]
pub struct MONITORINFOEXW {
    pub cbSize: u32, pub rcMonitor: RECT, pub rcWork: RECT, pub dwFlags: u32,
    pub szDevice: [u16; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BLENDFUNCTION {
    pub BlendOp: u8, pub BlendFlags: u8,
    pub SourceConstantAlpha: u8, pub AlphaFormat: u8,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32, pub biWidth: LONG, pub biHeight: LONG,
    pub biPlanes: WORD, pub biBitCount: WORD,
    pub biCompression: u32, pub biSizeImage: u32,
    pub biXPelsPerMeter: LONG, pub biYPelsPerMeter: LONG,
    pub biClrUsed: u32, pub biClrImportant: u32,
}

#[repr(C)]
pub struct BITMAPINFO { pub bmiHeader: BITMAPINFOHEADER, pub bmiColors: [COLORREF; 1] }

#[repr(C)]
pub struct GUID { pub data1: u32, pub data2: u16, pub data3: u16, pub data4: [u8; 8] }

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: u32, pub hWnd: HWND, pub uID: u32, pub uFlags: u32,
    pub uCallbackMessage: u32, pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: u32, pub dwStateMask: u32,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: u32,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: u32,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

#[repr(C)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32, pub hwndOwner: HWND, pub hInstance: HINSTANCE,
    pub lpstrFilter: *const u16, pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32, pub nFilterIndex: u32,
    pub lpstrFile: *mut u16, pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16, pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16, pub lpstrTitle: *const u16,
    pub Flags: u32, pub nFileOffset: u16, pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: LPARAM, pub lpfnHook: *const c_void, pub lpTemplateName: *const u16,
    pub pvReserved: *mut c_void, pub dwReserved: u32, pub FlagsEx: u32,
}

#[repr(C)]
pub struct LOGFONTW {
    pub lfHeight: LONG, pub lfWidth: LONG, pub lfEscapement: LONG, pub lfOrientation: LONG,
    pub lfWeight: LONG, pub lfItalic: u8, pub lfUnderline: u8, pub lfStrikeOut: u8,
    pub lfCharSet: u8, pub lfOutPrecision: u8, pub lfClipPrecision: u8, pub lfQuality: u8,
    pub lfPitchAndFamily: u8, pub lfFaceName: [u16; 32],
}

#[repr(C)]
pub struct NONCLIENTMETRICSW {
    pub cbSize: u32,
    pub iBorderWidth: i32, pub iScrollWidth: i32, pub iScrollHeight: i32,
    pub iCaptionWidth: i32, pub iCaptionHeight: i32,
    pub iSmCaptionWidth: i32, pub iSmCaptionHeight: i32,
    pub iMenuWidth: i32, pub iMenuHeight: i32,
    pub lfCaptionFont: LOGFONTW, pub lfSmCaptionFont: LOGFONTW, pub lfMenuFont: LOGFONTW,
    pub lfStatusFont: LOGFONTW, pub lfMessageFont: LOGFONTW,
}

#[repr(C)]
pub struct INITCOMMONCONTROLSEX { pub dwSize: u32, pub dwICC: u32 }

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC, pub fErase: BOOL, pub rcPaint: RECT,
    pub fRestore: BOOL, pub fIncUpdate: BOOL, pub rgbReserved: [u8; 32],
}

// GDI+ (flat C API)
#[repr(C)]
pub struct GdiplusStartupInput {
    pub GdiplusVersion: u32,
    pub DebugEventCallback: *mut c_void,
    pub SuppressBackgroundThread: i32,
    pub SuppressExternalCodecs: i32,
}

#[repr(C)]
pub struct GdipRect { pub X: i32, pub Y: i32, pub Width: i32, pub Height: i32 }

#[repr(C)]
pub struct BitmapData {
    pub Width: u32, pub Height: u32, pub Stride: i32, pub PixelFormat: i32,
    pub Scan0: *mut u8, pub Reserved: usize,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
// messages
pub const WM_NULL: u32 = 0x0000;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_SETTINGCHANGE: u32 = 0x001A;
pub const WM_ACTIVATEAPP: u32 = 0x001C;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_HSCROLL: u32 = 0x0114;
pub const WM_MOVE: u32 = 0x0003;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_HOTKEY: u32 = 0x0312;
pub const WM_DISPLAYCHANGE: u32 = 0x007E;
pub const WM_WINDOWPOSCHANGED: u32 = 0x0047;
pub const WM_CTLCOLORSTATIC: u32 = 0x0138;
pub const WM_DPICHANGED: u32 = 0x02E0;
pub const WM_APP: u32 = 0x8000;
pub const WM_SETFONT: u32 = 0x0030;
pub const WM_SETTEXT: u32 = 0x000C;
// mouse (tray classic v0 messages arrive in lParam)
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
// app-defined
pub const WM_TRAYICON: u32 = WM_APP + 1;

// window styles
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_EX_LAYERED: u32 = 0x0008_0000;
pub const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
pub const WS_EX_APPWINDOW: u32 = 0x0004_0000;
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;

// show window
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const SW_SHOW: i32 = 5;

// SetWindowPos
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const HWND_TOPMOST: HWND = -1isize as *mut c_void;
pub const HWND_NOTOPMOST: HWND = -2isize as *mut c_void;

// class cursor / icons
pub const IDC_ARROW: *const u16 = 32512 as usize as *const u16;
pub const IDI_APPLICATION: *const u16 = 32512 as usize as *const u16;

// menu
pub const MF_STRING: u32 = 0x0000;
pub const MF_SEPARATOR: u32 = 0x0800;
pub const MF_POPUP: u32 = 0x0010;
pub const MF_CHECKED: u32 = 0x0008;
pub const MF_GRAYED: u32 = 0x0001;
pub const MF_DISABLED: u32 = 0x0002;
pub const MF_BYCOMMAND: u32 = 0x0000;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;

// tray
pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIF_MESSAGE: u32 = 1;
pub const NIF_ICON: u32 = 2;
pub const NIF_TIP: u32 = 4;
pub const NIF_INFO: u32 = 0x10;
pub const NIIF_INFO: u32 = 1;

// layered window
pub const ULW_ALPHA: u32 = 2;
pub const AC_SRC_OVER: u8 = 0;
pub const AC_SRC_ALPHA: u8 = 1;

// GDI
pub const BI_RGB: u32 = 0;
pub const DIB_RGB_COLORS: u32 = 0;
pub const TRANSPARENT_BK: i32 = 1;
pub const OPAQUE_BK: i32 = 2;
pub const DT_CALCRECT: u32 = 0x0400;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_NOPREFIX: u32 = 0x0800;

// SPI
pub const SPI_GETNONCLIENTMETRICS: u32 = 0x0029;
pub const SPI_GETWORKAREA: u32 = 0x0030;

// metrics
pub const SM_CXSMICON: i32 = 49;
pub const SM_CYSMICON: i32 = 50;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;

// common controls
pub const ICC_BAR_CLASSES: u32 = 0x0004;
pub const ICC_WIN95_CLASSES: u32 = 0x0001;

// hotkey modifiers
pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

// virtual keys
pub const VK_SHIFT: u32 = 0x10;
pub const VK_CONTROL: u32 = 0x11;
pub const VK_MENU: u32 = 0x12;
pub const VK_LWIN: u32 = 0x5B;
pub const VK_RWIN: u32 = 0x5C;
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_LBUTTON: u32 = 0x01;

// open filename
pub const OFN_HIDEREADONLY: u32 = 0x0004;
pub const OFN_NOCHANGEDIR: u32 = 0x0008;
pub const OFN_PATHMUSTEXIST: u32 = 0x0800;
pub const OFN_FILEMUSTEXIST: u32 = 0x1000;

// combobox / trackbar messages
pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CBS_DROPDOWNLIST: u32 = 3;
pub const TBM_GETPOS: u32 = 0x0400;
pub const TBM_SETPOS: u32 = 0x0405;
pub const TBM_SETRANGE: u32 = 0x0406;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const BM_GETCHECK: u32 = 0x00F0;
pub const BST_CHECKED: usize = 1;
pub const BS_AUTOCHECKBOX: u32 = 2;
pub const CBN_SELCHANGE: u32 = 1;
pub const BN_CLICKED: u32 = 0;

// registry
pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001usize as *mut c_void;
pub const REG_SZ: u32 = 1;
pub const KEY_SET_VALUE: u32 = 0x2002;
pub const ERROR_SUCCESS: u32 = 0;

// misc
pub const CW_USEDEFAULT: i32 = i32::MIN; // 0x80000000
pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as *mut c_void;
pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_SHARE_READ: u32 = 1;
pub const FILE_SHARE_WRITE: u32 = 2;
pub const OPEN_EXISTING: u32 = 3;
pub const CREATE_ALWAYS: u32 = 2;
pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
pub const MOVEFILE_REPLACE_EXISTING: u32 = 1;

// message box
pub const MB_OK: u32 = 0;
pub const MB_OKCANCEL: u32 = 1;
pub const MB_YESNO: u32 = 4;
pub const MB_ICONERROR: u32 = 0x10;
pub const MB_ICONINFORMATION: u32 = 0x40;
pub const MB_ICONWARNING: u32 = 0x30;
pub const MB_SETFOREGROUND: u32 = 0x1_0000;
pub const IDYES: i32 = 6;
pub const IDCANCEL: i32 = 2;

// DPI awareness
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: *mut c_void =
    -4isize as *mut c_void;

// GDI+ pixel formats
pub const PIXELFORMAT_32BPP_PARGB: i32 = 0x0E200B;
pub const IMAGE_LOCK_MODE_READ: u32 = 1;

// DWM
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
pub const DWMWA_USE_IMMERSIVE_DARK_MODE_1809: u32 = 19;

// MapVirtualKey
pub const MAPVK_VK_TO_VSC: u32 = 0;

// ---------------------------------------------------------------------------
// kernel32
// ---------------------------------------------------------------------------
#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HMODULE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *const c_void;
    pub fn GetModuleFileNameW(hModule: HMODULE, lpFilename: *mut u16, nSize: u32) -> u32;
    pub fn CreateMutexW(lpMutexAttributes: *const c_void, bInitialOwner: BOOL, lpName: *const u16) -> HANDLE;
    pub fn ReleaseMutex(hMutex: HANDLE) -> BOOL;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn GetLastError() -> u32;
    pub fn GetEnvironmentVariableW(lpName: *const u16, lpBuffer: *mut u16, nSize: u32) -> u32;
    pub fn CreateFileW(lpFileName: *const u16, dwDesiredAccess: u32, dwShareMode: u32,
                       lpSecurityAttributes: *const c_void, dwCreationDisposition: u32,
                       dwFlagsAndAttributes: u32, hTemplateFile: HANDLE) -> HANDLE;
    pub fn ReadFile(hFile: HANDLE, lpBuffer: *mut u8, nNumberOfBytesToRead: u32,
                    lpNumberOfBytesRead: *mut u32, lpOverlapped: *mut c_void) -> BOOL;
    pub fn WriteFile(hFile: HANDLE, lpBuffer: *const u8, nNumberOfBytesToWrite: u32,
                     lpNumberOfBytesWritten: *mut u32, lpOverlapped: *mut c_void) -> BOOL;
    pub fn GetFileSizeEx(hFile: HANDLE, lpFileSize: *mut i64) -> BOOL;
    pub fn CopyFileW(lpExistingFileName: *const u16, lpNewFileName: *const u16,
                     bFailIfExists: BOOL) -> BOOL;
    pub fn DeleteFileW(lpFileName: *const u16) -> BOOL;
    pub fn MoveFileExW(lpExistingFileName: *const u16, lpNewFileName: *const u16,
                       dwFlags: u32) -> BOOL;
    pub fn GetTickCount64() -> u64;
}

// ---------------------------------------------------------------------------
// user32
// ---------------------------------------------------------------------------
#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> ATOM;
    pub fn CreateWindowExW(dwExStyle: u32, lpClassName: *const u16, lpWindowName: *const u16,
                           dwStyle: u32, x: i32, y: i32, nWidth: i32, nHeight: i32,
                           hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE,
                           lpParam: *const c_void) -> HWND;
    pub fn DefWindowProcW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn UpdateLayeredWindow(hWnd: HWND, hdcDst: HDC, pptDst: *mut POINT, psize: *const SIZE,
                               hdcSrc: HDC, pptSrc: *const POINT, crKey: COLORREF,
                               pblend: *const BLENDFUNCTION, dwFlags: u32) -> BOOL;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32) -> i32;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn PostMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn SendMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn RegisterHotKey(hWnd: HWND, id: i32, fsModifiers: u32, vk: u32) -> BOOL;
    pub fn UnregisterHotKey(hWnd: HWND, id: i32) -> BOOL;
    pub fn GetKeyState(nVirtKey: i32) -> i16;
    pub fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: u32,
                    lpTimerFunc: Option<unsafe extern "system" fn(HWND, u32, usize, u32)>) -> usize;
    pub fn KillTimer(hWnd: HWND, nIDEvent: usize) -> BOOL;
    pub fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, x: i32, y: i32, cx: i32, cy: i32,
                        uFlags: u32) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn SystemParametersInfoW(uiAction: u32, uiParam: u32, pvParam: *mut c_void,
                                 fWinIni: u32) -> BOOL;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    pub fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    pub fn LoadImageW(hInst: HINSTANCE, lpszName: *const u16, uType: u32, cxDesired: i32,
                      cyDesired: i32, fuLoad: u32) -> HANDLE;
    pub fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
    pub fn GetDpiForWindow(hWnd: HWND) -> u32;
    pub fn MonitorFromWindow(hWnd: HWND, dwFlags: u32) -> *mut c_void;
    pub fn MonitorFromPoint(pt: POINT, dwFlags: u32) -> *mut c_void;
    pub fn SetProcessDpiAwarenessContext(value: *mut c_void) -> BOOL;
    pub fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn IsDialogMessageW(hDlg: HWND, lpMsg: *mut MSG) -> BOOL;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
    pub fn GetKeyNameTextW(lParam: LPARAM, lpString: *mut u16, cchSize: i32) -> i32;
    pub fn EnumDisplayMonitors(hdc: HDC, lprcClip: *const RECT,
        lpfnEnum: Option<unsafe extern "system" fn(*mut c_void, HDC, *const RECT, LPARAM) -> BOOL>,
        dwData: LPARAM) -> BOOL;
    pub fn GetMonitorInfoW(hMonitor: *mut c_void, lpmi: *mut MONITORINFOEXW) -> BOOL;
    pub fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    pub fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> LONG_PTR;
    pub fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: LONG_PTR) -> LONG_PTR;
}

// ---------------------------------------------------------------------------
// gdi32
// ---------------------------------------------------------------------------
#[link(name = "gdi32")]
extern "system" {
    pub fn CreateDIBSection(hdc: HDC, pbmi: *const BITMAPINFO, usage: u32, ppvBits: *mut *mut c_void,
                            hSection: HANDLE, dwOffset: u32) -> HBITMAP;
    pub fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn CreateFontIndirectW(lf: *const LOGFONTW) -> HFONT;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetBkColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn DrawTextW(hdc: HDC, lpchText: *const u16, cchText: i32, lprc: *mut RECT, format: u32) -> i32;
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
}

// ---------------------------------------------------------------------------
// shell32
// ---------------------------------------------------------------------------
#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> BOOL;
    pub fn SHCreateDirectoryExW(pszPath: *const u16, psa: *const c_void,
                                ppsa: *mut u32) -> i32;
}

// ---------------------------------------------------------------------------
// advapi32 (registry)
// ---------------------------------------------------------------------------
#[link(name = "advapi32")]
extern "system" {
    pub fn RegCreateKeyExW(hKey: HKEY, lpSubKey: *const u16, Reserved: u32, lpClass: *const u16,
                           dwOptions: u32, samDesired: u32, lpSecurityAttributes: *const c_void,
                           phkResult: *mut HKEY, lpdwDisposition: *mut u32) -> i32;
    pub fn RegSetValueW(hKey: HKEY, lpValueName: *const u16, dwType: u32, lpData: *const u8,
                        cbData: u32) -> i32;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: *const u16) -> i32;
    pub fn RegCloseKey(hKey: HKEY) -> i32;
}

// ---------------------------------------------------------------------------
// comdlg32
// ---------------------------------------------------------------------------
#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

// ---------------------------------------------------------------------------
// comctl32
// ---------------------------------------------------------------------------
#[link(name = "comctl32")]
extern "system" {
    pub fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> BOOL;
}

// ---------------------------------------------------------------------------
// gdiplus (flat API — plain C functions, no COM)
// ---------------------------------------------------------------------------
#[link(name = "gdiplus")]
extern "system" {
    pub fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput,
                          output: *mut c_void) -> i32;
    pub fn GdiplusShutdown(token: usize);
    pub fn GdipCreateBitmapFromFile(filename: *const u16, bitmap: *mut *mut c_void) -> i32;
    pub fn GdipGetImageWidth(image: *mut c_void, width: *mut u32) -> i32;
    pub fn GdipGetImageHeight(image: *mut c_void, height: *mut u32) -> i32;
    pub fn GdipDisposeImage(image: *mut c_void) -> i32;
}

// GdipLockBits / GdipUnlockBits are missing from some import libraries
// (llvm-mingw), so they are resolved dynamically from the loaded gdiplus.dll.
pub fn GdipLockBits_dyn()
    -> Option<unsafe extern "system" fn(*mut c_void, *const GdipRect, u32, i32,
                                         *mut BitmapData) -> i32> {
    unsafe { proc_addr(b"GdipLockBits\0") }.map(|p| unsafe { std::mem::transmute(p) })
}

pub fn GdipUnlockBits_dyn()
    -> Option<unsafe extern "system" fn(*mut c_void, *const BitmapData) -> i32> {
    unsafe { proc_addr(b"GdipUnlockBits\0") }.map(|p| unsafe { std::mem::transmute(p) })
}

unsafe fn proc_addr(name: &[u8]) -> Option<*const c_void> {
    let h = GetModuleHandleW(wide("gdiplus.dll").as_ptr());
    if h == core::ptr::null_mut() { return None; }
    let p = GetProcAddress(h, name.as_ptr());
    if p == core::ptr::null() { None } else { Some(p) }
}

pub fn SetProcessDpiAware_dyn() -> Option<unsafe extern "system" fn() -> BOOL> {
    unsafe {
        let h = GetModuleHandleW(wide("user32.dll").as_ptr());
        if h == core::ptr::null_mut() { return None; }
        let p = GetProcAddress(h, b"SetProcessDpiAware\0".as_ptr());
        if p == core::ptr::null() { None } else { Some(std::mem::transmute(p)) }
    }
}

// ---------------------------------------------------------------------------
// dwmapi
// ---------------------------------------------------------------------------
#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, pvAttribute: *const c_void,
                                 cbAttribute: u32) -> i32;
}

// ---------------------------------------------------------------------------
// UTF-16 helpers (platform independent)
// ---------------------------------------------------------------------------
pub fn wide(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

pub fn wide_slice(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

pub fn from_utf16(s: &[u16]) -> String {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf16_lossy(&s[..end])
}

/// Fill a fixed-size UTF-16 buffer (truncating safely) from a &str.
pub fn copy_into_buf(buf: &mut [u16], s: &str) {
    for (i, b) in s.encode_utf16().take(buf.len().saturating_sub(1)).enumerate() {
        buf[i] = b;
    }
    let n = s.encode_utf16().count();
    if n < buf.len() {
        buf[n] = 0;
    }
}
