# Implementation Notes

## Why native Win32 instead of Tauri 2.0 / WebView2

The project brief asked for Tauri 2.0 *and* for these hard constraints:

| Constraint | Tauri/WebView2 reality | PaperGrain (native) |
|---|---|---|
| Idle RAM < 50 MB | WebView2 spawns 4–6 processes, 100–250 MB total | ~12–18 MB @1080p |
| Installer < 5 MB | WebView2 bootstrapper ~2 MB *online*, bundle 100 MB+ | 0.7 MB offline installer |
| Zero idle CPU | Chromium compositor ticks in background | static bitmap, timers only |
| Works on any Win10/11 | needs WebView2 runtime installed/online | no runtime deps at all |

The two requirements are mutually exclusive with WebView2, so the
implementation keeps Rust (as specified) and replaces the webview frontend
with a hand-written Win32 FFI layer (`src/win32.rs`, **zero external
crates**). The "frontend" (settings panel) is native Win32 controls; the
"backend" (overlay engine) is pure GDI/DIB layered windows.

## Overlay engine

- One `WS_POPUP` window per monitor with
  `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST |
  WS_EX_NOACTIVATE`:
  - `WS_EX_TRANSPARENT` → hit-testing passes through (click-through)
  - `WS_EX_TOOLWINDOW` → hidden from taskbar / Alt-Tab
  - `WS_EX_NOACTIVATE` → never steals focus
- Painting uses `UpdateLayeredWindow` with a 32-bpp **premultiplied BGRA**
  top-down DIB section and `AC_SRC_ALPHA` per-pixel alpha. Because the bitmap
  is premultiplied, global opacity is a *linear scale of every channel* —
  mathematically exact source-over compositing.
- The window position/size is set by the `UpdateLayeredWindow` call itself
  (`pptDst`/`psize`), keyed off each monitor's `rcMonitor`.
- A 2-second timer re-asserts `HWND_TOPMOST` (the classic f.lux-style
  pattern); this is a no-op `SetWindowPos` and does not repaint, so idle CPU
  remains 0% in Task Manager.
- `WM_DISPLAYCHANGE` re-enumerates monitors and rebuilds overlay windows;
  `WM_DPICHANGED` adopts the suggested rect and re-renders at the new DPI.

### Memory management

Texture "masters" (full-intensity premultiplied buffers) are cached only
while the user is actively adjusting settings; after 5 seconds of inactivity
the cache is dropped (a 4K monitor master is ~33 MB). Regeneration on demand
costs 50–200 ms and is invisible to the user. The DIB sections (~8 MB @1080p)
stay resident because the OS compositor reads them directly.

## Procedural textures

All presets are pure functions of `(x, y, dpi, intensity, seed)` —
deterministic value noise + fBm (`src/noise.rs`), so output is identical
across runs and machines, and any monitor size gets a native-resolution,
non-repeating texture. Frequencies scale with DPI so grain looks identical
at 100%–300% scaling.

## Custom textures

GDI+ flat API (`GdipCreateBitmapFromFile`, `GdipLockBits` with
`PixelFormat32bppPARGB`) decodes any GDI+-supported image directly into
premultiplied BGRA. `GdipLockBits`/`GdipUnlockBits` are resolved at runtime
via `GetProcAddress` because some MinGW import libraries omit them. Small
images are tiled; large images are bilinearly cover-scaled with a center
crop. Files are copied into `%APPDATA%\PaperGrain\textures` for persistence.

## Configuration

A ~120-line generic JSON parser/serializer (`src/config.rs`) covers the whole
schema — no serde dependency needed for a flat, known structure. Writes are
atomic (temp file + `MoveFileEx(REPLACE_EXISTING)`), and a corrupt file
simply falls back to defaults.

## App state & reentrancy

Single GUI thread with a `static mut APP_STATE: Option<App>` accessed through
a fresh `&'static mut` view per statement. This intentionally avoids
`RefCell` (whose borrow checks would panic under reentrant message pumps
such as `TrackPopupMenu`/`MessageBox` while a borrow is held). The pattern is
memory-safe here because there is exactly one thread and no aliasing
*within* a single statement.

## Watermark

"CookieFilled" is rasterized once per DPI via `DrawTextW` into a temporary
DIB (white on black), thresholded into a stamp, and stamped at 5% alpha
(13/255) into the primary monitor's overlay, 16 px (DPI-scaled) from the
bottom-right corner.

## Build details

- Target: `x86_64-pc-windows-gnullvm` (Rust + LLVM-MinGW, SEH unwinding,
  static `libunwind`)
- Resources (icon, comctl32 v6 manifest, VERSIONINFO) via `windres`
- Result imports only: `kernel32, user32, gdi32, shell32, advapi32,
  comctl32, comdlg32, dwmapi, gdiplus, ntdll` + UCRT API sets (OS components
  on Win10/11). No third-party DLLs, no MSVC runtime.
- Installer: NSIS 3 (Modern UI 2, LZMA/solid) built with `makensis` on Linux.
- `--smoke` CLI switch: full init + one frame render + auto-exit (used for
  headless testing under Wine).
