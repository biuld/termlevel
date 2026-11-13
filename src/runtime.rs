use nsh_core::prelude::*;

/// Build information and the application environment.
#[module]
pub struct Runtime;

#[module]
impl Runtime {
    /// The application version compiled into the native crate.
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// Enables the explicit deterministic preview entry for UI-only testing.
    pub fn preview_mode() -> bool {
        std::env::var("TERMLEVEL_PREVIEW").as_deref() == Ok("1")
    }

    /// Existing initial directory for a new workspace.
    pub fn cwd() -> String {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .to_string_lossy()
            .into_owned()
    }
}

pub(crate) fn terminal_cell_width(font: &str, size: f32) -> f32 {
    let css = match font {
        "monospace" | "ui-monospace" => format!("{size}px {font}"),
        _ => format!("{size}px \"{}\"", font.replace('"', "")),
    };
    nsh_core::element::measure_text(&css, "0000000000").0 / 10.0
}

#[derive(Clone, Copy, PartialEq, Props)]
pub struct TerminalMetricsProps {
    /// Terminal font family, shared with the renderer.
    pub font_family: Prop<SharedString>,
    /// Terminal font size in points.
    pub font_size: Prop<Points>,
}

#[derive(Debug, Clone, PartialEq, Event)]
pub enum TerminalMetricsEvent {
    /// The measured width of one terminal cell in points.
    #[args(width)]
    Change(f32),
}

/// Measure in the native reactive view scope, where the platform text engine is available.
#[component]
pub fn TerminalMetrics(
    props: TerminalMetricsProps,
    cx: Scope<TerminalMetricsEvent>,
) -> impl IntoElement {
    effect(move || {
        let width = terminal_cell_width(&props.font_family.get(), props.font_size.get().0);
        if width > 0.0 {
            cx.emit(TerminalMetricsEvent::Change(width));
        }
    });
    view()
        .absolute()
        .w(px(0.0))
        .h(px(0.0))
        .pointer_events(PointerEvents::None)
}
