//! Minimalist interactive console menu (MAS / Massgrave style).
//! Plain standard I/O; zero TUI library dependencies. All interface in English.
use crate::{
    background,
    config::{self, Config, Mode},
    diagnostics, live, settings,
};
use anyhow::Result;
use std::io::{self, Write};
use std::path::Path;

pub fn setup() -> Result<()> {
    first_run_setup()
}

pub fn diagnostics_menu() -> Result<()> {
    println!("{}", diagnostics::report());
    Ok(())
}

pub fn logs_menu() -> Result<()> {
    println!("{}", diagnostics::log()?);
    Ok(())
}

pub fn first_run_setup() -> Result<()> {
    let path = config::path()?;
    let config = if path.exists() {
        Config::load(&path).unwrap_or_default()
    } else {
        Config::default()
    };

    println!();
    println!("=====================================================");
    println!("                WELCOME TO AI DIKTE                  ");
    println!("=====================================================");
    println!(" AI Dikte is a lightweight, local dictation tool");
    println!(" powered by Gemini Live.");
    println!();
    println!(" To get started, please enter your Gemini API key.");
    println!(" (Get a free key at: https://aistudio.google.com/apikey)");
    println!("-----------------------------------------------------");

    loop {
        print!(" Gemini API Key: ");
        io::stdout().flush()?;
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        if key.is_empty() {
            println!(" [!] API key cannot be empty.");
            continue;
        }

        print!(" -> Verifying API key... ");
        io::stdout().flush()?;

        let test_result = {
            let cfg = config.clone();
            let k = key.to_string();
            tokio::runtime::Runtime::new()?.block_on(live::validate_key(&cfg, &k))
        };

        match test_result {
            Ok(()) => {
                println!("SUCCESS!");
                settings::save_verified(config.clone(), key.to_string(), &path, |_, _| Ok(()))?;
                println!(" [OK] Settings saved successfully.");
                break;
            }
            Err(err) => {
                println!("FAILED!");
                println!(" [!] Connection failed: {err:#}");
                println!("     Please check your API key and try again.");
            }
        }
    }

    println!();
    print!(" -> Starting background listener service... ");
    io::stdout().flush()?;
    if let Err(e) = background::start() {
        println!("FAILED!");
        eprintln!(" [!] Could not start background service: {e:#}");
    } else {
        println!("STARTED!");
        let _ = background::set_startup(true);
        println!(
            " [OK] Background service is active and set to start at sign-in.\n      Press {} to dictate.",
            if cfg!(windows) { "Win+Z" } else { "Meta+Z" }
        );
    }

    println!("\n Press Enter to continue to the main menu...");
    let mut pause = String::new();
    io::stdin().read_line(&mut pause)?;

    menu()
}

pub fn menu() -> Result<()> {
    let path = config::path()?;
    if !path.exists() {
        return first_run_setup();
    }
    let mut config = match Config::load(&path) {
        Ok(c) => c,
        Err(_) => return first_run_setup(),
    };
    if config.key().is_err() {
        return first_run_setup();
    }

    loop {
        let running = background::running().unwrap_or(false);
        let autostart = background::startup_enabled().unwrap_or(false);
        let mic = config.input_device.as_deref().unwrap_or("System Default");

        println!();
        println!("=====================================================");
        println!(
            "                   AI DIKTE (v{})                    ",
            env!("CARGO_PKG_VERSION")
        );
        println!("=====================================================");
        println!(
            " [Status]     : Background service is {}",
            if running { "RUNNING" } else { "STOPPED" }
        );
        println!(
            " [Hotkey]     : {}",
            if cfg!(windows) {
                "Win+Z (Press to talk, press to finish)"
            } else {
                "Meta+Z"
            }
        );
        println!(" [Voice/Mode] : {} ({:?})", config.language, config.mode);
        println!(" [Microphone] : {mic}");
        println!("-----------------------------------------------------");
        println!(" [1] Enter / Update API Key");
        println!(" [2] System & Connection Diagnostics (Doctor)");
        println!(" [3] Writing Style (Smart / Verbatim) & Vocabulary");
        println!(
            " [4] Start at Sign-in: [{}]",
            if autostart { "ENABLED" } else { "DISABLED" }
        );
        println!(
            " [5] Background Service: [{}]",
            if running { "Stop" } else { "Start" }
        );
        println!(" [0] Exit");
        println!("-----------------------------------------------------");
        print!(" Choice [0-5]: ");
        io::stdout().flush()?;

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => update_api_key(&mut config, &path)?,
            "2" => run_doctor(&path)?,
            "3" => update_style_and_vocabulary(&mut config, &path)?,
            "4" => toggle_autostart(autostart)?,
            "5" => toggle_daemon(running)?,
            "0" => {
                println!("\n Exiting AI Dikte menu.");
                break;
            }
            _ => println!(" [!] Invalid choice! Please enter a number between 0 and 5."),
        }
    }
    Ok(())
}

fn update_api_key(config: &mut Config, path: &Path) -> Result<()> {
    println!();
    println!("--- Update API Key ---");
    let current = config.key().unwrap_or_default();
    let masked = mask_key(&current);
    println!(" Current Key: {masked}");
    println!(" (Press Enter without typing to keep the current key)");
    print!(" New Gemini API Key: ");
    io::stdout().flush()?;

    let mut new_key = String::new();
    io::stdin().read_line(&mut new_key)?;
    let new_key = new_key.trim();

    if new_key.is_empty() {
        println!(" -> No change made.");
        return Ok(());
    }

    print!(" -> Verifying new key... ");
    io::stdout().flush()?;

    let test_result = {
        let cfg = config.clone();
        let k = new_key.to_string();
        tokio::runtime::Runtime::new()?.block_on(live::validate_key(&cfg, &k))
    };

    match test_result {
        Ok(()) => {
            println!("SUCCESS!");
            settings::save_verified(config.clone(), new_key.to_string(), path, |_, _| Ok(()))?;
            println!(" [OK] New API key saved successfully.");
        }
        Err(err) => {
            println!("FAILED!");
            println!(" [!] Verification failed: {err:#}");
            println!("     Previous settings preserved.");
        }
    }
    Ok(())
}

fn run_doctor(path: &Path) -> Result<()> {
    println!();
    println!("--- System & Connection Diagnostics ---");
    println!("{}", diagnostics::report());

    if let Ok(config) = Config::load(path)
        && let Ok(key) = config.key()
    {
        print!(" -> Testing live Gemini WebSocket connection... ");
        io::stdout().flush()?;
        match tokio::runtime::Runtime::new()?.block_on(live::validate_key(&config, &key)) {
            Ok(()) => println!("SUCCESS!"),
            Err(e) => println!("FAILED: {e:#}"),
        }
    }

    println!("\n Press Enter to return to menu...");
    let mut pause = String::new();
    io::stdin().read_line(&mut pause)?;
    Ok(())
}

fn update_style_and_vocabulary(config: &mut Config, path: &Path) -> Result<()> {
    loop {
        println!();
        println!("--- Writing Style & Vocabulary ---");
        println!(" [1] Writing Style    : {:?}", config.mode);
        println!(
            " [2] Custom Vocabulary: {} terms configured",
            config.custom_vocabulary.len()
        );
        println!(" [0] Back");
        print!(" Choice [0-2]: ");
        io::stdout().flush()?;

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => {
                config.mode = if config.mode == Mode::Smart {
                    Mode::Verbatim
                } else {
                    Mode::Smart
                };
                save_config_change(config, path)?;
                println!(" [OK] Writing style changed to {:?}.", config.mode);
                if config.mode == Mode::Smart {
                    println!("     (Smart: Cleans up filler words and adds punctuation)");
                } else {
                    println!("     (Verbatim: Types exact spoken words and repetitions)");
                }
            }
            "2" => {
                let words_display = if config.custom_vocabulary.is_empty() {
                    " (None)".to_string()
                } else {
                    config.custom_vocabulary.join(", ")
                };
                println!("\n Current Custom Vocabulary:\n {words_display}");
                println!(" Enter comma-separated terms (leave empty to keep current):");
                print!(" Terms: ");
                io::stdout().flush()?;
                let mut words = String::new();
                io::stdin().read_line(&mut words)?;
                let words = words.trim();
                if !words.is_empty() {
                    config.custom_vocabulary = parse_vocabulary(words);
                    save_config_change(config, path)?;
                    println!(" [OK] Custom vocabulary updated.");
                }
            }
            "0" => break,
            _ => println!(" [!] Invalid choice!"),
        }
    }
    Ok(())
}

fn save_config_change(config: &Config, path: &Path) -> Result<()> {
    let key = config.key()?;
    settings::save_verified(config.clone(), key, path, |_, _| Ok(()))
}

fn toggle_autostart(current: bool) -> Result<()> {
    let target = !current;
    background::set_startup(target)?;
    println!(
        "\n [OK] Start at sign-in: {}",
        if target { "ENABLED" } else { "DISABLED" }
    );
    Ok(())
}

fn toggle_daemon(running: bool) -> Result<()> {
    if running {
        print!("\n -> Stopping background service... ");
        io::stdout().flush()?;
        background::stop()?;
        println!("STOPPED.");
    } else {
        print!("\n -> Starting background service... ");
        io::stdout().flush()?;
        background::start()?;
        println!("STARTED.");
    }
    Ok(())
}

pub fn mask_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    } else if !key.is_empty() {
        "****".to_string()
    } else {
        "Not configured".to_string()
    }
}

pub fn parse_vocabulary(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key(""), "Not configured");
        assert_eq!(mask_key("12345"), "****");
        assert_eq!(mask_key("1234567890"), "1234...7890");
    }

    #[test]
    fn test_parse_vocabulary() {
        assert_eq!(parse_vocabulary(""), Vec::<String>::new());
        assert_eq!(
            parse_vocabulary("  Kubernetes , Rust, , DevOps "),
            vec![
                "Kubernetes".to_string(),
                "Rust".to_string(),
                "DevOps".to_string()
            ]
        );
    }
}
