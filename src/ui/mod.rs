use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph};
use ratatui::Frame;

use crate::config;
use crate::entities::enemy::{Enemy, EnemyKind};
use crate::entities::weapon::WeaponKind;
use crate::save::Settings;
use crate::systems::state::{App, AppPhase};

mod theme;
use theme::{
    bg, border_style, border_style_damage, enemy_color, gauge_fg_style, gauge_label_color,
    player_color, popup_header_style, text_color, BOMB_EXPLODE_COLOR, BOMB_FUSE_FAR_COLOR,
    BOMB_FUSE_MID_COLOR, BOMB_FUSE_NEAR_COLOR, BOSS_HP_BAR_COLOR, DARK_BG,
    DEFAULT_PROJECTILE_COLOR, DELAY_PREVIEW_FAR_COLOR, DELAY_PREVIEW_MID_COLOR,
    DELAY_PREVIEW_NEAR_COLOR, HP_HIGH_COLOR, HP_LOW_COLOR, HP_MID_COLOR, LASER_COLOR,
    THUNDER_ACTIVE_COLOR, THUNDER_WARN_FAR_COLOR, THUNDER_WARN_MID_COLOR, THUNDER_WARN_NEAR_COLOR,
};

/// Clear must run first to erase game-entity characters beneath the popup.
fn fill_bg(frame: &mut ratatui::Frame, area: Rect, dark: bool) {
    frame.render_widget(Clear, area);
    if dark {
        frame.render_widget(Block::default().style(Style::default().bg(DARK_BG)), area);
    }
}

fn popup_rect_top(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + config::STATUS_BAR_HEIGHT + 2;
    Rect::new(x, y, width, height)
}

fn popup_rect_centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let dark = app.save.dark_mode;

    if dark {
        frame.buffer_mut().set_style(size, bg(dark));
    }

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
        AppPhase::WeaponSelect(choices, idx) => {
            draw_game(frame, size, app);
            draw_weapon_select(frame, size, choices, *idx, dark);
        }
        AppPhase::Playing | AppPhase::Dead { .. } => draw_game(frame, size, app),
        AppPhase::Paused => {
            draw_game(frame, size, app);
            draw_pause_overlay(frame, size, app);
        }
        AppPhase::LevelUp(choices, idx) => {
            draw_game(frame, size, app);
            draw_levelup(frame, size, choices, &app.game.weapons, *idx, dark);
        }
        AppPhase::GameOver => draw_game_over(frame, size, app),
        AppPhase::Cleared => draw_cleared(frame, size, app),
    }
}

fn draw_title(frame: &mut Frame, area: Rect, app: &App) {
    let dark = app.save.dark_mode;
    let enter_line = if app.has_session {
        "║   Press [ENTER] to Continue        ║"
    } else {
        "║   Press [ENTER] to Start           ║"
    };
    let auto_restart_line = if app.save.auto_restart {
        "║   [A] Auto-Restart: ON             ║"
    } else {
        "║   [A] Auto-Restart: OFF            ║"
    };
    let dark_bg_line = if dark {
        "║   [B] Dark Mode: ON                ║"
    } else {
        "║   [B] Dark Mode: OFF               ║"
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
        auto_restart_line,
        dark_bg_line,
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

/// Appends numbered choice rows (name + description) into `lines`.
fn push_choice_lines(
    lines: &mut Vec<Line<'_>>,
    count: usize,
    selected: usize,
    name_fn: impl Fn(usize) -> String,
    desc_fn: impl Fn(usize) -> String,
) {
    for i in 0..count {
        let (prefix, name_color) = if i == selected {
            (">", Color::Yellow)
        } else {
            (" ", Color::Cyan)
        };
        lines.push(Line::from(Span::styled(
            format!(" {} [{}] {} ", prefix, i + 1, name_fn(i)),
            Style::default().fg(name_color),
        )));
        lines.push(Line::from(Span::styled(
            format!("       {}", desc_fn(i)),
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
    }
}

fn draw_weapon_select(
    frame: &mut Frame,
    area: Rect,
    choices: &[WeaponKind],
    selected: usize,
    dark_mode: bool,
) {
    let popup_width = 44u16;
    let popup_height = (3 + choices.len() * 3) as u16;
    let popup_area = popup_rect_top(area, popup_width, popup_height);

    fill_bg(frame, popup_area, dark_mode);

    let mut lines = vec![
        Line::from(Span::styled(
            " Choose Your Starting Weapon ",
            popup_header_style(),
        )),
        Line::from(""),
    ];

    push_choice_lines(
        &mut lines,
        choices.len(),
        selected,
        |i| choices[i].name().to_string(),
        |i| choices[i].description().to_string(),
    );

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Weapon ")
                .style(bg(dark_mode).fg(Color::Yellow)),
        )
        .style(bg(dark_mode))
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
    let dark = app.save.dark_mode;
    let block_style = border_style_damage(dark, app.is_damage_border_active());

    if dark {
        frame.buffer_mut().set_style(area, bg(dark));
    }

    let rows = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(area);

    let top_chunks = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(9),
        Constraint::Min(0),
    ])
    .split(rows[0]);

    // HP bar
    let hp_ratio = player.hp as f64 / player.max_hp as f64;
    let hp_color = if hp_ratio > 0.5 {
        HP_HIGH_COLOR
    } else if hp_ratio > 0.25 {
        HP_MID_COLOR
    } else {
        HP_LOW_COLOR
    };
    let hp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" HP ")
                .style(block_style),
        )
        .style(bg(dark))
        .gauge_style(gauge_fg_style(hp_color, dark))
        .ratio(hp_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            format!("{}/{}", player.hp, player.max_hp),
            Style::default().fg(gauge_label_color(hp_ratio)),
        ));
    frame.render_widget(hp_gauge, top_chunks[0]);

    // Time
    let remaining = config::GAME_DURATION_TICKS.saturating_sub(game.elapsed_ticks);
    let time_str = Settings::format_ticks(remaining);
    let time_widget = Paragraph::new(Span::styled(
        format!(" {}", time_str),
        Style::default().fg(Color::Green),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Time ")
            .style(block_style),
    )
    .style(bg(dark));
    frame.render_widget(time_widget, top_chunks[1]);

    // Weapons
    let weapon_slots: String = (0..config::MAX_WEAPONS)
        .map(|i| {
            if let Some(w) = game.weapons.get(i) {
                format!("[{}{}]", w.kind.abbr(), w.level)
            } else {
                "[--]".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let weapons_widget = Paragraph::new(Span::styled(
        format!(" {}", weapon_slots),
        Style::default().fg(Color::Cyan),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Weapons ")
            .style(block_style),
    )
    .style(bg(dark));
    frame.render_widget(weapons_widget, top_chunks[2]);

    // XP gauge (full width, below HP & Info)
    let xp_threshold = game.xp_threshold();
    let xp_ratio = if xp_threshold > 0 {
        game.xp as f64 / xp_threshold as f64
    } else {
        0.0
    };
    let xp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Lv.{} ", game.level))
                .style(block_style),
        )
        .style(bg(dark))
        .gauge_style(gauge_fg_style(Color::Cyan, dark))
        .ratio(xp_ratio.clamp(0.0, 1.0))
        .label(Span::styled(
            format!("{}/{}", game.xp, xp_threshold),
            Style::default().fg(gauge_label_color(xp_ratio)),
        ));
    frame.render_widget(xp_gauge, rows[1]);
}

/// Returns the preview cell for a delayed strike based on remaining delay ticks.
/// `far` and `mid` are the thresholds between the three color phases.
fn delay_preview_cell(delay_ticks: u32, far_threshold: u32, mid_threshold: u32) -> (char, Color) {
    if delay_ticks > far_threshold {
        ('.', DELAY_PREVIEW_FAR_COLOR)
    } else if delay_ticks > mid_threshold {
        ('.', DELAY_PREVIEW_MID_COLOR)
    } else {
        ('.', DELAY_PREVIEW_NEAR_COLOR)
    }
}

/// Draws a multi-cell enemy body and its HP bar above.
fn draw_multi_cell_enemy(
    buf: &mut Buffer,
    enemy: &Enemy,
    inner: Rect,
    sdx: i32,
    sdy: i32,
    color: Color,
    glyph_fn: impl Fn(i32, i32) -> char,
) {
    for by in 0..enemy.height {
        for bx in 0..enemy.width {
            let sx = inner.x as i32 + enemy.x + bx + sdx;
            let sy = inner.y as i32 + enemy.y + by + sdy;
            if in_bounds(inner, sx, sy) {
                set_cell(buf, sx as u16, sy as u16, glyph_fn(bx, by), color);
            }
        }
    }
    draw_boss_hp_bar(buf, enemy, inner, sdx, sdy);
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
                set_cell(buf, sx as u16, bar_y as u16, ch, BOSS_HP_BAR_COLOR);
            }
        }
    }
}

fn draw_field(frame: &mut Frame, area: Rect, app: &App) {
    let dark = app.save.dark_mode;
    let arena_style = border_style_damage(dark, app.is_damage_border_active());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Arena ")
        .style(arena_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let buf = frame.buffer_mut();
    let game = &app.game;
    let (sdx, sdy) = app.screen_shake_offset();

    for proj in &game.projectiles {
        let wk = WeaponKind::from_idx(proj.weapon_kind_idx);
        // Thunder warning indicator (damage=0): 3-phase blink
        let (glyph, color) = if proj.damage == 0 && wk == WeaponKind::Thunder {
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
                ('?', THUNDER_WARN_FAR_COLOR)
            } else if proj.ttl > 15 {
                ('!', THUNDER_WARN_MID_COLOR)
            } else {
                ('#', THUNDER_WARN_NEAR_COLOR)
            }
        } else if wk == WeaponKind::Thunder && proj.delay_ticks > 0 {
            // Thunder strike cell during warn phase: preview
            delay_preview_cell(proj.delay_ticks, 30, 15)
        } else if wk == WeaponKind::Thunder {
            // Thunder strike (active)
            (proj.glyph, THUNDER_ACTIVE_COLOR)
        // Fuse indicator (damage=0): blink + phase change
        } else if proj.damage == 0 && wk == WeaponKind::Bomb {
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
                ('O', BOMB_FUSE_FAR_COLOR)
            } else if proj.ttl > 30 {
                ('o', BOMB_FUSE_MID_COLOR)
            } else {
                ('*', BOMB_FUSE_NEAR_COLOR)
            }
        } else if wk == WeaponKind::Bomb && proj.delay_ticks > 0 {
            // Bomb explosion cell during fuse: preview color matches fuse indicator phase
            delay_preview_cell(proj.delay_ticks, 60, 30)
        } else if wk == WeaponKind::Bomb {
            // Bomb explosion cell detonating
            (proj.glyph, BOMB_EXPLODE_COLOR)
        } else {
            let c = match wk {
                WeaponKind::Laser => LASER_COLOR,
                _ => DEFAULT_PROJECTILE_COLOR,
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
            draw_multi_cell_enemy(buf, enemy, inner, sdx, sdy, color, |bx, by| {
                if by == 0 && (bx == 0 || bx == enemy.width - 1) {
                    '╔'
                } else if by == enemy.height - 1 && (bx == 0 || bx == enemy.width - 1) {
                    '╚'
                } else {
                    enemy.glyph
                }
            });
        } else if enemy.kind == EnemyKind::MidBoss {
            draw_multi_cell_enemy(buf, enemy, inner, sdx, sdy, color, |_, _| enemy.glyph);
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
        let player_color = player_color(&game.player);
        set_cell(
            buf,
            px as u16,
            py as u16,
            config::PLAYER_GLYPH,
            player_color,
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
    dark_mode: bool,
) {
    let popup_width = 40u16;
    let popup_height = (3 + choices.len() * 3) as u16;
    let popup_area = popup_rect_top(area, popup_width, popup_height);

    fill_bg(frame, popup_area, dark_mode);

    let mut lines = vec![
        Line::from(Span::styled(" LEVEL UP! ", popup_header_style())),
        Line::from(""),
    ];

    push_choice_lines(
        &mut lines,
        choices.len(),
        selected,
        |i| choices[i].name(weapons),
        |i| choices[i].description(weapons),
    );

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Choose Upgrade ")
                .style(bg(dark_mode).fg(Color::Yellow)),
        )
        .style(bg(dark_mode))
        .alignment(Alignment::Left);
    frame.render_widget(popup, popup_area);
}

fn draw_pause_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let dark = app.save.dark_mode;
    let max_w = (config::MAX_FIELD_WIDTH as u16 + 2).min(area.width);
    let max_h = (config::MAX_FIELD_HEIGHT as u16 + 2)
        .min(area.height.saturating_sub(config::STATUS_BAR_HEIGHT));
    let arena_x = area.x + (area.width.saturating_sub(max_w)) / 2;
    let arena_y = area.y + config::STATUS_BAR_HEIGHT;
    let arena_area = Rect::new(arena_x, arena_y, max_w, max_h);

    fill_bg(frame, arena_area, dark);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Arena ")
        .style(bg(dark).fg(Color::DarkGray));
    let inner = block.inner(arena_area);
    frame.render_widget(block, arena_area);

    let weapons = &app.game.weapons;
    let msg_height = 3u16 + config::MAX_WEAPONS as u16 + 2;
    let msg_y = inner.y + inner.height.saturating_sub(msg_height) / 2;
    let msg_area = Rect::new(inner.x, msg_y, inner.width, msg_height);

    let mut lines = vec![
        Line::from(Span::styled("PAUSED", popup_header_style())),
        Line::from(""),
        Line::from(Span::styled(
            "WEAPONS",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for i in 0..config::MAX_WEAPONS {
        if let Some(w) = weapons.get(i) {
            let max_level = w.kind.stats().damage_table.len();
            lines.push(Line::from(Span::styled(
                format!("{}  Lv {}/{}", w.kind.name(), w.level, max_level),
                Style::default().fg(Color::Green),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "--- empty ---",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[SPACE] Restart  [ESC] Quit",
        Style::default().fg(Color::Cyan),
    )));

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, msg_area);
}

fn draw_result_popup(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    title: &str,
    header: Vec<Line>,
    footer: &str,
    popup_height: u16,
) {
    let game = &app.game;
    let elapsed = Settings::format_ticks(game.elapsed_ticks);
    let dark = app.save.dark_mode;
    let body_color = text_color(dark);

    let mut lines = header;
    lines.extend([
        Line::from(Span::styled(
            format!("Time: {}", elapsed),
            Style::default().fg(body_color),
        )),
        Line::from(Span::styled(
            format!("Level: {}", game.level),
            Style::default().fg(body_color),
        )),
        Line::from(Span::styled(
            format!("Kills: {}", game.kill_count),
            Style::default().fg(body_color),
        )),
        Line::from(""),
        Line::from(Span::styled(footer, Style::default().fg(Color::Yellow))),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(border_style(dark)),
        )
        .style(bg(dark))
        .alignment(Alignment::Center);

    let popup_area = popup_rect_centered(area, 36, popup_height);
    fill_bg(frame, popup_area, dark);
    frame.render_widget(paragraph, popup_area);
}

fn draw_game_over(frame: &mut Frame, area: Rect, app: &App) {
    draw_result_popup(
        frame,
        area,
        app,
        " Result ",
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "GAME OVER",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ],
        "[R] Retry  [Q] Quit",
        10,
    );
}

fn draw_cleared(frame: &mut Frame, area: Rect, app: &App) {
    draw_result_popup(
        frame,
        area,
        app,
        " Victory! ",
        vec![
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
        ],
        "[R] Play Again  [Q] Quit",
        12,
    );
}
