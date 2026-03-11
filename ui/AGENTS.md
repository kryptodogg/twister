# UI Styling Protocol: Spatial Computing & The "Credit Card" Footprint

Agents working on the Toto Core UI (whether React prototype, Slint, or other frontends) MUST adhere to the following strict spatial computing guidelines. This ensures the widget functions as a true floating desktop instrument.

## 1. Resolution Independence & Spatial Scaling
*   **NEVER USE HARDCODED PIXELS (`px`).**
*   **Structural Scaling:** Use `rem` for all primary layout dimensions, margins, and padding. This allows the entire instrument to scale simply by adjusting the root container's base font size.
*   **Local Component Scaling:** Use `em` for fonts, icons, and tightly coupled internal borders/shadows so they scale relative to their parent container.
*   **Aspect Ratio Preservation:** To enforce the compact "Credit Card" footprint (approx 1.58 to 1.7 ratio), use `aspect-ratio` properties on the main structural blocks instead of fixed widths/heights. The container should fluidly expand vertically ONLY when collapsable menus (e.g., Settings/Active Denial) are toggled open.
*   **Typography:** The primary UI font is **Inter Medium 500**. Only use monospaced fonts (e.g., Space Mono, JetBrains Mono) for rapidly changing telemetry values to prevent text jitter.

## 2. Platform-Native Transparency & Blur (Glassmorphism)

To achieve the "floating" spatial aesthetic, the UI relies on deep OS-level compositor integration rather than simple CSS opacity. When translating these designs to native windowing frameworks (like Winit or Slint's platform layer), you must request the correct OS primitives:

### Windows 11 (Desktop Window Manager - DWM)
*   **Mica / Acrylic:** You must configure the window to request the `DWMWA_SYSTEMBACKDROP_TYPE` attribute.
    *   Set it to `DWMSBT_TRANSIENTWINDOW` for Acrylic (deep blur, used for the expanded settings pane).
    *   Set it to `DWMSBT_MAINWINDOW` for Mica (opaque tinting based on wallpaper, good for the main header).
*   **Corner Radii:** Request rounded corners natively via `DWMWA_WINDOW_CORNER_PREFERENCE` set to `DWMWCP_ROUND`.
*   **Borderless/Frameless:** The window must be borderless (`WS_POPUP`), but ensure `WS_THICKFRAME` is maintained if edge-resizing is required.

### Linux / GNOME (Wayland & X11)
*   **Visual Configuration:** The window *must* be created with a 32-bit visual (RGBA) to support an alpha channel. On X11, this requires querying XRender for an appropriate visual with depth 32. On Wayland, ensure the EGL/Vulkan surface is configured for `B8G8R8A8` or equivalent alpha-capable format.
*   **Compositor Blur:** To achieve the frosted glass look natively:
    *   **KDE Plasma / KWin:** Set the `_KDE_NET_WM_BLUR_BEHIND_REGION` X11 property to instruct the compositor to blur the region beneath the window.
    *   **GNOME / Mutter:** Wayland protocols for background blur are still fragmented. Fall back to rendering a pseudo-blur shader in the `WgpuShaderZone` or rely on the CSS/Slint `backdrop-filter: blur()` equivalent if the renderer supports reading back the framebuffer.

## 3. The Core Stack
*   **Header:** Minimal vertical footprint. Contains the Anomaly Score and Neural Auto-Steer.
*   **Canvas:** The primary visualization zone (`WgpuShaderZone`). Receives the majority of the collapsed vertical space.
*   **Telemetry Strip:** Bottom bar containing the miniature Learning Loss chart and DVR recording status.
*   **Fold-out Settings:** Expanding downwards, pushing the window bounds natively. Houses Active Denial toggles and Probe tools.
