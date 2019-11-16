pub mod polar_watch;

extern crate nfd;
extern crate polar_prost as polar;

#[allow(unused_imports)]
use log::{error, info};

use polar_prost::Message;
use polar_watch::{PolarError, PolarWatch};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn read_bytes<S>(path: S) -> std::io::Result<Vec<u8>>
where
    S: Into<String>,
{
    use std::fs::File;
    use std::io::Read;
    let path: String = path.into();

    let mut file = File::open(path)?;

    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    return Ok(data);
}

// Try to parse file as a PbTrainingSessionTarget and return human readable description
pub fn describe_favourite(data: Vec<u8>) -> Option<String> {
    let session = match polar::data::PbTrainingSessionTarget::decode(data) {
        Ok(session) => session,
        _ => return None,
    };

    let phases = match &session.exercise_target[0].phases {
        Some(phases) => phases,
        _ => return None,
    };

    match TrainingBlock::new(&phases.phase) {
        Some(session) => Some(session.describe()),
        _ => None,
    }
}

pub fn upload_favourites(paths: Vec<String>) -> Result<(), PolarError> {
    let mut files = Vec::new();
    for path in paths.clone() {
        match read_bytes(path.clone()) {
            Ok(data) => {
                files.push(data.clone());
            }
            error => {
                return Err(PolarError::new(format!(
                    "Failed to read file '{}'\n\t{:?}",
                    path, error
                )))
            }
        }
    }

    let mut context = rusb::Context::new()?;
    let mut watch = PolarWatch::find_one(&mut context)?;

    watch.delete_all_favorites()?;

    for slot in 0..files.len() {
        println!("Uploading {}:", paths[slot]);

        match describe_favourite(files[slot].clone()) {
            Some(description) => println!("{}\n", description),
            _ => println!("Failed to parse file, trying to upload anyway\n"),
        }

        let watch_path = format!("/U/0/FAV/{:02}", slot);

        // It's okay if this fails, directory might already exist
        let _ = watch.mkdir(watch_path.clone());

        watch.send_file(watch_path + "/TST.BPB", files[slot].as_slice())?;
    }

    Ok(())
}

fn split_at<I>(vector: Vec<I>, picker: &dyn Fn(&I) -> bool) -> (Vec<I>, Vec<I>) {
    let mut first = vec![];
    let mut second = vec![];

    let mut use_second = false;
    for item in vector {
        use_second = use_second || picker(&item);

        if use_second {
            second.push(item);
        } else {
            first.push(item);
        }
    }

    (first, second)
}

use std::time::Duration;
fn human_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    let mut result = "".to_string();

    if hours > 0 {
        result += &format!("{}h", hours);
    }

    if minutes > 0 {
        result += &format!("{}'", minutes);
    }

    if seconds > 0 || minutes + hours == 0 {
        result += &format!("{}''", seconds);
    }

    result
}

#[derive(Debug)]
enum TrainingBlock {
    Repeat {
        id: u32,
        times: u32,
        block: Vec<TrainingBlock>,
    },
    Phase {
        id: u32,
        name: String,
        duration: Duration,
    },
}

impl TrainingBlock {
    fn new(phases: &std::vec::Vec<polar::data::PbPhase>) -> Option<TrainingBlock> {
        let mut result = vec![];

        for (id, phase) in phases.iter().enumerate() {
            result.push(TrainingBlock::from(id as u32, phase.clone()));

            match phase.jump_index {
                Some(jump_id) => {
                    let (new_result, block) = split_at(result, &|phase| phase.id() == jump_id);

                    let times = match phase.repeat_count {
                        Some(value) => value + 1,
                        None => 0,
                    };

                    result = new_result;
                    match block.first() {
                        Some(phase) => result.push(TrainingBlock::Repeat {
                            id: phase.id(),
                            times,
                            block,
                        }),
                        None => return None,
                    };
                }
                None => (),
            }
        }

        Some(TrainingBlock::Repeat {
            id: 0,
            times: 1,
            block: result,
        })
    }

    fn from(id: u32, phase: polar::data::PbPhase) -> TrainingBlock {
        let id = id + 1;
        let duration = match phase.goal.duration {
            Some(dur) => {
                Duration::from_millis(dur.millis.unwrap_or(0).into())
                    + Duration::from_secs(dur.seconds.unwrap_or(0).into())
                    + Duration::from_secs((60 * dur.minutes.unwrap_or(0)).into())
                    + Duration::from_secs((3600 * dur.hours.unwrap_or(0)).into())
            }
            None => Duration::from_secs(0),
        };

        TrainingBlock::Phase {
            id,
            name: phase.name.text,
            duration,
        }
    }

    fn id(&self) -> u32 {
        match self {
            TrainingBlock::Phase {
                id,
                name: _,
                duration: _,
            } => id,
            TrainingBlock::Repeat {
                id,
                times: _,
                block: _,
            } => id,
        }
        .clone()
    }

    fn describe(&self) -> String {
        match self {
            TrainingBlock::Phase {
                id: _,
                name,
                duration,
            } => format!("{} {}", name, human_duration(duration.clone())),
            TrainingBlock::Repeat {
                id: _,
                times,
                block,
            } => {
                let duration = self.inner_duration();
                let mut result = format!("Repeat x{} [{}]", times, human_duration(duration));

                for item in block {
                    result += &("\n\t".to_string() + &item.describe().replace("\n", "\n\t"));
                }

                result.to_string()
            }
        }
    }

    fn duration(&self) -> Duration {
        match self {
            TrainingBlock::Phase {
                id: _,
                name: _,
                duration,
            } => duration.clone(),
            TrainingBlock::Repeat {
                id: _,
                times,
                block,
            } => {
                let mut result = Duration::from_secs(0);

                for phase in block {
                    result += phase.duration();
                }

                result * times.clone()
            }
        }
    }

    fn inner_duration(&self) -> Duration {
        match self {
            TrainingBlock::Phase {
                id: _,
                name: _,
                duration,
            } => duration.clone(),
            TrainingBlock::Repeat {
                id: _,
                times: _,
                block,
            } => {
                let mut result = Duration::from_secs(0);

                for phase in block {
                    result += phase.duration();
                }

                result
            }
        }
    }
}
