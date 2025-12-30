use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use glob::glob;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as nuConfig, Matcher};
use serde::Deserialize;
use std::env::var;
use std::fs;
use std::path::{Path, PathBuf};
// use sequoia_openpgp as openpgp;
// use openpgp::parse::Parse;
// use openpgp::policy::StandardPolicy;
// use openpgp::serialize::stream::Decryptor;
// use std::io::{self, Read};

#[derive(Deserialize, Debug)]
struct Config {
    minimum_length: usize,
    max_results: usize,
    // store_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            minimum_length: 3,
            // store_path:
            max_results: 10,
        }
    }
}

struct State {
    store_path: PathBuf,
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

    // TODO: Also config this
    let mut store_path = PathBuf::new();
    store_path.push(&var("HOME").unwrap());
    store_path.push(".password-store");

    State { store_path, config }
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

    let mut file_glob_pattern = state.store_path.to_path_buf();
    file_glob_pattern.push("**");
    file_glob_pattern.push("*.gpg");
    let pass_files = glob(&file_glob_pattern.to_string_lossy()).unwrap();

    let base_store_path = state.store_path.to_path_buf();

    let mut matches: RVec<Match> = RVec::new();

    let mut matcher = Matcher::new(nuConfig::DEFAULT.match_paths());
    let pattern = Pattern::parse(&input, CaseMatching::Ignore, Normalization::Smart);

    let mut all_files: Vec<String> = Vec::new();
    for entry in pass_files {
        match entry {
            Ok(path) => {
                // Ignore any git files
                if path.starts_with(Path::join(&base_store_path, ".git")) {
                    continue;
                }
                all_files.push(path.to_string_lossy().into_owned());
            }
            Err(e) => println!("{:?}", e),
        }
    }
    let fuzzy_matches: Vec<(String, u32)> = pattern.match_list(all_files, &mut matcher);
    for fmatch in fuzzy_matches.iter().take(state.config.max_results) {
        let match_path = Path::new(&fmatch.0);
        let relative_path = match_path.strip_prefix(&base_store_path).unwrap();

        let mut title: RString;
        let description: ROption<RString>;
        if match_path.is_dir() {
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
            id: ROption::RNone,
            icon: ROption::RNone,
        });
    }
    matches.into()
}

// fn decrypt_file_with_agent(filepath: Path) -> String {
// }

#[handler]
fn handler(_selection: Match) -> HandleResult {
    HandleResult::Close
}
