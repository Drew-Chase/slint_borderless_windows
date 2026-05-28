use slint::{PhysicalSize, WindowSize};
use slint_borderless_windows::TitlebarSetup;

slint::include_modules!();
fn main() -> Result<(), slint::PlatformError> {
    let app = MainWindow::new().expect("Failed to create main window");
    app.window().set_size(WindowSize::Physical(PhysicalSize::new(800,600)));

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
