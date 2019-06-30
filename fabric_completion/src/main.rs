// cargo install --force --path fabric_completion/ && complete -C _fabric_completion fab

use std::env::current_dir;
use std::fs::File;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use cachedir::CacheDirConfig;
use shell_completion::{BashCompletionInput, CompletionInput, CompletionSet};


const CACHE_NAMESPACE : &str = "fab_completion";
const CACHE_FILENAME : &str = "tasks";
const CACHE_CLEAR_TOKEN : &str = "_cache_clear";


fn main() {
    let input = BashCompletionInput::from_env()
        .expect("Missing expected environment variables");

    complete(input).suggest();
}

fn complete(input: impl CompletionInput) -> Vec<String> {
    match input.arg_index() {
        0 => unreachable!(),
        1 => complete_fab_commands(input),
        2 => complete_second(input),
        _ => vec![],
    }
}

fn get_cache_dir(namespace : &str) -> PathBuf{
    let current_dir = current_dir().unwrap().to_path_buf();
    let cache_key_path = format!("{}{}", namespace, current_dir.display());
    CacheDirConfig
        ::new(&cache_key_path)
        .user_cache(true)
        .get_cache_dir()
        .unwrap()
        .into_path_buf()
}

fn cache_get(cache_filename : &str) -> Vec<String> {
    let cache_dir : PathBuf = get_cache_dir(CACHE_NAMESPACE);
    let full_path = cache_dir.join(cache_filename);
    if !Path::new(&full_path).exists() {
        vec![]
    }else{
        let file = File::open(full_path).expect("Unable to open cache file");
        let buf = BufReader::new(file);
        buf.lines()
            .map(|l| l.expect("Unable to parse line from cache file"))
            .collect()
    }
}

fn cache_set(cache_filename : &str, vector : Vec<String>) {
    let cache_dir : PathBuf = get_cache_dir(CACHE_NAMESPACE);
    let full_path = cache_dir.join(cache_filename);
    let mut f = File::create(full_path).expect("Unable to create cache file");
    for line in vector {
        writeln!(f, "{}", line).expect("Unable to write to cache file");
    }
}


fn cache_clear(cache_filename : &str) {
    let cache_dir : PathBuf = get_cache_dir(CACHE_NAMESPACE);
    let full_path = cache_dir.join(cache_filename);
    let path_to_file = Path::new(&full_path);
    if path_to_file.exists() {
        fs::remove_file(path_to_file).expect("Unable to delete cache file");
    }
}

fn get_cached(cache_filename : &str, f: &Fn() -> Vec<String>) -> Vec<String> {
    let vector = cache_get(cache_filename);
    if vector.is_empty() {
        let vector = f();
        cache_set(cache_filename, vector);
    }
    vector
}

fn get_fab_tasks() -> Vec<String> {
    let output =
        Command::new("fab")
        .arg("--shortlist")
        .output()
        .expect("Unable to get list of tasks from fabric");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let fab_tasks =
        stdout
        .lines()
        .map(|line| line.trim())
        .map(str::to_owned)
        .collect();

    fab_tasks
}


fn complete_second(input: impl CompletionInput) -> Vec<String> {
    // For fab tasks this is never used for anything,
    // just to trigger clearing the completion cache.
    let previous_word = input.previous_word();
    if previous_word == CACHE_CLEAR_TOKEN {
        cache_clear(CACHE_FILENAME);
        println!("_CACHE_CLEARED");
        // Now populate the cache again but don't return anything.
        get_cached(CACHE_FILENAME, &get_fab_tasks);
    }
    vec![]
}

fn complete_fab_commands(input: impl CompletionInput) -> Vec<String> {
    let mut fab_tasks = get_cached(CACHE_FILENAME, &get_fab_tasks);

    let current_word = input.current_word();

    if fab_tasks.contains(&current_word.to_owned()) {
        // If current_word exactly matches a task (without a space at the end)
        // then do completion with the arguments info appended to it.
        complete_fab_command_args(input)
    }else{
        fab_tasks.push(CACHE_CLEAR_TOKEN.to_owned());
        input.complete_subcommand(fab_tasks)
    }
}


fn complete_fab_command_args(input: impl CompletionInput) -> Vec<String> {
    // $ fab -d <task>
    // Displaying detailed information for task '<task>':
    //
    //     No docstring provided
    //     Arguments: mode=None

    let current_word = input.current_word().to_owned();

    let output =
        Command::new("fab")
        .arg("-d")
        .arg(input.current_word())
        .output()
        .expect("Failed to execute fabric");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut task_args : Vec<String> =
        stdout
        .lines()
        .skip(3)  // TODO: until "Arguments:"
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.replace("Arguments: ", &format!("{}:", &current_word))
        })
        .collect();

    task_args.push(current_word);

    input.complete_subcommand(task_args)
}
