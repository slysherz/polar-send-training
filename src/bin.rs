extern crate nfd;
extern crate polar_send_training;

use log::{error, info};
use polar_send_training::polar_watch::PolarError;
use polar_send_training::{upload_favourites, VERSION};
use simplelog::*;

// Pauses the program and waits for the user to press enter
fn pause(message: &str) {
    use std::io;
    use std::io::prelude::*;

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "{}", message).unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

// Initializes logging to a file if possible, or to the console if not
pub fn init_logger() {
    let log_filename = "polar-send-training.log";
    let log_level = LevelFilter::Info;
    let log_config = Config::default();

    let logger_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .create_new(false)
        .append(true)
        .open(log_filename);

    let logger: Box<dyn SharedLogger> = match logger_file {
        Ok(file) => WriteLogger::new(log_level, log_config, file),
        Err(_) => match TermLogger::new(log_level, log_config.clone(), TerminalMode::Mixed) {
            Some(logger) => logger,
            None => SimpleLogger::new(log_level, log_config),
        },
    };

    // Ignore errors so that the application works even when logging doesn't
    let _ = CombinedLogger::init(vec![logger]);
}

fn fake_main() {
    init_logger();
    info!("Initalizing polar-send-training version {}", VERSION);

    // Skip program name
    let mut files: Vec<String> = std::env::args().skip(1).collect();

    if files.is_empty() {
        info!("No files provided, asking user");
        let result = nfd::open_file_multiple_dialog(Some("BPB"), None).unwrap_or_else(|error| {
            error!("Failed to open file dialog: {:?}", error);
            pause("Please close the program and try again.");
            panic!(error);
        });

        match result {
            nfd::Response::Okay(file_path) => files = [file_path].to_vec(),
            nfd::Response::OkayMultiple(files_paths) => files = files_paths,
            nfd::Response::Cancel => {
                info!("Dialog canceled");
                std::process::exit(0);
            }
        }
    }

    info!("Uploading files {:?}", files);
    match upload_favourites(files) {
        Err(PolarError::LibusbError { error }) => println!("Something went wrong\n\t{:?}\n", error),
        Err(PolarError::PolarError { error }) => println!("Something went wrong\n\t{:?}\n", error),
        _ => println!("\nAll files were transfered successfully. Life is good :)\n"),
    }
}

// We try to catch all possible errors in main, so that we can warn the user about them
fn main() {
    let result = std::panic::catch_unwind(|| {
        fake_main();
    });

    if result.is_err() {
        println!("Something went horribly wrong and the program blew up :(");
    }

    pause("You can now close the program.");
}
