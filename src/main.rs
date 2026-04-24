mod core;
mod fs_adapter;
mod persistence;
mod ui;

use gpui::AppContext;
use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_component::Root;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1440.0), px(860.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Offline-First File Explorer".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let content = cx.new(|cx| ui::AppView::new(window, cx));
                cx.new(|cx| Root::new(content, window, cx))
            },
        )
        .expect("failed to open main window");
    });
}
