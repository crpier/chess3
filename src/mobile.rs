//! Touch presentation and controls; all orders still go through the shared App/Game.
use super::*;

impl Layout {
    #[cfg(any(target_os = "emscripten", test))]
    pub(super) fn for_touch(w: i32, h: i32) -> Self {
        let landscape = w >= 500 && w > h;
        let board = if landscape {
            Rectangle::new(8., 8., (w - 264).max(1) as f32, (h - 16).max(1) as f32)
        } else {
            Rectangle::new(8., 64., (w - 16).max(1) as f32, (h - 176).max(1) as f32)
        };
        Self {
            center: Vector2::new(board.x + board.width / 2., board.y + board.height / 2.),
            // Radius-five flat-top board: 17 radii wide and 11*sqrt(3) high.
            radius: (board.width / 17.).min(board.height / (11. * SQRT3)),
            panel_x: if landscape { w - 248 } else { 8 },
            width: w,
            height: h,
            touch: true,
            board,
        }
    }
}

fn landscape(l: Layout) -> bool {
    l.width >= 500 && l.width > l.height
}

pub(super) fn draw_too_small(d: &mut Canvas<'_, '_>, l: Layout) {
    wrapped(d, "More room needed", 16, 24, l.width - 32, 24, TEXT);
    wrapped(
        d,
        "Your game is paused. Turn the phone upright or try fullscreen to make room for the board.",
        16,
        90,
        l.width - 32,
        17,
        MUTED,
    );
}

// Bound text to its allotted rows, including long feedback. Rules show full text on pages.
fn label(
    d: &mut Canvas<'_, '_>,
    text: &str,
    position: (i32, i32),
    width: i32,
    size: i32,
    rows: usize,
    tint: Color,
) {
    let (x, y) = position;
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        let last = lines.last_mut().unwrap();
        let candidate = if last.is_empty() {
            word.to_owned()
        } else {
            format!("{last} {word}")
        };
        if d.measure_text(&candidate, size) <= width || last.is_empty() {
            *last = candidate;
        } else {
            lines.push(word.to_owned());
        }
    }
    if lines.len() > rows {
        lines.truncate(rows);
        let last = lines.last_mut().unwrap();
        while !last.is_empty() && d.measure_text(&format!("{last}..."), size) > width {
            last.pop();
        }
        last.push_str("...");
    }
    for (i, line) in lines.iter().enumerate() {
        d.draw_text(line, x, y + i as i32 * (size + 3), size, tint);
    }
}

pub(super) fn draw_panel(
    app: &mut App,
    d: &mut Canvas<'_, '_>,
    l: Layout,
    mouse: Vector2,
    click: bool,
    danger: [bool; 2],
) {
    let wide = landscape(l);
    let x = if wide { l.panel_x } else { 8 };
    let width = if wide { 232 } else { l.width - 16 };
    if wide {
        d.draw_rectangle(x - 4, 0, width + 8, l.height, PANEL);
    }
    let status = match &app.game.outcome {
        Outcome::Playing if app.frozen() => app.pause_label().unwrap().to_owned(),
        Outcome::Playing if app.game.now() < game::START_DELAY => format!(
            "Starts in {:.1}s",
            (game::START_DELAY - app.game.now()) as f32 / 1000.
        ),
        Outcome::Playing => format!(
            "Live / {:02}:{:02}",
            app.game.now() / 60000,
            (app.game.now() / 1000) % 60
        ),
        Outcome::KingCaptured(side) | Outcome::Resigned(side) => format!("{} wins!", side.name()),
        Outcome::Draw(_) => "Draw".into(),
    };
    d.draw_text(&status, x, 5, 20, TEXT);
    for (i, side) in [Side::White, Side::Black].into_iter().enumerate() {
        let state = if app.game.active(side) >= game::MOVE_LIMIT {
            "full"
        } else if app.can_start(side) {
            "ready"
        } else {
            "wait"
        };
        d.draw_text(
            &format!("{}: {}/2 {state}", side.name(), app.game.active(side)),
            x + if wide { 0 } else { i as i32 * width / 2 },
            31 + if wide { i as i32 * 24 } else { 0 },
            16,
            if app.can_start(side) { ACCENT } else { GOLD },
        );
    }
    if danger[0] || danger[1] {
        d.draw_text(
            match danger {
                [true, true] => "Threat: both kings",
                [true, false] => "Threat: White king",
                _ => "Threat: Black king",
            },
            x,
            if wide { 79 } else { 49 },
            13,
            GOLD,
        );
    }
    let id = if app.touch_side == Side::White {
        app.selected
    } else {
        app.black_selected
    };
    let hint = id.and_then(|id| app.game.unit(id)).map_or_else(
        || format!("Tap a {} piece, then a destination", app.touch_side.name()),
        |u| {
            format!(
                "{} {}{} ({})",
                u.piece.side.name(),
                u.piece.kind.name(),
                u.promoting_to()
                    .map_or(String::new(), |kind| format!("/{}", kind.name())),
                if u.marching() {
                    "committed"
                } else if app.can_start(u.piece.side) {
                    "ready"
                } else {
                    "waiting"
                }
            )
        },
    );
    let hint_y = if wide { 100 } else { l.height - 108 };
    label(d, &hint, (x, hint_y), width, 14, 1, ACCENT);
    label(
        d,
        &app.notice,
        (x, hint_y + 20),
        width,
        13,
        if wide { 4 } else { 2 },
        TEXT,
    );
    if wide && l.height >= 360 {
        d.draw_text(&app.availability(app.touch_side), x, 200, 14, MUTED);
        if let Some(line) = app.game.moves.last() {
            label(d, line, (x, 224), width, 13, 2, MUTED);
        }
    }
    let enabled = app.modal == Modal::None;
    let button_width = (width - 12) as f32 / 3.;
    let rect = |i| {
        Rectangle::new(
            x as f32 + i as f32 * (button_width + 6.),
            (l.height - 56) as f32,
            button_width,
            48.,
        )
    };
    if button(
        d,
        rect(0),
        if app.paused { "Resume" } else { "Pause" },
        mouse,
        click,
        enabled && app.game.outcome == Outcome::Playing,
    ) {
        app.paused = !app.paused;
    }
    let quick = if app.ai {
        format!("CPU: {}", app.cpu_level.name())
    } else {
        format!("Touch: {}", app.touch_side.name())
    };
    if button(d, rect(1), &quick, mouse, click, enabled) {
        if app.ai {
            app.cycle_cpu();
        } else {
            app.touch_side = if app.touch_side == Side::White {
                Side::Black
            } else {
                Side::White
            };
            app.inspected = app.touch_side;
            app.clear_selection();
            app.feedback(format!(
                "Touch now controls {}. Existing orders still finish.",
                app.touch_side.name()
            ));
        }
    }
    if button(d, rect(2), "Menu", mouse, click, enabled) {
        app.modal = Modal::Controls;
    }
}

const RULES: [(&str, &str); 6] = [
    (
        "Win by capturing the king",
        "Both sides begin together after a 3-second countdown. Pieces travel continuously. Capture the enemy king to win; simultaneous king captures draw. Check is a warning, not a forced response.",
    ),
    (
        "Two committed moves per side",
        "Only two of your pieces may travel at once. A slot becomes free when its piece finishes or is captured. Tap a piece to see every legal destination. Mint means ready; muted previews mean wait.",
    ),
    (
        "No cancellation or queues",
        "Choose carefully: a travelling piece cannot be redirected or stopped. Repeated taps never add a queue. Keep a slot free for defense. There is no cooldown after arrival.",
    ),
    (
        "Capture and interception",
        "Captures complete at landing. Crossing enemy ground moves can capture each other. Knights jump over traffic. Ordinary steps take 1.2 seconds; king steps take 0.8 seconds; knight jumps take 1.2 seconds.",
    ),
    (
        "Movement and promotion",
        "Each piece has its own hex movement. Friendly travel lanes are reserved. Pawns promote at the far rim. In Menu, tap Promote to cycle the next promotion choice. Already committed promotions do not change.",
    ),
    (
        "Touch, local play and pause",
        "Tap a piece, then a marked destination. Solo controls White. In local mode, Touch: White / Black selects which side you control; it does not take a turn or cancel orders. Pause, menus and backgrounding freeze the game. Dragging or multi-touch never submits a move.",
    ),
];

pub(super) fn draw_modal(
    app: &mut App,
    d: &mut Canvas<'_, '_>,
    l: Layout,
    mouse: Vector2,
    click: bool,
) {
    d.draw_rectangle(0, 0, l.width, l.height, Color::new(8, 13, 20, 235));
    let x = 12;
    let y = 12;
    let w = l.width - 24;
    let h = l.height - 24;
    d.draw_rectangle(x, y, w, h, PANEL);
    let title = match app.modal {
        Modal::Controls => "Game controls".to_owned(),
        Modal::Help => format!("Rules / {} of 6", app.help_page + 1),
        Modal::Movement => [
            Kind::King,
            Kind::Bishop,
            Kind::Knight,
            Kind::Rook,
            Kind::Queen,
            Kind::Pawn,
        ][app.help_page % 6]
            .name()
            .to_owned(),
        Modal::NewGame => "Start a new game?".to_owned(),
        Modal::Mode => "Switch mode?".to_owned(),
        Modal::Resign(side) => format!("Resign {}?", side.name()),
        Modal::None => String::new(),
    };
    d.draw_text(&title, x + 12, y + 12, 24, TEXT);
    match app.modal {
        Modal::Controls => {
            let bw = (w - 36) as f32 / 3.;
            let preset = app.promotion[usize::from(app.touch_side == Side::Black)];
            let promotion = format!(
                "Promote: {}",
                match preset {
                    Kind::Queen => "Q",
                    Kind::Rook => "R",
                    Kind::Bishop => "B",
                    Kind::Knight => "N",
                    _ => "?",
                }
            );
            for (i, text) in [
                "Flip", "Clear", "Back", "New", "Mode", "Rules", "Moves", &promotion, "Resign",
            ]
            .into_iter()
            .enumerate()
            {
                let rect = Rectangle::new(
                    (x + 12) as f32 + (i % 3) as f32 * (bw + 6.),
                    (y + 56 + (i / 3) as i32 * 56) as f32,
                    bw,
                    48.,
                );
                if button(
                    d,
                    rect,
                    text,
                    mouse,
                    click,
                    i != 8 || app.game.outcome == Outcome::Playing,
                ) {
                    match i {
                        0 => app.flip = !app.flip,
                        1 => {
                            app.clear_selection();
                            app.feedback("Selection cleared. Existing orders still finish.");
                        }
                        2 => app.modal = Modal::None,
                        3 => app.modal = Modal::NewGame,
                        4 => app.modal = Modal::Mode,
                        5 => {
                            app.modal = Modal::Help;
                            app.help_page = 0;
                        }
                        6 => {
                            app.modal = Modal::Movement;
                            app.help_page = 0;
                        }
                        7 => {
                            let slot = usize::from(app.touch_side == Side::Black);
                            let next = (Kind::PROMOTIONS
                                .iter()
                                .position(|kind| *kind == preset)
                                .unwrap()
                                + 1)
                                % 4;
                            app.promotion[slot] = Kind::PROMOTIONS[next];
                            app.game.set_promotion(app.touch_side, app.promotion[slot]);
                            app.feedback(format!(
                                "{} promotion preset: {}",
                                app.touch_side.name(),
                                app.promotion[slot].name()
                            ));
                        }
                        8 => app.modal = Modal::Resign(app.touch_side),
                        _ => unreachable!(),
                    }
                }
            }
            label(
                d,
                &format!(
                    "{} next promotion: {}",
                    app.touch_side.name(),
                    app.promotion[usize::from(app.touch_side == Side::Black)].name()
                ),
                (x + 12, y + 230),
                w - 24,
                15,
                1,
                ACCENT,
            );
            label(
                d,
                &app.notice,
                (x + 12, y + 254),
                w - 24,
                14,
                ((h - 262) / 17).max(1) as usize,
                TEXT,
            );
        }
        Modal::Help | Modal::Movement => {
            if app.modal == Modal::Help {
                let (heading, body) = RULES[app.help_page % 6];
                wrapped(d, heading, x + 12, y + 54, w - 24, 19, ACCENT);
                wrapped(d, body, x + 12, y + 110, w - 24, 17, TEXT);
            } else {
                let kind = [
                    Kind::King,
                    Kind::Bishop,
                    Kind::Knight,
                    Kind::Rook,
                    Kind::Queen,
                    Kind::Pawn,
                ][app.help_page % 6];
                label(d, kind.movement(), (x + 12, y + 50), w - 24, 15, 2, TEXT);
                let position = game::Position {
                    pieces: [(
                        Hex::new(0, 0),
                        Piece {
                            side: Side::White,
                            kind,
                        },
                    )]
                    .into_iter()
                    .collect(),
                };
                let legal = position.legal_from(Hex::new(0, 0));
                let radius = ((h - 150) as f32 / 13.)
                    .min((w - 36) as f32 / 11.)
                    .clamp(3., 24.);
                let center = Vector2::new(l.width as f32 / 2., y as f32 + (h + 20) as f32 / 2.);
                for q in -3..=3 {
                    for r in -3..=3 {
                        let tile = Hex::new(q, r);
                        if tile.distance(Hex::new(0, 0)) > 3 {
                            continue;
                        }
                        let point = center
                            + Vector2::new(1.5 * q as f32, SQRT3 * (r as f32 + q as f32 / 2.))
                                * radius;
                        let tint = if tile == Hex::new(0, 0) {
                            GOLD
                        } else if legal.iter().any(|mv| mv.to == tile) {
                            ACCENT
                        } else {
                            Color::new(53, 68, 78, 255)
                        };
                        d.draw_poly(point, 6, radius - 0.7, 0., tint);
                    }
                }
            }
            let bw = (w - 42) as f32 / 4.;
            for (i, text) in [
                "Prev",
                "Next",
                if app.modal == Modal::Help {
                    "Moves"
                } else {
                    "Rules"
                },
                "Back",
            ]
            .into_iter()
            .enumerate()
            {
                if button(
                    d,
                    Rectangle::new(
                        (x + 12) as f32 + i as f32 * (bw + 6.),
                        (y + h - 56) as f32,
                        bw,
                        44.,
                    ),
                    text,
                    mouse,
                    click,
                    true,
                ) {
                    match i {
                        0 => app.help_page = (app.help_page + 5) % 6,
                        1 => app.help_page = (app.help_page + 1) % 6,
                        2 => {
                            app.modal = if app.modal == Modal::Help {
                                Modal::Movement
                            } else {
                                Modal::Help
                            };
                            app.help_page = 0;
                        }
                        3 => app.modal = Modal::Controls,
                        _ => unreachable!(),
                    }
                }
            }
        }
        modal => {
            let body = match modal {
                Modal::Mode if app.ai => {
                    "Switch to local two-player chess. Both players share the board; the Touch button chooses which side to control. This starts a fresh game."
                }
                Modal::Mode => {
                    "Switch to solo chess. You play White against the Black CPU. This starts a fresh game."
                }
                Modal::Resign(_) => "Your opponent will win. Committed orders will be cleared.",
                _ => "The current board and move history will be cleared.",
            };
            wrapped(d, body, x + 12, y + 72, w - 24, 18, MUTED);
            let bw = (w - 30) as f32 / 2.;
            if button(
                d,
                Rectangle::new((x + 12) as f32, (y + h - 60) as f32, bw, 48.),
                "Confirm",
                mouse,
                click,
                true,
            ) {
                confirm(app);
            }
            if button(
                d,
                Rectangle::new((x + 18) as f32 + bw, (y + h - 60) as f32, bw, 48.),
                "Cancel",
                mouse,
                click,
                true,
            ) {
                app.modal = Modal::Controls;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn touch_board_and_centers_fit_phone_and_tablet_in_both_orientations() {
        for (w, h) in [(390, 788), (320, 512), (844, 334), (768, 968), (1024, 712)] {
            let l = Layout::for_touch(w, h);
            assert!(l.usable(), "{w}x{h}");
            for flip in [false, true] {
                for q in -5..=5 {
                    for r in -5..=5 {
                        let tile = Hex::new(q, r);
                        if !tile.on_board() {
                            continue;
                        }
                        let p = l.point(tile, flip);
                        assert!(
                            p.x - l.radius >= l.board.x - 0.01
                                && p.x + l.radius <= l.board.x + l.board.width + 0.01
                        );
                        assert!(
                            p.y - l.radius * SQRT3 / 2. >= l.board.y - 0.01
                                && p.y + l.radius * SQRT3 / 2. <= l.board.y + l.board.height + 0.01
                        );
                        assert_eq!(l.hex(p, flip), Some(tile));
                    }
                }
            }
        }
    }
    #[test]
    fn touch_black_orders_use_same_commitments_and_slots_without_turns() {
        let mut app = App::new();
        app.ai = false;
        app.game.advance(game::START_DELAY);
        let id = app
            .game
            .units()
            .iter()
            .find(|u| u.piece.side == Side::Black && u.tile() == Hex::new(0, -4))
            .unwrap()
            .id;
        app.click_pointer_for(Side::Black, Some(id), Some(Hex::new(0, -4)));
        assert_eq!(app.black_selected, Some(id));
        app.click_pointer_for(Side::Black, None, Some(Hex::new(0, -3)));
        assert_eq!(app.game.active(Side::Black), 1);
        app.click_pointer_for(Side::Black, None, Some(Hex::new(1, -3)));
        assert_eq!(app.game.active(Side::Black), 1);
        assert!(app.notice.contains("already committed"));
    }
    #[test]
    fn touch_both_sides_reject_third_moves_and_never_queue_spam() {
        let mut app = App::new();
        app.ai = false;
        app.game.advance(game::START_DELAY);
        for (side, sign) in [(Side::White, 1), (Side::Black, -1)] {
            for (q, r) in [(0, 4), (-1, 4), (1, 3)] {
                let from = Hex::new(q * sign, r * sign);
                let to = Hex::new(q * sign, (r - 1) * sign);
                let id = app
                    .game
                    .units()
                    .iter()
                    .find(|u| u.piece.side == side && u.tile() == from)
                    .unwrap()
                    .id;
                app.click_pointer_for(side, Some(id), Some(from));
                app.click_pointer_for(side, None, Some(to));
            }
            assert_eq!(app.game.active(side), 2);
            assert!(app.notice.contains("both slots were occupied"));
            for _ in 0..100 {
                app.click_pointer_for(side, None, Some(Hex::new(sign, 2 * sign)));
            }
            assert_eq!(app.game.active(side), 2);
            assert!(app.notice.contains("Nothing was queued"));
        }
        assert_eq!(app.game.orders().len(), 4);
        app.update(1.3);
        assert!(app.game.orders().is_empty());
        app.update(5.);
        assert!(app.game.orders().is_empty());
    }

    #[test]
    fn touch_cannot_control_cpu_or_issue_orders_while_paused() {
        let mut app = App::new();
        app.game.advance(game::START_DELAY);
        let black = app
            .game
            .units()
            .iter()
            .find(|u| u.tile() == Hex::new(0, -4))
            .unwrap()
            .id;
        app.click_pointer_for(Side::Black, Some(black), Some(Hex::new(0, -4)));
        assert!(app.black_selected.is_none());
        app.ai = false;
        app.paused = true;
        app.click_pointer_for(Side::Black, Some(black), Some(Hex::new(0, -4)));
        app.click_pointer_for(Side::Black, None, Some(Hex::new(0, -3)));
        assert!(app.black_selected.is_none());
        assert!(app.game.orders().is_empty());
    }
}
