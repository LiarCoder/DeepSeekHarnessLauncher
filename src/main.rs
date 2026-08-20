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
    let single_instance = match single_instance::SingleInstance::acquire() {
        Ok(Some(single_instance)) => single_instance,
        Ok(None) => return,
        Err(error) => {
            app::show_fatal_error(&error);
            return;
        }
    };

    match app::App::new(single_instance).and_then(app::App::run) {
        Ok(()) => {}
        Err(error) => app::show_fatal_error(&error),
    }
}
