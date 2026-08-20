#![windows_subsystem = "windows"]

mod app;
mod browser;
mod command;
mod dsh;
mod logger;
mod paths;
mod process;
mod registry;
mod single_instance;
mod state;

fn main() {
    let Ok(Some(single_instance)) = single_instance::SingleInstance::acquire() else {
        return;
    };

    match app::App::new(single_instance).and_then(app::App::run) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("DeepSeek Harness Launcher 启动失败：{error}");
        }
    }
}
