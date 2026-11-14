// A release build on Windows opens no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context as _;
use ely_gpui_component::Assets;
use elyterminal::{
    app,
    sessions::Program,
    settings::{self, SettingsStore, StartIn},
    term::shell::{ShellIntegration, Start, command, launch},
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
    #[cfg(windows)]
    if app::args::embedded(std::env::args_os().skip(1)) {
        log::info!("app: COM started this copy for a console session");
        app::start(cx)?;
        elyterminal::defterm::listen(cx)?;
        return wait_for_handoff(cx);
    }
    let asked = app::args::parse(std::env::args_os().skip(1))?;
    app::start(cx)?;
    #[cfg(windows)]
    if let Err(error) = elyterminal::defterm::listen(cx) {
        log::error!("{error:#}");
    }
    let settings = SettingsStore::get(cx);
    let (launch, program) = if let Some((shell, args)) = asked.command {
        let folder = match asked.folder {
            Some(folder) => folder,
            None => std::env::current_dir().context("app: no current folder for -e")?,
        };
        let launch = command(&shell, &args, folder)?;
        let program = Program {
            shell,
            args,
            written: true,
        };
        (launch, program)
    } else {
        // No session shows yet, so Session's folder means home.
        let start = match (asked.folder, settings.start_in) {
            (Some(folder), _) => Start::In(folder),
            (None, StartIn::Folder) => Start::In(settings::start_folder(&settings.start_folder)?),
            (None, StartIn::Home | StartIn::Session) => Start::Home,
        };
        let launch = launch(
            &settings.shell,
            &[],
            start,
            &cx.global::<ShellIntegration>().0,
        )?;
        let program = Program {
            shell: settings.shell.clone(),
            ..Program::default()
        };
        (launch, program)
    };
    app::open(launch, program, cx)?;
    cx.activate(true);
    Ok(())
}

/// A copy COM started that gets no session in 30 seconds stops.
#[cfg(windows)]
fn wait_for_handoff(cx: &mut App) -> anyhow::Result<()> {
    cx.spawn(async |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_secs(30))
            .await;
        if cx.update(|cx| cx.windows().is_empty()) {
            log::error!("app: COM started this copy, and no console session came in 30 seconds");
            std::process::exit(1);
        }
    })
    .detach();
    Ok(())
}

fn main() {
    if let Some(code) = elyterminal::control::command_line() {
        std::process::exit(code);
    }
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
