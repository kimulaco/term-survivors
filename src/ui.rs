use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph};
use ratatui::Frame;

use crate::config;
use crate::entities::enemy::{Enemy, EnemyKind};
use crate::save::Settings;
use crate::systems::state::{App, AppPhase, WEAPON_CHOICES};

/// Distinct colors per enemy kind — all in red/orange/purple/pink/brown range.
fn enemy_color(kind: EnemyKind) -> Color {
    match kind {
        EnemyKind::Bug => Color::Red,
        EnemyKind::Virus => Color::Rgb(160, 80, 220), // purple
        EnemyKind::Crash => Color::Rgb(255, 140, 0),  // orange
        EnemyKind::MemLeak => Color::Rgb(160, 100, 50), // brown
        EnemyKind::Elite => Color::Rgb(255, 160, 180), // light pink
        EnemyKind::MidBoss => Color::Rgb(180, 60, 100), // dark rose
        EnemyKind::Boss => Color::Rgb(220, 20, 60),   // crimson
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Minimum size check
    if size.width < config::MIN_WIDTH || size.height < config::MIN_HEIGHT {
        let msg = Paragraph::new(format!(
            "Terminal too small!\nMinimum: {}x{}\nCurrent: {}x{}",
            config::MIN_WIDTH,
            config::MIN_HEIGHT,
            size.width,
            size.height
        ))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
        frame.render_widget(msg, size);
        return;
    }

    match &app.phase {
        AppPhase::Title => draw_title(frame, size, app),
        AppPhase::WeaponSelect(idx) => {
            draw_game(frame, size, app);
            draw_weapon_select(frame, size, *idx);
        }
        AppPhase::Playing => draw_game(frame, size, app),
        AppPhase::Paused => {
            draw_game(frame, size, app);
            draw_pause_overlay(frame, size);
        }
        AppPhase::LevelUp(choices, idx) => {
            draw_game(frame, size, app);
            draw_levelup(frame, size, choices, &app.game.weapons, *idx);
        }
        AppPhase::GameOver => draw_game_over(frame, size, app),
        AppPhase::Cleared => draw_cleared(frame, size, app),
    }
}

fn draw_title(frame: &mut Frame, area: Rect, app: &App) {
    let enter_line = if app.has_session {
        "║   Press [ENTER] to Continue        ║"
    } else {
        "║   Press [ENTER] to Start           ║"
    };
    let sound_line = if app.save.sound_enabled {
        "║   [V] Sound: ON                    ║"
    } else {
        "║   [V] Sound: OFF                   ║"
    };
    let auto_restart_line = if app.save.auto_restart {
        "║   [A] Auto-Restart: ON             ║"
    } else {
        "║   [A] Auto-Restart: OFF            ║"
    };

    let mut title_art = vec![
        "╔════════════════════════════════════╗",
        "║      TERM SURVIVORS                ║",
        "║                                    ║",
        "║   A Terminal Roguelike Shooter     ║",
        "║                                    ║",
        enter_line,
    ];
    if app.has_session {
        title_art.push("║   Press [N] to New Game            ║");
    }
    title_art.extend_from_slice(&[
        "║   Press [ESC] to Quit              ║",
        "║                                    ║",
        "║   Settings:                        ║",
        sound_line,
        auto_restart_line,
        "║                                    ║",
        "║   Controls:                        ║",
        "║   WASD - Move                      ║",
        "║   SPACE - Pause (in game)          ║",
        "║   1/2/3 - Select upgrades          ║",
        "║   ESC - Quit                       ║",
        "╚════════════════════════════════════╝",
    ]);

    let mut lines: Vec<Line> = Vec::new();
    let start_y = (area.height as usize).saturating_sub(title_art.len() + 4) / 2;
    for _ in 0..start_y {
        lines.push(Line::from(""));
    }
    for line in &title_art {
        lines.push(Line::from(Span::styled(
            *line,
            Style::default().fg(Color::Cyan),
        )));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn draw_weapon_select(frame: &mut Frame, area: Rect, selected: usize) {
    let popup_width = 44u16;
    let popup_height = (3 + WEAPON_CHOICES.len() * 3) as u16;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + config::STATUS_BAR_HEIGHT + 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " Choose Your Starting Weapon ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, kind) in WEAPON_CHOICES.iter().enumerate() {
        let (prefix, name_color) = if i == selected {
            (">", Color::Yellow)
        } else {
            (" ", Color::Cyan)
        };
        lines.push(Line::from(Span::styled(
            format!(" {} [{}] {} ", prefix, i + 1, kind.name()),
            Style::default().fg(name_color),
        )));
        lines.push(Line::from(Span::styled(
            format!("       {}", kind.description()),
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
    }

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Weapon ")
                .style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(popup, popup_area);
}

fn draw_game(frame: &mut Frame, area: Rect, app: &App) {
    let max_w = (config::MAX_FIELD_WIDTH as u16 + 2).min(area.width);
    let max_h = (config::MAX_FIELD_HEIGHT as u16 + 2)
        .min(area.height.saturating_sub(config::STATUS_BAR_HEIGHT));

    let game_area = Rect {
        x: area.x + (area.width.saturating_sub(max_w)) / 2,
        y: area.y,
        width: max_w,
        height: area.height,
    };

    let chunks = Layout::vertical([
        Constraint::Length(config::STATUS_BAR_HEIGHT),
        Constraint::Length(max_h),
        Constraint::Min(1),
    ])
    .split(game_area);

    draw_status_bar(frame, chunks[0], app);
    draw_field(frame, chunks[1], app);
    draw_controls(frame, chunks[2]);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let game = &app.game;
    let player = &game.player;

    let rows = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(area);

    let top_chunks =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(rows[0]);

    // HP bar
    let hp_ratio = player.hp as f64 / player.max_hp as f64;
    let hp_color = if hp_ratio > 0.5 {
        Color::Green
    } else if hp_ratio > 0.25 {
        Color::Yellow
    } else {
        Color::Red
    };

    let hp_label_color = if hp_ratio > 0.5 {
        Color::Black
    } else {
        Color::White
    };
    let hp_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" HP "))
        .gauge_style(Style::default().fg(hp_color))
        .ratio(hp_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            format!("{}/{}", player.hp, player.max_hp),
            Style::default().fg(hp_label_color),
        ));
    frame.render_widget(hp_gauge, top_chunks[0]);

    // Info
    let remaining = config::GAME_DURATION_TICKS.saturating_sub(game.elapsed_ticks);
    let time_str = Settings::format_ticks(remaining);
    let weapons_str: String = game
        .weapons
        .iter()
        .map(|w| format!("{}({})", w.kind.name(), w.level))
        .collect::<Vec<_>>()
        .join(" ");

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" Time: {}  ", time_str),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("Kills:{} ", game.kill_count),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(Span::styled(
            format!(" Weapons: {}", weapons_str),
            Style::default().fg(Color::Cyan),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Info "));
    frame.render_widget(info, top_chunks[1]);

    // XP gauge (full width, below HP & Info)
    let xp_threshold = game.xp_threshold();
    let xp_ratio = if xp_threshold > 0 {
        game.xp as f64 / xp_threshold as f64
    } else {
        0.0
    };

    let xp_label_color = if xp_ratio > 0.5 {
        Color::Black
    } else {
        Color::White
    };
    let xp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Lv.{} ", game.level)),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(xp_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            format!("{}/{}", game.xp, xp_threshold),
            Style::default().fg(xp_label_color),
        ));
    frame.render_widget(xp_gauge, rows[1]);
}

fn draw_boss_hp_bar(buf: &mut Buffer, enemy: &Enemy, inner: Rect, sdx: i32, sdy: i32) {
    let hp_ratio = enemy.hp as f64 / enemy.max_hp as f64;
    let bar_width = enemy.width.min(inner.width as i32);
    let filled = (hp_ratio * bar_width as f64) as i32;
    let bar_y = inner.y as i32 + enemy.y - 1 + sdy;
    if bar_y >= inner.y as i32 {
        for bx in 0..bar_width {
            let sx = inner.x as i32 + enemy.x + bx + sdx;
            if in_bounds(inner, sx, bar_y) {
                let ch = if bx < filled { '█' } else { '░' };
                set_cell(buf, sx as u16, bar_y as u16, ch, Color::Red);
            }
        }
    }
}

fn draw_field(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" Arena ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let buf = frame.buffer_mut();
    let game = &app.game;
    let (sdx, sdy) = app.screen_shake_offset();

    for proj in &game.projectiles {
        // Thunder warning indicator (damage=0, weapon_kind_idx=5): 3-phase blink
        let (glyph, color) = if proj.damage == 0 && proj.weapon_kind_idx == 5 {
            let period: u32 = if proj.ttl > 30 {
                8
            } else if proj.ttl > 15 {
                5
            } else {
                3
            };
            if proj.ttl % period >= period / 2 {
                (' ', Color::Reset)
            } else if proj.ttl > 30 {
                ('?', Color::Yellow)
            } else if proj.ttl > 15 {
                ('!', Color::LightYellow)
            } else {
                ('#', Color::LightRed)
            }
        } else if proj.weapon_kind_idx == 5 && proj.delay_ticks > 0 {
            // Thunder strike cell during warn phase: preview
            if proj.delay_ticks > 30 {
                ('.', Color::DarkGray)
            } else if proj.delay_ticks > 15 {
                ('.', Color::Gray)
            } else {
                ('.', Color::Yellow)
            }
        } else if proj.weapon_kind_idx == 5 {
            // Thunder strike (active)
            (proj.glyph, Color::White)
        // Fuse indicator (damage=0, weapon_kind_idx=3): blink + phase change
        } else if proj.damage == 0 && proj.weapon_kind_idx == 3 {
            let period: u32 = if proj.ttl > 60 {
                16
            } else if proj.ttl > 30 {
                8
            } else {
                4
            };
            if proj.ttl % period >= period / 2 {
                (' ', Color::Reset) // write space to actively clear the cell
            } else if proj.ttl > 60 {
                ('O', Color::LightYellow)
            } else if proj.ttl > 30 {
                ('o', Color::Yellow)
            } else {
                ('*', Color::LightRed)
            }
        } else if proj.weapon_kind_idx == 3 && proj.delay_ticks > 0 {
            // Bomb explosion cell during fuse: preview color matches fuse indicator phase
            if proj.delay_ticks > 60 {
                ('.', Color::DarkGray)
            } else if proj.delay_ticks > 30 {
                ('.', Color::Gray)
            } else {
                ('.', Color::Yellow)
            }
        } else if proj.weapon_kind_idx == 3 {
            // Bomb explosion cell detonating
            (proj.glyph, Color::Yellow)
        } else {
            let c = match proj.weapon_kind_idx {
                0 => Color::LightBlue,  // Orbit
                1 => Color::Yellow,     // Laser
                2 => Color::Cyan,       // Drone
                4 => Color::LightGreen, // Scatter
                _ => Color::LightBlue,
            };
            (proj.glyph, c)
        };
        let w = proj.width.max(1);
        let h = proj.height.max(1);
        for dy in 0..h {
            for dx in 0..w {
                let sx = inner.x as i32 + proj.x + dx + sdx;
                let sy = inner.y as i32 + proj.y + dy + sdy;
                if in_bounds(inner, sx, sy) {
                    set_cell(buf, sx as u16, sy as u16, glyph, color);
                }
            }
        }
    }

    // Draw enemies
    for enemy in &game.enemies {
        let color = enemy_color(enemy.kind);

        if enemy.kind == EnemyKind::Boss {
            for by in 0..enemy.height {
                for bx in 0..enemy.width {
                    let sx = inner.x as i32 + enemy.x + bx + sdx;
                    let sy = inner.y as i32 + enemy.y + by + sdy;
                    if in_bounds(inner, sx, sy) {
                        let ch = if by == 0 && (bx == 0 || bx == enemy.width - 1) {
                            '╔'
                        } else if by == enemy.height - 1 && (bx == 0 || bx == enemy.width - 1) {
                            '╚'
                        } else {
                            enemy.glyph
                        };
                        set_cell(buf, sx as u16, sy as u16, ch, color);
                    }
                }
            }
            draw_boss_hp_bar(buf, enemy, inner, sdx, sdy);
        } else if enemy.kind == EnemyKind::MidBoss {
            for by in 0..enemy.height {
                for bx in 0..enemy.width {
                    let sx = inner.x as i32 + enemy.x + bx + sdx;
                    let sy = inner.y as i32 + enemy.y + by + sdy;
                    if in_bounds(inner, sx, sy) {
                        set_cell(buf, sx as u16, sy as u16, enemy.glyph, color);
                    }
                }
            }
            draw_boss_hp_bar(buf, enemy, inner, sdx, sdy);
        } else {
            let sx = inner.x as i32 + enemy.x + sdx;
            let sy = inner.y as i32 + enemy.y + sdy;
            if in_bounds(inner, sx, sy) {
                set_cell(buf, sx as u16, sy as u16, enemy.glyph, color);
            }
        }
    }

    // Draw player
    let px = inner.x as i32 + game.player.x + sdx;
    let py = inner.y as i32 + game.player.y + sdy;
    if in_bounds(inner, px, py) {
        let player_style =
            if game.player.invincible_ticks > 0 && game.player.invincible_ticks % 6 < 3 {
                Color::Yellow // Blink when invincible
            } else {
                Color::LightBlue
            };
        set_cell(
            buf,
            px as u16,
            py as u16,
            config::PLAYER_GLYPH,
            player_style,
        );
    }
}

fn in_bounds(inner: Rect, x: i32, y: i32) -> bool {
    x >= inner.x as i32
        && x < (inner.x + inner.width) as i32
        && y >= inner.y as i32
        && y < (inner.y + inner.height) as i32
}

fn set_cell(buf: &mut Buffer, x: u16, y: u16, ch: char, color: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(ch);
        cell.set_fg(color);
    }
}

fn draw_controls(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("[WASD] ", Style::default().fg(Color::Cyan)),
        Span::styled("Move  ", Style::default().fg(Color::Gray)),
        Span::styled("[SPACE] ", Style::default().fg(Color::Cyan)),
        Span::styled("Pause  ", Style::default().fg(Color::Gray)),
        Span::styled("[M] ", Style::default().fg(Color::Cyan)),
        Span::styled("Menu  ", Style::default().fg(Color::Gray)),
        Span::styled("[ESC] ", Style::default().fg(Color::Cyan)),
        Span::styled("Quit", Style::default().fg(Color::Gray)),
    ]);
    let para = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(para, area);
}

fn draw_levelup(
    frame: &mut Frame,
    area: Rect,
    choices: &[crate::systems::levelup::Upgrade],
    weapons: &[crate::entities::weapon::Weapon],
    selected: usize,
) {
    let popup_width = 40u16;
    let popup_height = (3 + choices.len() * 3) as u16;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + config::STATUS_BAR_HEIGHT + 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            " LEVEL UP! ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, choice) in choices.iter().enumerate() {
        let (prefix, name_color) = if i == selected {
            (">", Color::Yellow)
        } else {
            (" ", Color::Cyan)
        };
        lines.push(Line::from(Span::styled(
            format!(" {} [{}] {} ", prefix, i + 1, choice.name(weapons)),
            Style::default().fg(name_color),
        )));
        lines.push(Line::from(Span::styled(
            format!("       {}", choice.description(weapons)),
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
    }

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Choose Upgrade ")
                .style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(popup, popup_area);
}

fn draw_pause_overlay(frame: &mut Frame, area: Rect) {
    let max_w = (config::MAX_FIELD_WIDTH as u16 + 2).min(area.width);
    let max_h = (config::MAX_FIELD_HEIGHT as u16 + 2)
        .min(area.height.saturating_sub(config::STATUS_BAR_HEIGHT));
    let arena_x = area.x + (area.width.saturating_sub(max_w)) / 2;
    let arena_y = area.y + config::STATUS_BAR_HEIGHT;
    let arena_area = Rect::new(arena_x, arena_y, max_w, max_h);

    frame.render_widget(Clear, arena_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Arena ")
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(arena_area);
    frame.render_widget(block, arena_area);

    let msg_height = 3u16;
    let msg_y = inner.y + inner.height.saturating_sub(msg_height) / 2;
    let msg_area = Rect::new(inner.x, msg_y, inner.width, msg_height);

    let lines = vec![
        Line::from(Span::styled(
            "PAUSED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[SPACE] Restart  [ESC] Quit",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, msg_area);
}

fn draw_game_over(frame: &mut Frame, area: Rect, app: &App) {
    let game = &app.game;
    let elapsed = Settings::format_ticks(game.elapsed_ticks);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GAME OVER",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Time: {}", elapsed)),
        Line::from(format!("Level: {}", game.level)),
        Line::from(format!("Kills: {}", game.kill_count)),
        Line::from(""),
        Line::from(Span::styled(
            "[R] Retry  [Q] Quit",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Result "))
        .alignment(Alignment::Center);

    let popup_width = 36u16;
    let popup_height = 10u16;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn draw_cleared(frame: &mut Frame, area: Rect, app: &App) {
    let game = &app.game;
    let elapsed = Settings::format_ticks(game.elapsed_ticks);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "★ CLEARED! ★",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "You defeated the Kernel Panic!",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!("Time: {}", elapsed)),
        Line::from(format!("Level: {}", game.level)),
        Line::from(format!("Kills: {}", game.kill_count)),
        Line::from(""),
        Line::from(Span::styled(
            "[R] Play Again  [Q] Quit",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Victory! "))
        .alignment(Alignment::Center);

    let popup_width = 36u16;
    let popup_height = 12u16;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}
