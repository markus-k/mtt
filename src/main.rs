use std::fs::{create_dir_all, File};
use std::path::Path;
use std::time::Duration;

use chrono::prelude::*;
use clap::{crate_version, Clap};
use directories::ProjectDirs;
use humantime::format_duration;
use serde::{Deserialize, Serialize};

#[derive(Clap)]
#[clap(author = "Markus Kasten <github@markuskasten.eu>")]
#[clap(version = crate_version!())]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(about = "Starts the timer")]
    Start(StartCommand),
    #[clap(about = "Stops the timer")]
    Stop,
    #[clap(about = "Abort the current timer")]
    Abort,
    #[clap(about = "Shows the current total time")]
    Show,
    #[clap(about = "Resets the total time")]
    Reset,
}

#[derive(Clap)]
struct StartCommand {}

#[derive(Debug, PartialEq)]
enum AppError {
    TimerAlreadyRunning,
    NoTimerRunning,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            AppError::TimerAlreadyRunning => "Timer already running",
            AppError::NoTimerRunning => "No timer running",
        };

        f.write_str(string)
    }
}

#[derive(Deserialize, Serialize)]
struct AppState {
    total: Duration,
    current_start: Option<DateTime<Utc>>,
}
impl Default for AppState {
    fn default() -> Self {
        AppState {
            total: Duration::from_secs(0),
            current_start: None,
        }
    }
}

impl AppState {
    fn start_timer(&mut self, start_time: DateTime<Utc>) -> Result<(), AppError> {
        match self.current_start {
            Some(_) => Err(AppError::TimerAlreadyRunning),
            None => {
                self.current_start = Some(start_time);

                Ok(())
            }
        }
    }

    fn stop_timer(&mut self, stop_time: DateTime<Utc>) -> Result<Duration, AppError> {
        let duration = self.current_duration(stop_time)?;

        self.total += duration;

        self.current_start = None;

        Ok(duration)
    }

    fn current_duration(&self, time: DateTime<Utc>) -> Result<Duration, AppError> {
        match self.current_start {
            Some(start) => {
                let duration = (time - start).to_std().unwrap_or_default();

                Ok(duration)
            }
            None => Err(AppError::NoTimerRunning),
        }
    }

    fn is_timer_running(&self) -> bool {
        self.current_start.is_some()
    }

    fn abort_timer(&mut self) {
        self.current_start = None;
    }
}

fn get_statefile_path() -> std::path::PathBuf {
    let dirs = ProjectDirs::from("eu", "markuskasten", "mtt").unwrap();
    let state_filename = "state.json";

    create_dir_all(&dirs.data_dir()).unwrap();

    let state_path = dirs.data_dir().join(state_filename);

    state_path
}

fn read_appstate(path: &Path) -> Result<AppState, serde_json::Error> {
    let file = File::open(path);

    if let Ok(file) = file {
        serde_json::from_reader(file)
    } else {
        Ok(AppState::default())
    }
}

fn write_appstate(state: &AppState, path: &Path) -> Result<(), serde_json::Error> {
    let file = File::create(path).unwrap();

    serde_json::to_writer(file, state)
}

fn get_duration_string(duration: &Duration) -> String {
    let duration_secs = Duration::from_secs(duration.as_secs());

    let formatted = format_duration(duration_secs);

    formatted.to_string()
}

fn main() {
    let opts = Opts::parse();

    let state_path = get_statefile_path();
    let mut state = read_appstate(&state_path).unwrap_or_default();

    match opts.subcmd {
        SubCommand::Start(_cmd) => {
            if let Err(err) = state.start_timer(Utc::now()) {
                println!("Cannot start timer: {}", err);
            } else {
                println!("Timer started.");
            }
        }
        SubCommand::Stop => {
            match state.stop_timer(Utc::now()) {
                Err(err) => println!("Cannot stop timer: {}", err),
                Ok(duration) => println!("Tracked: {}", get_duration_string(&duration)),
            };

            println!("Total time: {}", get_duration_string(&state.total));
        }
        SubCommand::Abort => {
            state.abort_timer();

            println!("Timer was stopped and duration discarded.");
        }
        SubCommand::Show => {
            if state.is_timer_running() {
                let duration = state.current_duration(Utc::now()).unwrap();
                println!("Current timer: {}", get_duration_string(&duration));
            }
            println!("Total: {}", get_duration_string(&state.total));
        }
        SubCommand::Reset => {
            state.total = Duration::from_secs(0);

            println!("Total duration was reset.");
        }
    };

    write_appstate(&state, &state_path).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appstate_default_no_timer_running() {
        let state = AppState::default();
        assert_eq!(state.current_start, None);
    }

    #[test]
    fn test_appstate_start_timer() {
        let mut state = AppState::default();
        let start_time = Utc::now();

        let res = state.start_timer(start_time);
        assert_eq!(res, Ok(()));

        assert_eq!(state.current_start.unwrap(), start_time);
    }

    #[test]
    fn test_appstate_stop_without_timer_running() {
        let mut state = AppState::default();
        let stop_time = Utc::now();

        let res = state.stop_timer(stop_time);
        let err = res.expect_err("stop_timer should not succeed");

        assert_eq!(err, AppError::NoTimerRunning);
    }

    #[test]
    fn test_appstate_abort() {
        let mut state = AppState::default();
        let start_time = Utc::now();

        state.start_timer(start_time).unwrap();

        state.abort_timer();

        assert!(!state.is_timer_running());
    }

    #[test]
    fn test_current_duration() {
        let mut state = AppState::default();
        let duration = Duration::from_secs(180);
        let start_time = Utc::now();
        let stop_time = start_time + chrono::Duration::from_std(duration).unwrap();

        state.start_timer(start_time).unwrap();

        assert_eq!(state.current_duration(stop_time).unwrap(), duration);
    }
}
