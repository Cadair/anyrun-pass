use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use glob::glob;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as nuConfig, Matcher};
use serde::Deserialize;
use std::env::var;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Debug)]
struct Config {
    minimum_length: usize,
    max_results: usize,
    store_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            minimum_length: 3,
            store_path: {
                if let Ok(env_path) = &var("PASSWORD_STORE_DIR") {
                    PathBuf::from(env_path)
                } else {
                    let mut store_path = PathBuf::new();
                    store_path.push(&var("HOME").unwrap());
                    store_path.push(".password-store");
                    store_path
                }
            },
            max_results: 10,
        }
    }
}

struct State {
    pass_files: Vec<String>,
    config: Config,
}

#[init]
fn init(config_dir: RString) -> State {
    eprintln!("[pass] Initialized");
    let config = match fs::read_to_string(format!("{config_dir}/pass.ron")) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("[pass] Failed to parse config: {why}");
            Config::default()
        }),
        Err(why) => {
            eprintln!("[pass] No config file provided, using default: {why}");
            Config::default()
        }
    };

    eprintln!("[pass] Loading entries from store: {}", config.store_path.to_string_lossy());

    let mut file_glob_pattern = config.store_path.to_path_buf();
    file_glob_pattern.push("**");
    file_glob_pattern.push("*.gpg");
    let paths = glob(&file_glob_pattern.to_string_lossy()).unwrap();

    // Now we filter the results A bit and unwrap them
    let mut pass_files: Vec<String> = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) => {
                // Ignore any git files
                if path.starts_with(Path::join(&config.store_path, ".git")) {
                    continue;
                }
                let relative_path = path.strip_prefix(&config.store_path).unwrap();
                pass_files.push(String::from(relative_path.to_string_lossy()));
            }
            Err(e) => eprintln!("[pass] Skipping file {:?} - {}", e.path(), e.error()),
        }
    }
    eprintln!("[pass] Found {} entries in store.", pass_files.len());
    State { pass_files, config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Pass".into(),
        icon: "padlock".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &mut State) -> RVec<Match> {
    if input.len() < state.config.minimum_length {
        return RVec::new();
    }

    let mut matches: RVec<Match> = RVec::new();

    let mut matcher = Matcher::new(nuConfig::DEFAULT.match_paths());
    let pattern = Pattern::parse(&input, CaseMatching::Ignore, Normalization::Smart);

    let fuzzy_matches: Vec<(&String, u32)> = pattern.match_list(&state.pass_files, &mut matcher);
    for fmatch in fuzzy_matches.iter().take(state.config.max_results) {
        let relative_path = Path::new(&fmatch.0);
        let id = state.pass_files.iter().position(|r| r == fmatch.0);
        let Some(id) = id else { continue };

        let mut title: RString;
        let description: ROption<RString>;
        if relative_path.is_dir() {
            title = RString::from(relative_path.to_string_lossy());
            title.push('/');
            description = ROption::RNone;
        } else {
            title = RString::from(
                relative_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            description = ROption::RSome(RString::from(
                relative_path.parent().unwrap().to_string_lossy(),
            ));
        }
        // let filename = RString::from(without_ext.to_string_lossy().into_owned());
        matches.push(Match {
            title: title,
            description: description,
            use_pango: false,
            id: ROption::RSome(id.try_into().unwrap()),
            icon: ROption::RNone,
        });
    }
    matches.into()
}

fn shell_out_to_pass(store_path: &str, filename: &str) -> Result<String, Error> {
    let output = Command::new("pass")
        .env("PASSWORD_STORE_DIR", store_path)
        .arg(filename)
        .output()?;
    let output_str = String::from_utf8(output.stdout).unwrap();
    match output_str.lines().next() {
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No output",
            ))
        }
        Some(first_line) => return Ok(first_line.to_string()),
    }
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let ROption::RSome(id) = selection.id else {
        eprintln!("[pass] Internal Error - Unable to open {}", selection.title);
        return HandleResult::Close;
    };
    let id: usize = id as usize;
    let relative_path = &state.pass_files[id];
    let secret_name = relative_path
        .strip_suffix(".gpg")
        .unwrap_or_else(|| relative_path);

    println!("[pass] Reading password from store: {}", secret_name);
    match shell_out_to_pass(&state.config.store_path.to_string_lossy(), secret_name) {
        Err(e) => {
            eprintln!("[pass] Failed to read password from store: {}", e);
            return HandleResult::Close;
        }
        Ok(password) => {
            eprintln!("{}", password);
            return HandleResult::Copy(RVec::from(password.into_bytes()));
        }
    }
}
