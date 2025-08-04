mod application;
mod assets;
mod integrated_app;
mod render;
mod util;

fn main() {
    // Use new integrated application with teapot rendering
    let mut app = integrated_app::IntegratedApp::new();
    if let Err(e) = app.run() {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
