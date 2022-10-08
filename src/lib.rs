use std::path::{PathBuf};
use serde::{Serialize, Deserialize};
use log::{error, info};
use regex::Regex;
use std::fs::{self, DirEntry};
use std::fs::File;
use thiserror::Error;
use std::env;

#[derive(Error, Debug)]
pub enum TempDirErrors {
    #[error("Faled to create Temporary Directory")]
    CreationFailed,
    #[error("Invalid duration string specified")]
    WrongDurationString,
    #[error("Invalid time period specified")]
    WrongPeriodString,
    #[error("Invalid time amount specified")]
    WrongTimeAmount,
    #[error("Meta data storage directory couldn't be created/found")]
    StoreFolderError
}
enum PeriodStringValue {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
}
impl PeriodStringValue {
    fn value(self) -> i64 {
        match self {
            Self::Second => 1,
            Self::Minute => 60,
            Self::Hour => 3600,
            Self::Day => 86400,
            Self::Week => 604800,
            Self::Month => 2678400,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TemporaryDirectory {
    name: String,
    duration: String,
    created_at: i64,
    end_time: i64,
    path: Option<PathBuf>,
}
impl TemporaryDirectory {
    pub fn new(name: String, duration: String) -> Result<TemporaryDirectory, TempDirErrors> {
        match parse_duration_string(&duration) {
            Ok(value) => {
                info!("Total lifetime: {value}");
                let time = chrono::offset::Local::now();
                let startime: i64 = time.timestamp();
                let endtime: i64 = startime + value;
                Ok(TemporaryDirectory {
                    name: name,
                    duration: duration,
                    created_at: startime,
                    end_time: endtime,
                    path: None,
                })
            }
            Err(_) => {
                error!("Failed to create Temporary Directory");
                Err(TempDirErrors::CreationFailed)
            }
        }
    }

    pub fn create(mut self) {
        let directory = fs::create_dir(&self.name);
        match directory {
            Ok(_) => {
                match PathBuf::from(&self.name).canonicalize() {
                    Ok(path) => self.path = Some(path),
                    Err(_) => error!("Something went wrong creating the path"),
                }
                info!("Directory created successfully");
                self.save();
            }
            Err(_) => error!("Failed to create directory"),
        }
    }

    pub fn save(self) {
        let mut path = match info_store_path() {
            Ok(path) => path,
            Err(_) => {
                error!("Meta data directory couldn't be found. Temporary directory cannot be created");
                self.delete();
                return
            }
        };

        match fs::read_dir(&path).ok() {
            Some(_) => {},
            None => {
                info!("Meta data directory not found. Creating it now");
                // this line is bad
                match fs::create_dir(&path) {
                    Ok(()) => info!("Meta data directory created"),
                    Err(_) => {
                        error!("Meta data directory couldn't be created. Temporary directory couldn't be created");
                        self.delete();
                        return
                    }
                }
            }
        };

        // Create meta data file
        path.push(format!("{}.json", self.name));

        let file = match File::create(&path) {
            Ok(file) => file,
            Err(_) => {
                error!("Meta data file couldn't be created. Temporary directory couldn't be created");
                self.delete();
                return
            }
        };

        match serde_json::to_writer(&file, &self) {
            Ok(_) => info!("Temporary directory saved"),
            Err(_) => {
                error!("Failed to save meta data file. Temporary directory couldn't be created");
                self.delete();
                return
            }
        };
    }

    pub fn delete(self) {
        match self.path {
            Some(path) => {
                match fs::remove_dir(&path) {
                    Ok(_) => info!("Removed directory"),
                    Err(_) => error!("Unable to remove directory"),
                }
            }
            None => {
                error!("Directory can't be removed, path is not specified")
            }
        }
    }
}

// Proper error handling
pub fn clean_directories() {
    let path = match info_store_path() {
        Ok(path) => path,
        Err(_) => {
            error!("Meta data directory couldn't be found. Temporary directories cannot be deleted");
            return
        }
    };

    let temporary_directory_files = match fs::read_dir(&path).ok() {
        Some(dir) => dir,
        None => {
            info!("Meta data directory couln't be opened. Temporary directories cannot be deleted");
            return
        }
    };
    let mut deleted_directory_files: Vec<DirEntry> = Vec::new();
    for temporary_directory_file in temporary_directory_files {
        match temporary_directory_file {
            Ok(file_name) => {
                let path = file_name.path();
                // This could fail
                let file = match File::open(path) {
                    Ok(file) => file,
                    Err(_) => {
                        error!("Meta data file couldn't be read. Continuing");
                        continue;
                    }
                };
                
                let temporary_directory: TemporaryDirectory = match serde_json::from_reader(file) {
                    Ok(data) => data,
                    Err(_) => {
                        error!("Temporary directory couldn't be parsed. Continuing");
                        continue;
                    }
                };

                if check_temporary_directory(&temporary_directory) {
                    temporary_directory.delete();
                }
                deleted_directory_files.push(file_name);
            },
            Err(_) => {
                info!("No meta data files stored")
            }
        }
    }

    for deleted_file in deleted_directory_files {
        let path = deleted_file.path();
        match fs::remove_file(&path) {
            Ok(()) => info!("{path:?} meta data file deleted"),
            Err(_) => error!("{path:?} meta data file couldn't be deleted"),
        }
    }
}

fn check_temporary_directory(tempdir: &TemporaryDirectory) -> bool {
    let current_time: i64 = chrono::offset::Local::now().timestamp();
    let directory_end_time = tempdir.end_time;

    current_time > directory_end_time
}

fn info_store_path() -> Result<PathBuf, TempDirErrors> {
    let path_to_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return Err(TempDirErrors::StoreFolderError)
    };

    match path_to_exe.parent() {
        Some(path) => {
            let mut folder = PathBuf::from(path);
            folder.push("temporary_directories");
            return Ok(folder)
        }
        None => return Err(TempDirErrors::StoreFolderError)
    }
}

pub fn parse_duration_string(duration: &str) -> Result<i64, TempDirErrors> {
    let duration_amount = match parse_amount(duration) {
        Ok(amount) => {
            Ok(amount)
        }
        Err(TempDirErrors::WrongDurationString) => {
            error!("Unable to parse duration string: Invalid duration string specified");
            Err(TempDirErrors::WrongDurationString)
        }
        Err(TempDirErrors::WrongTimeAmount) => {
            error!("Unable to parse duration string: Invalid amount specified");
            Err(TempDirErrors::WrongDurationString)
        }
        _ => {
            error!("Something unknown went wrong parsing the duration amount");
            Err(TempDirErrors::WrongDurationString)
        }
    };
    // return duration_amount;
    let period_amount = match parse_period(duration) {
        Ok(value) => {
            Ok(value)
        }
        Err(TempDirErrors::WrongPeriodString) => {
            error!("Unable to parse duration string: Invalid period specified");
            Err(TempDirErrors::WrongDurationString)
        }
        Err(_) => {
            error!("Something unknown went wrong parsing the duration period");
            Err(TempDirErrors::WrongDurationString)
        }
    };

    if !duration_amount.is_err() && !period_amount.is_err() {
        return Ok(duration_amount.unwrap() * period_amount.unwrap());
    } else {
        return Err(TempDirErrors::WrongDurationString);
    }
}
fn parse_amount(duration: &str) -> Result<i64, TempDirErrors> {
    let regex_amount = Regex::new(r"[A-Za-z]+").unwrap();
    let amount_vec: Vec<&str> = regex_amount.split(duration).filter(|x| *x != "").collect();
    if amount_vec.len() != 1 {
        return Err(TempDirErrors::WrongDurationString);
    }
    let amount = amount_vec[0].parse::<i64>();
    match amount {
        Ok(value) => Ok(value),
        Err(_) => Err(TempDirErrors::WrongTimeAmount),
    }
}
fn parse_period(duration: &str) -> Result<i64, TempDirErrors> {
    // Need to account for possible unwrap error
    let period_string: String = duration
        .chars()
        .filter(|x| x.is_alphabetic())
        .map(|x| x.to_lowercase().next().unwrap())
        .collect();

    let period_amount = match period_string.as_str() {
        "s" => Ok(PeriodStringValue::Second.value()),
        "min" => Ok(PeriodStringValue::Minute.value()),
        "h" => Ok(PeriodStringValue::Hour.value()),
        "d" => Ok(PeriodStringValue::Day.value()),
        "w" => Ok(PeriodStringValue::Week.value()),
        "m" => Ok(PeriodStringValue::Month.value()),
        _ => Err(TempDirErrors::WrongPeriodString),
    };
    period_amount
}