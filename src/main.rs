use clap::Parser;
use clap::Subcommand;
use gtk4::Application;
use gtk4::prelude::*;
use log::info;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use zbus::blocking::{Connection, Proxy};

mod config;
mod ipc;
mod protocols;
mod runtime;
mod ui;

const APP_ID: &str = "xyz.yrwq.lush";

#[derive(Parser)]
#[command(name = "lush")]
#[command(about = "\n\nwhile !light.exists() { bloom.anyway(); }\n// :)", long_about = None)]
struct Cli {
    #[arg(short = 'c', long = "config", value_name = "file")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Daemon,
    Open { name: String },
    Close { name: String },
    Toggle { name: String },
    Show { name: String },
    Hide { name: String },
    List,
    Reload,
    ReloadCss,
    Ping,
}

fn main() {
    let cli = Cli::parse();
    env_logger::builder().format_timestamp(None).init();
    if let Some(command) = &cli.command
        && !matches!(command, Command::Daemon)
    {
        if let Err(err) = run_client(command) {
            eprintln!("lush: {}", err);
            std::process::exit(1);
        }
        return;
    }

    info!("starting lush daemon");
    run_daemon(cli.config.as_ref());
}

fn run_daemon(config_path: Option<&PathBuf>) {
    let resolved_config_path = resolve_config_path(config_path);
    let initial_loaded = load_config_or_exit(Some(&resolved_config_path));
    let app = Application::builder().application_id(APP_ID).build();
    let initial_loaded = Rc::new(initial_loaded);
    let resolved_config_path = Rc::new(resolved_config_path);

    app.connect_activate(move |app| {
        let session = ui::build_windows(app, &initial_loaded);
        let state = Rc::new(RefCell::new(DaemonState {
            config_path: (*resolved_config_path).clone(),
            session,
        }));
        let (tx, rx) = async_channel::unbounded();
        ipc::control::start_service(tx);
        let app = app.clone();
        let state = state.clone();

        glib::MainContext::default().spawn_local(async move {
            while let Ok(request) = rx.recv().await {
                handle_control_request(state.as_ref(), &app, request);
            }
        });
    });

    app.run_with_args(&[APP_ID]);
}

struct DaemonState {
    config_path: PathBuf,
    session: ui::UiSession,
}

fn handle_control_request(
    state: &RefCell<DaemonState>,
    app: &Application,
    request: ipc::control::ControlRequest,
) {
    match request {
        ipc::control::ControlRequest::OpenWindow(name) => {
            state
                .borrow()
                .session
                .runtime
                .queue_app_command(runtime::lua_runtime::AppCommand::Open(name));
        }
        ipc::control::ControlRequest::CloseWindow(name) => {
            state
                .borrow()
                .session
                .runtime
                .queue_app_command(runtime::lua_runtime::AppCommand::Close(name));
        }
        ipc::control::ControlRequest::ToggleWindow(name) => {
            state
                .borrow()
                .session
                .runtime
                .queue_app_command(runtime::lua_runtime::AppCommand::Toggle(name));
        }
        ipc::control::ControlRequest::SetWindowVisible(name, visible) => {
            state
                .borrow()
                .session
                .runtime
                .queue_app_command(runtime::lua_runtime::AppCommand::SetVisible(name, visible));
        }
        ipc::control::ControlRequest::ListWindows(reply) => {
            let mut names = state.borrow().session.runtime.window_names();
            names.sort();
            let _ = reply.send(names);
        }
        ipc::control::ControlRequest::ReloadConfig(reply) => {
            let result = reload_config_in_process(state.borrow_mut(), app);
            let _ = reply.send(result);
        }
        ipc::control::ControlRequest::ReloadCss(reply) => {
            let _ = reply.send(state.borrow().session.style.reload());
        }
        ipc::control::ControlRequest::Ping(reply) => {
            let _ = reply.send("pong".to_string());
        }
    }
}

fn load_config_or_exit(cfg: Option<&PathBuf>) -> config::LoadedConfig {
    let script_path = cfg.cloned().unwrap_or_else(config::find_config);
    info!("loading config: {:?}", script_path);

    match config::load(&script_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("lush: config error:\n  {}", err);
            std::process::exit(1);
        }
    }
}

fn resolve_config_path(cfg: Option<&PathBuf>) -> PathBuf {
    cfg.cloned().unwrap_or_else(config::find_config)
}

fn reload_config_in_process(
    mut state: std::cell::RefMut<'_, DaemonState>,
    app: &Application,
) -> Result<(), String> {
    let loaded =
        config::load(&state.config_path).map_err(|err| format!("config reload failed: {}", err))?;
    ui::reconfigure_windows(app, &mut state.session, &loaded.app);
    Ok(())
}

fn run_client(command: &Command) -> Result<(), String> {
    let connection = Connection::session()
        .map_err(|err| format!("failed to connect to session bus: {}", err))?;
    let proxy = Proxy::new(
        &connection,
        ipc::control::BUS_NAME,
        ipc::control::BUS_PATH,
        ipc::control::BUS_INTERFACE,
    )
    .map_err(|err| format!("daemon not reachable; start `lush daemon` first ({})", err))?;

    match command {
        Command::Daemon => {}
        Command::Open { name } => {
            proxy
                .call::<_, _, ()>("OpenWindow", &(name.clone(),))
                .map_err(|err| format!("open failed: {}", err))?;
        }
        Command::Close { name } => {
            proxy
                .call::<_, _, ()>("CloseWindow", &(name.clone(),))
                .map_err(|err| format!("close failed: {}", err))?;
        }
        Command::Toggle { name } => {
            proxy
                .call::<_, _, ()>("ToggleWindow", &(name.clone(),))
                .map_err(|err| format!("toggle failed: {}", err))?;
        }
        Command::Show { name } => {
            proxy
                .call::<_, _, ()>("SetWindowVisible", &(name.clone(), true))
                .map_err(|err| format!("show failed: {}", err))?;
        }
        Command::Hide { name } => {
            proxy
                .call::<_, _, ()>("SetWindowVisible", &(name.clone(), false))
                .map_err(|err| format!("hide failed: {}", err))?;
        }
        Command::List => {
            let names: Vec<String> = proxy
                .call("ListWindows", &())
                .map_err(|err| format!("list failed: {}", err))?;
            for name in names {
                println!("{}", name);
            }
        }
        Command::Reload => {
            proxy
                .call::<_, _, ()>("Reload", &())
                .map_err(|err| format!("reload failed: {}", err))?;
        }
        Command::ReloadCss => {
            proxy
                .call::<_, _, ()>("ReloadCss", &())
                .map_err(|err| format!("reload-css failed: {}", err))?;
        }
        Command::Ping => {
            let reply: String = proxy
                .call("Ping", &())
                .map_err(|err| format!("ping failed: {}", err))?;
            println!("{}", reply);
        }
    }

    Ok(())
}
