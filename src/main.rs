mod config;
mod entities;
mod logger;
mod save;
mod systems;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use systems::state::{App, AppPhase};

fn print_help() {
    println!("term-survivors {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    term-survivors [COMMAND]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!();
    println!("COMMANDS:");
    println!("    start    Start the game [default]");
    println!("    clear    Delete save data (~/.term_survivors)");
}

fn run_clear() -> io::Result<()> {
    let dir = dirs::home_dir().map(|h| h.join(".term_survivors"));
    match dir {
        Some(path) if path.exists() => {
            std::fs::remove_dir_all(&path)?;
            println!("Cleared: {}", path.display());
        }
        Some(path) => {
            println!("Nothing to clear: {} does not exist", path.display());
        }
        None => {
            eprintln!("Could not determine home directory");
        }
    }
    Ok(())
}

#[cfg(feature = "simulate")]
fn run_simulate(args: &[String]) -> io::Result<()> {
    use systems::simulate::{run_all, RunConfig, RunOutcome, ALL_WEAPONS};

    let mut games_per_weapon: usize = 20;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--games" {
            if let Some(n) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                games_per_weapon = n;
                i += 1;
            }
        }
        i += 1;
    }

    let cfg = RunConfig {
        games_per_weapon,
        ..Default::default()
    };
    eprintln!(
        "Running {} games per weapon ({} total)...",
        games_per_weapon,
        games_per_weapon * ALL_WEAPONS.len()
    );

    let results = run_all(&cfg);

    // CSV output to stdout (pipeable)
    println!("game,starting_weapon,outcome,elapsed_sec,kill_count,final_level,final_hp,weapons");
    for (idx, r) in results.iter().enumerate() {
        let outcome = match r.outcome {
            RunOutcome::GameOver => "GameOver",
            RunOutcome::Cleared => "Cleared",
            RunOutcome::Timeout => "Timeout",
        };
        let elapsed_sec = r.elapsed_ticks as f64 / 60.0;
        let weapons_str: Vec<&str> = r.weapons.iter().map(|w| w.name()).collect();
        println!(
            "{},{},{},{:.1},{},{},{},\"{}\"",
            idx + 1,
            r.starting_weapon.name(),
            outcome,
            elapsed_sec,
            r.kill_count,
            r.final_level,
            r.final_hp,
            weapons_str.join("|"),
        );
    }

    // Summary to stderr
    eprintln!();
    eprintln!("=== Summary by starting weapon ===");
    eprintln!(
        "{:<8}  {:<8}  {:>10}  {:>10}",
        "Weapon", "Clear%", "AvgSurv(s)", "AvgKills"
    );
    for &weapon in &ALL_WEAPONS {
        let wr: Vec<_> = results
            .iter()
            .filter(|r| r.starting_weapon == weapon)
            .collect();
        let n = wr.len();
        let clears = wr
            .iter()
            .filter(|r| matches!(r.outcome, RunOutcome::Cleared))
            .count();
        let clear_pct = 100.0 * clears as f64 / n as f64;
        let avg_sec = wr
            .iter()
            .map(|r| r.elapsed_ticks as f64 / 60.0)
            .sum::<f64>()
            / n as f64;
        let avg_kills = wr.iter().map(|r| r.kill_count as f64).sum::<f64>() / n as f64;
        eprintln!(
            "{:<8}  {:>5.1}%  {:>10.0}  {:>10.0}",
            weapon.name(),
            clear_pct,
            avg_sec,
            avg_kills
        );
    }

    Ok(())
}

fn run_game() -> io::Result<()> {
    logger::init();
    logger::info(&format!(
        "term-survivors v{} started",
        env!("CARGO_PKG_VERSION")
    ));

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let field_width = (size.width as i32 - 2).clamp(1, config::MAX_FIELD_WIDTH);
    let field_height = (size.height as i32 - config::STATUS_BAR_HEIGHT as i32 - 2)
        .clamp(1, config::MAX_FIELD_HEIGHT);
    let mut app = App::new(field_width, field_height);

    let tick_duration = Duration::from_millis(config::TICK_DURATION_MS);

    loop {
        let frame_start = Instant::now();

        terminal.draw(|frame| ui::draw(frame, &app))?;

        let mut should_break = false;
        let mut dx: i32 = 0;
        let mut dy: i32 = 0;

        while event::poll(if frame_start.elapsed() < tick_duration {
            tick_duration.saturating_sub(frame_start.elapsed())
        } else {
            Duration::ZERO
        })? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match &app.phase {
                        AppPhase::Title => match key.code {
                            KeyCode::Enter => {
                                if app.has_session {
                                    app.resume_game();
                                } else {
                                    app.start_game();
                                }
                            }
                            KeyCode::Char('n') => app.start_game(),
                            KeyCode::Char('a') => app.toggle_auto_restart(),
                            KeyCode::Char('b') => app.toggle_dark_mode(),
                            KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                        AppPhase::WeaponSelect(choices, idx) => {
                            let len = choices.len();
                            match key.code {
                                KeyCode::Char('w') | KeyCode::Up => {
                                    app.phase = AppPhase::WeaponSelect(
                                        choices.clone(),
                                        idx.saturating_sub(1),
                                    )
                                }
                                KeyCode::Char('s') | KeyCode::Down => {
                                    app.phase = AppPhase::WeaponSelect(
                                        choices.clone(),
                                        (idx + 1).min(len - 1),
                                    )
                                }
                                KeyCode::Char(' ') | KeyCode::Enter => {
                                    app.select_starting_weapon(*idx)
                                }
                                KeyCode::Char('1') => app.select_starting_weapon(0),
                                KeyCode::Char('2') => app.select_starting_weapon(1),
                                KeyCode::Char('3') => app.select_starting_weapon(2),
                                KeyCode::Char('m') => app.phase = AppPhase::Title,
                                KeyCode::Esc => app.phase = AppPhase::Title,
                                _ => {}
                            }
                        }
                        AppPhase::Playing => match key.code {
                            KeyCode::Char('a') | KeyCode::Left => {
                                dx -= 1;
                            }
                            KeyCode::Char('d') | KeyCode::Right => {
                                dx += 1;
                            }
                            KeyCode::Char('w') | KeyCode::Up => {
                                dy -= 1;
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                dy += 1;
                            }
                            KeyCode::Char(' ') => {
                                app.pause();
                                break;
                            }
                            KeyCode::Char('m') => app.return_to_title(),
                            KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                        AppPhase::Paused => match key.code {
                            KeyCode::Char(' ') => {
                                app.resume_from_pause();
                                break;
                            }
                            KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                        AppPhase::LevelUp(choices, idx) => {
                            let len = choices.len();
                            match key.code {
                                KeyCode::Char('w') | KeyCode::Up => {
                                    app.phase =
                                        AppPhase::LevelUp(choices.clone(), idx.saturating_sub(1))
                                }
                                KeyCode::Char('s') | KeyCode::Down => {
                                    app.phase = AppPhase::LevelUp(
                                        choices.clone(),
                                        (idx + 1).min(len.saturating_sub(1)),
                                    )
                                }
                                KeyCode::Char(' ') | KeyCode::Enter => app.select_upgrade(*idx),
                                KeyCode::Char('1') => app.select_upgrade(0),
                                KeyCode::Char('2') => app.select_upgrade(1),
                                KeyCode::Char('3') => app.select_upgrade(2),
                                KeyCode::Char('m') => app.return_to_title(),
                                KeyCode::Esc => {
                                    should_break = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        AppPhase::Dead { .. } => {}
                        AppPhase::GameOver | AppPhase::Cleared => match key.code {
                            KeyCode::Char('r') => app.start_game(),
                            KeyCode::Char('q') | KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                    }
                }
                Event::Resize(w, h) => {
                    let fw = (w as i32 - 2).clamp(1, config::MAX_FIELD_WIDTH);
                    let fh = (h as i32 - config::STATUS_BAR_HEIGHT as i32 - 2)
                        .clamp(1, config::MAX_FIELD_HEIGHT);
                    app.resize(fw, fh);
                }
                _ => {}
            }
        }

        if should_break {
            break;
        }

        app.dx = dx.clamp(-1, 1);
        app.dy = dy.clamp(-1, 1);
        app.tick();
    }

    app.save_session();

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // simulate has its own flags, handle before generic arg parsing
    #[cfg(feature = "simulate")]
    if args.first().map(|s| s.as_str()) == Some("simulate") {
        return run_simulate(&args[1..]);
    }

    let mut subcommand: Option<&str> = None;
    for arg in &args {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("term-survivors {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "start" => subcommand = Some("start"),
            "clear" => subcommand = Some("clear"),
            unknown => {
                eprintln!("Unknown argument: {}", unknown);
                eprintln!("Run 'term-survivors --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    match subcommand.unwrap_or("start") {
        "clear" => run_clear(),
        _ => run_game(),
    }
}
