use std::collections::HashMap;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::iter::Sum;
use std::path::Path;
use std::time::Duration;

use chrono::prelude::*;
use clap::{crate_version, Clap};
use directories::ProjectDirs;
use humantime::format_duration;
use serde::{Deserialize, Serialize};

/*
 * Usage:
 *
 * mtt new NAME
 * mtt start [NAME]
 * mtt stop [STOP-TIME] [-m MESSAGE]
 * mtt list
 * mtt show
 *
 */

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
    Stop(StopCommand),
    #[clap(about = "Abort the current timer")]
    Abort,
    #[clap(about = "Shows the current total time")]
    Show,
    #[clap(about = "Resets the total time")]
    Reset,
}

#[derive(Clap)]
struct StartCommand {
    #[clap(about = "Timer to start")]
    timer_name: Option<String>,

    #[clap(long, short, about = "Create timer with this name")]
    create: bool,
}

#[derive(Clap)]
struct StopCommand {
    #[clap(about = "Timer to stop")]
    timer_name: Option<String>,

    #[clap(
        long,
        about = "Stop time to use instead of now (if you forgot to stop your timer again)"
    )]
    stop_time: String,

    #[clap(long, about = "A comment to add to this timer record")]
    comment: String,
}

#[derive(Debug, PartialEq)]
enum AppError {
    TimerAlreadyRunning,
    NoTimerRunning,
    NoSuchTimer,
}

impl Error for AppError {}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            AppError::TimerAlreadyRunning => "Timer already running",
            AppError::NoTimerRunning => "No timer running",
            AppError::NoSuchTimer => "No timer with this name",
        };

        f.write_str(string)
    }
}

#[derive(Deserialize, Serialize)]
struct TimerRecord {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    comment: String,
}
impl TimerRecord {
    fn new(start: DateTime<Utc>, end: DateTime<Utc>, comment: String) -> Self {
        Self {
            start,
            end,
            comment,
        }
    }

    fn duration(&self) -> Duration {
        // in case start > end date, return 0s duration
        (self.end - self.start).to_std().unwrap_or_default()
    }
}

#[derive(Deserialize, Serialize)]
struct Timer {
    records: Vec<TimerRecord>,
    current_start: Option<DateTime<Utc>>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            records: vec![],
            current_start: None,
        }
    }
}

impl Timer {
    fn start_timer(&mut self, start_time: DateTime<Utc>) -> Result<(), AppError> {
        if self.current_start.is_some() {
            return Err(AppError::TimerAlreadyRunning);
        }

        self.current_start = Some(start_time);

        Ok(())
    }

    fn stop_timer(
        &mut self,
        stop_time: DateTime<Utc>,
        comment: String,
    ) -> Result<&TimerRecord, AppError> {
        if let Some(current_start) = self.current_start {
            self.records
                .push(TimerRecord::new(current_start, stop_time, comment));
            let record = self.records.last().unwrap();

            self.current_start = None;

            Ok(record)
        } else {
            Err(AppError::NoTimerRunning)
        }
    }

    fn total_duration(&self) -> Duration {
        let durations = self.records.iter().map(|record| record.duration());
        Duration::sum(durations)
    }

    fn is_running(&self) -> bool {
        self.current_start.is_some()
    }
}

#[derive(Deserialize, Serialize)]
struct AppState {
    timers: HashMap<String, Timer>,
    active_timer: Option<String>,
}
impl Default for AppState {
    fn default() -> Self {
        AppState {
            timers: HashMap::default(),
            active_timer: None,
        }
    }
}

impl AppState {
    fn get_active_timer(&self) -> Option<&Timer> {
        match &self.active_timer {
            Some(timer_name) => self.timers.get(timer_name),
            None => None,
        }
    }

    fn has_active_timer(&self) -> bool {
        if let Some(timer_name) = &self.active_timer {
            self.timers.contains_key(timer_name)
        } else {
            false
        }
    }

    fn set_timer_active(&mut self, timer_name: &str) -> Result<(), AppError> {
        if self.timers.contains_key(timer_name) {
            self.active_timer = Some(String::from(timer_name));

            Ok(())
        } else {
            Err(AppError::NoSuchTimer)
        }
    }

    fn create_timer(&mut self, name: &str) -> Option<&Timer> {
        if self.timers.contains_key(name) {
            None
        } else {
            let timer = Timer::default();
            self.timers.insert(name.to_string(), timer);

            self.get_timer(name)
        }
    }

    fn get_timer(&self, name: &str) -> Option<&Timer> {
        self.timers.get(name)
    }

    fn read_from_file(path: &Path) -> Result<Self, serde_json::Error> {
        let file = File::open(path);

        if let Ok(file) = file {
            serde_json::from_reader(file)
        } else {
            Ok(AppState::default())
        }
    }

    fn write_to_file(&self, path: &Path) -> Result<(), serde_json::Error> {
        let file = File::create(path).unwrap();

        serde_json::to_writer(file, self)
    }
}

fn get_statefile_path() -> std::path::PathBuf {
    let dirs = ProjectDirs::from("eu", "markuskasten", "mtt").unwrap();
    let state_filename = "state.json";

    create_dir_all(&dirs.data_dir()).unwrap();

    let state_path = dirs.data_dir().join(state_filename);

    state_path
}

fn get_duration_string(duration: &Duration) -> String {
    let duration_secs = Duration::from_secs(duration.as_secs());

    let formatted = format_duration(duration_secs);

    formatted.to_string()
}

fn main() {
    let opts = Opts::parse();

    let state_path = get_statefile_path();
    let mut state = AppState::read_from_file(&state_path).unwrap_or_default();

    match opts.subcmd {
        SubCommand::Start(_cmd) => {}
        SubCommand::Stop(_cmd) => {}
        SubCommand::Abort => {}
        SubCommand::Show => {}
        SubCommand::Reset => {}
    };

    state.write_to_file(&state_path).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timerrecord_duration() {
        let duration = Duration::from_secs(1234);
        let start = Utc::now();
        let end = start + chrono::Duration::from_std(duration).unwrap();

        let record = TimerRecord::new(start, end, "".to_owned());

        assert_eq!(record.duration(), duration);

        // zero duration
        let record = TimerRecord::new(start, start, "".to_owned());

        assert_eq!(record.duration(), Duration::ZERO);
    }

    #[test]
    fn test_timer_total_duration() {
        let duration = Duration::from_secs(1234);
        let start = Utc::now();
        let end = start + chrono::Duration::from_std(duration).unwrap();
        let record = TimerRecord::new(start, end, "".to_owned());

        let duration2 = Duration::from_secs(321);
        let start2 = Utc::now();
        let end2 = start2 + chrono::Duration::from_std(duration2).unwrap();
        let record2 = TimerRecord::new(start2, end2, "Playing solitaire".to_owned());

        let total_duration = duration + duration2;

        let timer = Timer {
            records: vec![record, record2],
            current_start: None,
        };

        assert_eq!(timer.total_duration(), total_duration);
    }

    #[test]
    fn test_appstate_set_active_timer_nonexisting() {
        let mut state = AppState::default();

        assert_eq!(
            state.set_timer_active("something").unwrap_err(),
            AppError::NoSuchTimer
        );
    }

    #[test]
    fn test_appstate_create_timer() {
        let mut state = AppState::default();
        let timer_name = "timer name";

        state.create_timer(timer_name).unwrap();

        // can't create a second timer with the same name
        assert!(state.create_timer(timer_name).is_none());
    }

    #[test]
    fn test_appstate_get_timer() {
        let mut state = AppState::default();
        let timer_name = "timer name";

        state.create_timer(timer_name).unwrap();

        let timer1 = state.get_timer(timer_name).unwrap();

        let timer2 = state.get_timer(timer_name).unwrap();

        assert!(std::ptr::eq(timer1, timer2));
    }
}
