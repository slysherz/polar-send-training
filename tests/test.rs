extern crate nfd;
extern crate polar_send_training;

use polar_send_training::polar_watch::{polar, PolarWatch};

fn test_all_files_from(test_dir: &str) {
    let mut context = rusb::Context::new().unwrap();
    let mut watch = PolarWatch::find_one(&mut context).unwrap();

    for entry in std::fs::read_dir(test_dir).unwrap() {
        let empty_extension = std::ffi::OsStr::new("");
        let file: std::fs::DirEntry = entry.unwrap();
        let path = file.path();
        let extension = path
            .extension()
            .unwrap_or(&empty_extension)
            .to_string_lossy()
            .to_uppercase();

        if extension == "BPB" {
            println!("Testing file {}", file.file_name().to_string_lossy());

            let data = polar_send_training::read_bytes(path.to_string_lossy()).unwrap();

            // Sending the data to the watch is enough, it automatically validates the tranfer
            watch.send_file("/U/0/FAV/00/TST.BPB", &data).unwrap();
        }
    }
}

#[test]
fn test_examples() {
    test_all_files_from("tests/examples");
}

#[test]
// Try to send files with "all" possible sizes. Errors from separating packets in the wrong place,
// or losing one byte in the middle, are common
fn fuzz_by_size() {
    let mut context = rusb::Context::new().unwrap();
    let mut watch = PolarWatch::find_one(&mut context).unwrap();

    for i in 0..1000 {
        println!("Sending with size {}", i);

        // Generate a mostly empty train session, but add text to the description to increase the
        // file size
        let session = polar::data::PbTrainingSessionTarget {
            name: polar::types::PbOneLineText {
                text: "name".to_string(),
            },
            description: Some(polar::types::PbMultiLineText {
                text: "A".repeat(i),
            }),
            duration: None,
            event_id: None,
            exercise_target: vec![polar::data::PbExerciseTarget {
                phases: None,
                route: None,
                sport_id: None,
                steady_race_pace: None,
                strava_segment_target: None,
                target_type: 0,
                volume_target: None,
            }],
            sport_id: None,
            start_time: None,
            target_done: None,
            training_program_id: None,
        };

        let data = polar::encode(session).unwrap();
        watch.send_file("/U/0/FAV/00/TST.BPB", &data).unwrap();
    }
}
