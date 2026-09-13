# Changelog

## 1.0.0 (2026-09-11)

Initial release.

- Transparent, click-through, always-on-top paper texture overlay
  (per-pixel-alpha layered windows, one per monitor)
- Four procedural texture presets: fine grain, craft paper, notebook,
  parchment (DPI-aware, generated at native resolution)
- Custom texture loading (PNG / JPG / BMP / GIF via GDI+), tiled or
  cover-scaled automatically
- Opacity 10-90% (1% steps) and grain intensity controls
- Multi-monitor support with per-display toggle
- Global hotkey toggle (default Ctrl+Shift+P, configurable)
- System tray app with full context menu and Explorer-restart recovery
- Compact dark/light settings panel
- "CookieFilled" watermark (5% opacity, bottom-right, optional)
- Optional auto-start with Windows (HKCU Run)
- JSON configuration in %APPDATA%\PaperGrain
- ~420 KB single-file executable, no runtime dependencies
