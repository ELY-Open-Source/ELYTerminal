// A release build on Windows opens no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context as _;
use ely_gpui_component::Assets;
use elyterminal::{
    app,
    sessions::{
        Program,
        saved::{self, Saved},
    },
    settings::{self, SettingsStore, StartIn},
    term::shell::{ShellIntegration, Start, command, launch},
    workspace::Keeps,
};
use futures::StreamExt as _;
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

/// Starts the app; on macOS a folder `dropped` on the icon before it started names the first session's.
fn run(dropped: &mut Vec<String>, cx: &mut App) -> anyhow::Result<()> {
    #[cfg(windows)]
    if app::args::embedded(std::env::args_os().skip(1)) {
        log::info!("app: COM started this copy for a console session");
        app::start(cx)?;
        elyterminal::defterm::listen(cx)?;
        return wait_for_handoff(cx);
    }
    let mut asked = app::args::parse(std::env::args_os().skip(1))?;
    if asked.folder.is_none() && !dropped.is_empty() {
        let url = dropped.remove(0);
        match app::dropped_folder(&url) {
            Ok(folder) => {
                log::info!("app: starts in {}, dropped on the icon", folder.display());
                asked.folder = Some(folder);
            }
            Err(error) => log::error!("{error:#}"),
        }
    }
    app::start(cx)?;
    #[cfg(windows)]
    if let Err(error) = elyterminal::defterm::listen(cx) {
        log::error!("{error:#}");
    }
    // A run given a folder or a program neither opens the sessions left nor keeps its own.
    let keeps = asked.folder.is_none() && asked.command.is_none();
    let left = match keeps.then(|| left(cx)).transpose() {
        Ok(left) => left.flatten(),
        Err(error) => {
            log::error!("{error:#}; this run keeps no sessions");
            cx.set_global(Keeps(false));
            None
        }
    };
    cx.try_global::<Keeps>()
        .is_none()
        .then(|| cx.set_global(Keeps(keeps)));
    if let Some((saved, first)) = left {
        match launch(
            first.0.as_str(),
            &first.1,
            Start::In(first.2),
            &cx.global::<ShellIntegration>().0,
        ) {
            Ok(started) => {
                let program = Program {
                    shell: first.0,
                    args: first.1,
                    written: false,
                };
                let main = app::open(started, program, cx)?;
                main.update(cx, |workspace, window, cx| {
                    workspace.restore(saved, window, cx)
                })?;
                cx.activate(true);
                return Ok(());
            }
            Err(error) => log::error!("sessions: the first pane left does not start: {error:#}"),
        }
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

/// The sessions the last run left, while the settings ask for them, and their first pane's program, arguments and folder.
#[allow(clippy::type_complexity)]
fn left(cx: &App) -> anyhow::Result<Option<(Saved, (String, Vec<String>, std::path::PathBuf))>> {
    // Read whatever the setting says, so a file that does not read is never written over.
    let Some(saved) = saved::load(&saved::path()?)? else {
        return Ok(None);
    };
    if !SettingsStore::get(cx).restore_sessions {
        return Ok(None);
    }
    let Some(saved) = saved.kept(|folder| folder.is_dir()) else {
        log::info!("sessions: none left opens");
        return Ok(None);
    };
    let (shell, args, folder) = saved.sessions[0].layout.panes()[0];
    let first = (shell.to_string(), args.to_vec(), folder.to_path_buf());
    Ok(Some((saved, first)))
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
    let application = application().with_assets(Assets);
    // AppKit hands a folder dropped on the icon over before the app starts, and while it runs.
    let (send, mut urls) = futures::channel::mpsc::unbounded::<String>();
    application.on_open_urls(move |dropped| {
        for url in dropped {
            if let Err(error) = send.unbounded_send(url) {
                log::error!("app: a folder dropped on the icon is lost: {error}");
            }
        }
    });
    application.run(move |cx: &mut App| {
        ely_gpui_component::init(cx).expect("Ely failed to start");
        let mut dropped = Vec::new();
        while let Ok(url) = urls.try_recv() {
            dropped.push(url);
        }
        if let Err(error) = run(&mut dropped, cx) {
            log::error!("{error:#}");
            std::process::exit(1);
        }
        cx.spawn(async move |cx| {
            let later = futures::stream::iter(dropped).chain(urls);
            futures::pin_mut!(later);
            while let Some(url) = later.next().await {
                if let Err(error) = cx.update(|cx| app::dropped(&url, cx)) {
                    log::error!("{error:#}");
                }
            }
        })
        .detach();
    });
}
