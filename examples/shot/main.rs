//! Plays SPECS on the app's windows and photographs one.

#[cfg(target_os = "macos")]
fn main() {
    use std::time::Duration;

    use ely_gpui_component::Assets;
    use elyterminal::{
        app,
        settings::window::SettingsWindow,
        term::{Launch, Quote},
    };
    use gpui::AnyWindowHandle;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("elyterminal=info"))
        .init();
    let mut args = std::env::args().skip(1).peekable();
    let keys: Vec<String> = if args.peek().map(String::as_str) == Some("--keys") {
        args.next();
        let keys = args.next().expect("keys after --keys");
        keys.split_whitespace().map(String::from).collect()
    } else {
        Vec::new()
    };
    let out = args.next().expect("a png path");
    let program = args.next().expect("a program");
    let launch = Launch {
        program: Some((program, args.collect())),
        cwd: std::env::current_dir().expect("a current folder"),
        env: Vec::new(),
        reports: false,
        quote: Quote::Posix,
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut gpui::App| {
            ely_gpui_component::init(cx).expect("Ely starts");
            app::start(cx).expect("the app starts");
            let main: AnyWindowHandle = app::open(launch, cx).expect("the window opens").into();
            cx.spawn(async move |cx| {
                let number = |aim: AnyWindowHandle, cx: &mut gpui::AsyncApp| {
                    let number = aim.update(cx, |_, window, _| capture::number(window));
                    number.expect("the window lives").expect("a window number")
                };
                let mut aim = main;
                let mut snaps = 0;
                capture::front(number(aim, cx)).expect("the window on screen");
                cx.background_executor().timer(Duration::from_secs(1)).await;
                for spec in &keys {
                    match spec.as_str() {
                        "wait" => cx.background_executor().timer(Duration::from_secs(1)).await,
                        "snap" => {
                            snaps += 1;
                            let frames = capture::frames(number(aim, cx)).expect("the frames");
                            for (index, (child, _)) in frames.into_iter().skip(1).enumerate() {
                                let path =
                                    out.replace(".png", &format!(".child{index}.snap{snaps}.png"));
                                capture::snapshot(child, std::path::Path::new(&path))
                                    .expect("a child's picture");
                                log::info!(
                                    "shot: snap {snaps}, child window {child}, wrote {path}"
                                );
                            }
                        }
                        "to:main" => {
                            aim = main;
                            capture::front(number(aim, cx)).expect("the window on screen");
                        }
                        "to:settings" => {
                            let windows = cx.update(|cx| cx.windows());
                            let settings = windows
                                .into_iter()
                                .find(|window| window.downcast::<SettingsWindow>().is_some());
                            aim = settings.expect("a settings window");
                            capture::front(number(aim, cx)).expect("the window on screen");
                        }
                        "close" => {
                            aim.update(cx, |_, window, _| window.remove_window())
                                .expect("the window lives");
                            log::info!("shot: closed the window");
                            aim = main;
                        }
                        spec if spec.starts_with("drop@") => {
                            let paths =
                                std::env::var("SHOT_DROP").expect("SHOT_DROP names the paths");
                            let event = typing::drop(&spec[5..], &paths).expect("a drop");
                            // A drag moves over frames before it lands.
                            for step in event {
                                aim.update(cx, |_, window, cx| window.dispatch_event(step, cx))
                                    .expect("the window lives");
                                cx.background_executor()
                                    .timer(Duration::from_millis(120))
                                    .await;
                            }
                            log::info!("shot: dropped {paths:?} at {}", &spec[5..]);
                        }
                        spec => typing::post(spec, number(aim, cx)).expect("a key event"),
                    }
                    if spec.starts_with("to:") {
                        log::info!("shot: {spec}");
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(120))
                        .await;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let title = aim.update(cx, |_, window, _| window.window_title());
                let title = title.expect("the window lives");
                capture::snapshot(number(aim, cx), std::path::Path::new(&out)).expect("a picture");
                let frames = capture::frames(number(aim, cx)).expect("the window's frames");
                log::info!("shot: the window at {:?}", frames[0].1);
                for (index, (child, at)) in frames.into_iter().skip(1).enumerate() {
                    let path = out.replace(".png", &format!(".child{index}.png"));
                    capture::snapshot(child, std::path::Path::new(&path))
                        .expect("a child's picture");
                    log::info!("shot: child window {child} at {at:?}, wrote {path}");
                }
                let views = capture::views(number(aim, cx)).expect("the window's views");
                log::info!("shot: wrote {out}, titled {title:?}, over {views:?}");
                for line in capture::menus() {
                    log::info!("shot: menu {line}");
                }
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
}

#[cfg(target_os = "macos")]
mod capture;

#[cfg(target_os = "macos")]
mod typing;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("shot photographs windows on macOS only");
    std::process::exit(1);
}
