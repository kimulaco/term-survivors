mod audio;
mod config;
mod entities;
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

use systems::state::{App, AppPhase, WEAPON_CHOICES};

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

fn run_game() -> io::Result<()> {
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
    let field_width = (size.width as i32 - 2).max(1).min(config::MAX_FIELD_WIDTH);
    let field_height = (size.height as i32 - config::STATUS_BAR_HEIGHT as i32 - 2)
        .max(1)
        .min(config::MAX_FIELD_HEIGHT);
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
                            KeyCode::Char('v') => app.toggle_mute(),
                            KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                        AppPhase::WeaponSelect(idx) => {
                            let len = WEAPON_CHOICES.len();
                            match key.code {
                                KeyCode::Char('w') | KeyCode::Up => {
                                    app.phase = AppPhase::WeaponSelect(idx.saturating_sub(1))
                                }
                                KeyCode::Char('s') | KeyCode::Down => {
                                    app.phase = AppPhase::WeaponSelect((idx + 1).min(len - 1))
                                }
                                KeyCode::Char(' ') | KeyCode::Enter => {
                                    app.select_starting_weapon(*idx)
                                }
                                KeyCode::Char('1') => app.select_starting_weapon(0),
                                KeyCode::Char('2') => app.select_starting_weapon(1),
                                KeyCode::Char('3') => app.select_starting_weapon(2),
                                KeyCode::Char('4') => app.select_starting_weapon(3),
                                KeyCode::Char('v') => app.toggle_mute(),
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
                            KeyCode::Char(' ') => app.pause(),
                            KeyCode::Char('m') => app.return_to_title(),
                            KeyCode::Char('v') => app.toggle_mute(),
                            KeyCode::Esc => {
                                should_break = true;
                                break;
                            }
                            _ => {}
                        },
                        AppPhase::Paused => match key.code {
                            KeyCode::Char(' ') => app.resume_from_pause(),
                            KeyCode::Char('v') => app.toggle_mute(),
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
                                KeyCode::Char('v') => app.toggle_mute(),
                                KeyCode::Esc => {
                                    should_break = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
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
                    let fw = (w as i32 - 2).max(1).min(config::MAX_FIELD_WIDTH);
                    let fh = (h as i32 - config::STATUS_BAR_HEIGHT as i32 - 2)
                        .max(1)
                        .min(config::MAX_FIELD_HEIGHT);
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
