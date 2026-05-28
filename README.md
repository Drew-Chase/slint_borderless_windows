# slint-borderless-windows

A Rust library that provides a native-feeling borderless window with a custom titlebar for [Slint](https://slint.dev/) applications on Windows.

## Features

- Removes the default OS window frame (`no-frame: true`) while preserving native behavior
- Windows 11 DWM drop shadow and rounded corners via `DwmExtendFrameIntoClientArea` and `DwmSetWindowAttribute`
- Edge and corner resize hit-testing via a WinAPI window subclass (`WM_NCHITTEST`)
- Ready-to-use `Titlebar` Slint component with minimize, maximize/restore, and close buttons
- Drag-to-move via winit's `drag_window()`
- Double-click titlebar to toggle maximize/restore

## Requirements

- Windows (the DWM and subclass APIs are Windows-only)
- Rust edition 2024
- Slint 1.16.1+

## Usage

### 1. Add the dependency

In your `Cargo.toml`:

```toml
[dependencies]
slint_borderless_windows = { path = "path/to/slint_borderless_windows" }
slint = { version = "1.16.1", features = ["backend-winit"] }
```

### 2. Import the Titlebar component in your `.slint` file

```slint
import { Titlebar, WindowControls } from "path/to/titlebar.slint";

// Re-export WindowControls so it is accessible from Rust
export { WindowControls }

export component MainWindow inherits Window {
    no-frame: true;

    VerticalLayout {
        padding: 0px;
        spacing: 0px;

        Titlebar {
            title: "My App";
        }

        // Your content here
    }
}
```

Setting `no-frame: true` on the root `Window` removes the OS chrome. The `Titlebar` component replaces it.

### 3. Wire up the callbacks in Rust

```rust
use slint_borderless_windows::TitlebarSetup;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = MainWindow::new().expect("Failed to create main window");

    let frame = app.as_weak().setup_borderless().expect("Failed to setup custom frame");
    let frame_maximize = frame.clone();
    let frame_close = frame.clone();
    let frame_drag = frame.clone();
    let frame_dblclick = frame.clone();

    app.global::<WindowControls>().on_maximize(move || frame_maximize.toggle_maximized());
    app.global::<WindowControls>().on_close(move || frame_close.close());
    app.global::<WindowControls>().on_drag(move || frame_drag.drag());
    app.global::<WindowControls>().on_double_click(move || frame_dblclick.toggle_maximized());
    app.global::<WindowControls>().on_minimize(move || frame.minimize());

    app.run()?;
    Ok(())
}
```

`setup_borderless()` applies the DWM styling and installs the resize subclass. It returns a `WindowFrame` handle that exposes the window control methods.

## API

### `TitlebarSetup` trait

Implemented on `slint::Weak<T>`. Call `setup_borderless()` after creating your component to apply the custom frame.

```rust
fn setup_borderless(&self) -> Result<WindowFrame<T>, slint::PlatformError>
```

### `WindowFrame<T>`

A lightweight, cloneable handle (backed by `slint::Weak<T>`) for controlling the window.

| Method | Description |
|---|---|
| `minimize()` | Minimizes the window |
| `maximize(bool)` | Sets maximized state explicitly |
| `toggle_maximized()` | Toggles between maximized and restored |
| `close()` | Quits the Slint event loop |
| `drag()` | Initiates a native window drag-move |

### `Titlebar` component (`titlebar.slint`)

A 32px tall titlebar strip. Drop it at the top of a `VerticalLayout`.

| Property | Type | Default | Description |
|---|---|---|---|
| `title` | `string` | `"Application"` | Text displayed in the titlebar |
| `icon` | `image` | _(none)_ | Optional icon shown left of the title |

### `WindowControls` global (`titlebar.slint`)

Bridges the Slint UI to Rust. Wire up each callback from Rust as shown above.

| Callback | Triggered by |
|---|---|
| `minimize()` | Minimize button click |
| `maximize()` | Maximize/restore button click |
| `close()` | Close button click |
| `drag()` | Mouse-down on the titlebar drag area |
| `double-click()` | Double-click on the titlebar drag area |

| Property | Type | Description |
|---|---|---|
| `maximized` | `bool` | Controls which icon the maximize button shows (square vs. restore). Set this from Rust when the window state changes. |

## How it works

`setup_borderless()` performs three things on the underlying Win32 window handle:

1. **Rounded corners** — calls `DwmSetWindowAttribute` with `DWMWCP_ROUND`
2. **Drop shadow** — calls `DwmExtendFrameIntoClientArea` with a 1px bottom margin, which is enough for DWM to render a shadow without exposing any native frame
3. **Resize hit-testing** — installs a window subclass via `SetWindowSubclass` that intercepts `WM_NCHITTEST` and returns the correct `HT*` values for all eight resize edges and corners

`WM_NCCALCSIZE` is also intercepted and returns `0` when `wParam != 0`, which tells Windows the entire window rect is client area — removing all non-client chrome.

## Example

A full working example is in `examples/basic_example/`. Build and run it with:

```sh
cd examples/basic_example
cargo run
```
