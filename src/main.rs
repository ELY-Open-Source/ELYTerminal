// A release build on Windows opens no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ely_gpui_component::Assets;
use elyterminal::{
    app,
    settings::{self, SettingsStore, StartIn},
    term::shell::{ShellIntegration, Start, launch},
};
use gpui::App;

/// A Linux app with no display server fails at start instead of running headless.
#[cfg(target_os = "linux")]
fn application() -> gpui::Application {
    gpui_platform::linux(gpui::WindowingModes::WAYLAND | gpui::WindowingModes::X11)
}

#[cfg(not(target_os = "linux"))]
fn application() -> gpui::Application {
    gpui_platform::application()
}

fn run(cx: &mut App) -> anyhow::Result<()> {
    app::start(cx)?;
    let settings = SettingsStore::get(cx);
    // No session shows yet, so Session's folder means home.
    let start = match settings.start_in {
        StartIn::Folder => Start::In(settings::start_folder(&settings.start_folder)?),
        StartIn::Home | StartIn::Session => Start::Home,
    };
    let launch = launch(&settings.shell, start, &cx.global::<ShellIntegration>().0)?;
    app::open(launch, cx)?;
    cx.activate(true);
    Ok(())
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("elyterminal=info,ely_gpui_component=info"),
    )
    .init();
    application().with_assets(Assets).run(|cx: &mut App| {
        ely_gpui_component::init(cx).expect("Ely failed to start");
        if let Err(error) = run(cx) {
            log::error!("{error:#}");
            std::process::exit(1);
        }
    });
}
