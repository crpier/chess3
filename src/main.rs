mod game;
use anyhow::{Context, Result};
use game::{Game, Hex, Kind, Move, Outcome, Piece, Side};
use raylib::prelude::*;
use std::{collections::BTreeMap, ffi::CString, path::PathBuf};

const BG: Color = Color::new(19, 27, 35, 255);
const PANEL: Color = Color::new(29, 40, 50, 255);
const TEXT: Color = Color::new(232, 236, 232, 255);
const MUTED: Color = Color::new(149, 169, 177, 255);
const ACCENT: Color = Color::new(111, 211, 179, 255);
const GOLD: Color = Color::new(240, 191, 105, 255);
const SQRT3: f32 = 1.732_050_8;
// The entire texture rectangle fits inside 90% of a flat-top hex, including shadows.
const PIECE_DIAMETER: f32 = 1.14;

// Keep every label and its measurement on the same readable, embedded font.
struct Canvas<'a, 'b> {
    draw: &'a mut RaylibDrawHandle<'b>,
    font: &'a Font,
}
impl<'a, 'b> std::ops::Deref for Canvas<'a, 'b> {
    type Target = RaylibDrawHandle<'b>;
    fn deref(&self) -> &Self::Target {
        self.draw
    }
}
impl std::ops::DerefMut for Canvas<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.draw
    }
}
impl Canvas<'_, '_> {
    fn draw_text(&mut self, text: &str, x: i32, y: i32, size: i32, color: Color) {
        self.draw.draw_text_ex(
            self.font,
            text,
            Vector2::new(x as f32, y as f32),
            size as f32,
            0.,
            color,
        );
    }
    fn measure_text(&self, text: &str, size: i32) -> i32 {
        self.font.measure_text(text, size as f32, 0.).x.ceil() as i32
    }
}

#[derive(Clone, Copy)]
struct Layout {
    center: Vector2,
    radius: f32,
    panel_x: i32,
    width: i32,
    height: i32,
}
impl Layout {
    fn usable(self) -> bool {
        self.width >= 1000 && self.height >= 640
    }
    fn new(w: i32, h: i32) -> Self {
        Self {
            center: Vector2::new((w - 340) as f32 / 2., h as f32 / 2. + 15.),
            radius: (((w - 380) as f32 / 17.).min((h - 155) as f32 / 19.)).max(8.),
            panel_x: w - 320,
            width: w,
            height: h,
        }
    }
    fn point(self, h: Hex, flip: bool) -> Vector2 {
        let sign = if flip { -1. } else { 1. };
        self.center
            + Vector2::new(1.5 * h.q as f32, SQRT3 * (h.r as f32 + h.q as f32 / 2.))
                * self.radius
                * sign
    }
    fn hex(self, p: Vector2, flip: bool) -> Option<Hex> {
        let sign = if flip { -1. } else { 1. };
        let p = (p - self.center) / self.radius * sign;
        let q = p.x * 2. / 3.;
        let r = -p.x / 3. + p.y / SQRT3;
        let s = -q - r;
        let (mut iq, mut ir, is) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((q - iq).abs(), (r - ir).abs(), (s - is).abs());
        if dq > dr && dq > ds {
            iq = -ir - is;
        } else if dr > ds {
            ir = -iq - is;
        }
        let h = Hex::new(iq as i32, ir as i32);
        h.on_board().then_some(h)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Modal {
    None,
    Help,
    NewGame,
    Mode,
    Resign,
    Promotion(Hex, Hex),
}
struct Animation {
    mv: Move,
    piece: Piece,
    elapsed: f32,
}
struct App {
    game: Game,
    selected: Option<Hex>,
    legal: Vec<Move>,
    ai: bool,
    flip: bool,
    modal: Modal,
    animation: Option<Animation>,
    ai_wait: f32,
    notice: String,
}
impl App {
    fn new() -> Self {
        Self {
            game: Game::default(),
            selected: None,
            legal: vec![],
            ai: true,
            flip: false,
            modal: Modal::None,
            animation: None,
            ai_wait: 0.,
            notice: "Select a white piece to begin.".into(),
        }
    }
    fn clear_selection(&mut self) {
        self.selected = None;
        self.legal.clear();
    }
    fn reset(&mut self) {
        self.game = Game::default();
        self.clear_selection();
        self.animation = None;
        self.modal = Modal::None;
        self.ai_wait = 0.;
        self.notice = "A fresh board. White moves first.".into();
    }
    fn play(&mut self, mv: Move) {
        let piece = self.game.position.pieces[&mv.from];
        if self.game.play(mv) {
            self.animation = Some(Animation {
                mv,
                piece,
                elapsed: 0.,
            });
            self.clear_selection();
            self.ai_wait = 0.;
            self.notice = self.game.moves.last().cloned().unwrap_or_default();
        }
    }
    fn click_hex(&mut self, hex: Option<Hex>) {
        if self.animation.is_some()
            || self.modal != Modal::None
            || self.game.outcome != Outcome::Playing
            || (self.ai && self.game.position.turn == Side::Black)
        {
            return;
        }
        let Some(hex) = hex else {
            self.clear_selection();
            return;
        };
        if let Some(mv) = self.legal.iter().find(|m| m.to == hex).copied() {
            if mv.promotion.is_some() {
                self.modal = Modal::Promotion(mv.from, mv.to);
            } else {
                self.play(mv);
            }
        } else if self
            .game
            .position
            .pieces
            .get(&hex)
            .is_some_and(|p| p.side == self.game.position.turn)
        {
            if self.selected == Some(hex) {
                self.clear_selection();
            } else {
                self.selected = Some(hex);
                self.legal = self.game.position.legal_from(hex);
                self.notice = if self.legal.is_empty() {
                    "That piece has no legal moves.".into()
                } else {
                    format!(
                        "{} at {}: choose a marked tile.",
                        self.game.position.pieces[&hex].kind.name(),
                        hex.label()
                    )
                };
            }
        } else {
            self.notice = "Choose your piece, then a marked destination.".into();
            self.clear_selection();
        }
    }
    fn update(&mut self, dt: f32) {
        if let Some(a) = &mut self.animation {
            a.elapsed += dt;
            if a.elapsed >= 0.28 {
                self.animation = None;
            }
            return;
        }
        if self.modal == Modal::None
            && self.ai
            && self.game.position.turn == Side::Black
            && self.game.outcome == Outcome::Playing
        {
            self.ai_wait += dt;
            if self.ai_wait >= 0.55
                && let Some(mv) = self.game.ai_move()
            {
                self.play(mv);
            }
        }
    }
    fn undo(&mut self) {
        self.animation = None;
        if self.ai
            && self.game.position.turn == Side::White
            && !self.game.history.is_empty()
            && !matches!(self.game.outcome, Outcome::Resigned(_))
        {
            self.game.undo();
        }
        self.game.undo();
        self.clear_selection();
        self.modal = Modal::None;
        self.ai_wait = 0.;
        self.notice = "Move taken back.".into();
    }
}

fn textures(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
) -> Result<BTreeMap<(Side, Kind), Texture2D>> {
    macro_rules! asset {
        ($name:literal) => {
            include_bytes!(concat!("../assets/", $name)).as_slice()
        };
    }
    let sources = [
        (
            Side::White,
            Kind::King,
            asset!("w_king_png_shadow_256px.png"),
        ),
        (
            Side::White,
            Kind::Queen,
            asset!("w_queen_png_shadow_256px.png"),
        ),
        (
            Side::White,
            Kind::Rook,
            asset!("w_rook_png_shadow_256px.png"),
        ),
        (
            Side::White,
            Kind::Knight,
            asset!("w_knight_png_shadow_256px.png"),
        ),
        (
            Side::White,
            Kind::Pawn,
            asset!("w_pawn_png_shadow_256px.png"),
        ),
        (
            Side::Black,
            Kind::King,
            asset!("b_king_png_shadow_256px.png"),
        ),
        (
            Side::Black,
            Kind::Queen,
            asset!("b_queen_png_shadow_256px.png"),
        ),
        (
            Side::Black,
            Kind::Rook,
            asset!("b_rook_png_shadow_256px.png"),
        ),
        (
            Side::Black,
            Kind::Knight,
            asset!("b_knight_png_shadow_256px.png"),
        ),
        (
            Side::Black,
            Kind::Pawn,
            asset!("b_pawn_png_shadow_256px.png"),
        ),
    ];
    sources
        .into_iter()
        .map(|(side, kind, bytes)| {
            let image = Image::load_image_from_mem(".png", bytes).context("Decode piece image")?;
            let texture = rl
                .load_texture_from_image(thread, &image)
                .context("Upload piece image")?;
            Ok(((side, kind), texture))
        })
        .collect()
}
fn draw_piece(
    d: &mut Canvas<'_, '_>,
    textures: &BTreeMap<(Side, Kind), Texture2D>,
    piece: Piece,
    p: Vector2,
    radius: f32,
) {
    if piece.kind == Kind::Bishop {
        let radius = radius * 0.84;
        let fill = if piece.side == Side::White {
            Color::new(250, 244, 226, 255)
        } else {
            Color::new(49, 45, 43, 255)
        };
        let outline = if piece.side == Side::White {
            Color::new(95, 82, 65, 255)
        } else {
            Color::new(207, 184, 150, 255)
        };
        d.draw_ellipse(
            p.x as i32,
            (p.y + radius * 0.52) as i32,
            radius * 0.51,
            radius * 0.14,
            outline,
        );
        d.draw_triangle(
            p + Vector2::new(0., -radius * 0.5),
            p + Vector2::new(-radius * 0.40, radius * 0.48),
            p + Vector2::new(radius * 0.40, radius * 0.48),
            outline,
        );
        d.draw_triangle(
            p + Vector2::new(0., -radius * 0.38),
            p + Vector2::new(-radius * 0.30, radius * 0.40),
            p + Vector2::new(radius * 0.30, radius * 0.40),
            fill,
        );
        d.draw_circle_v(p + Vector2::new(0., -radius * 0.35), radius * 0.25, outline);
        d.draw_circle_v(p + Vector2::new(0., -radius * 0.35), radius * 0.20, fill);
        d.draw_line_ex(
            p + Vector2::new(0., -radius * 0.51),
            p + Vector2::new(radius * 0.12, -radius * 0.31),
            2.,
            outline,
        );
        d.draw_circle_v(p + Vector2::new(0., -radius * 0.65), radius * 0.07, outline);
    } else {
        let texture = &textures[&(piece.side, piece.kind)];
        let scale = radius * PIECE_DIAMETER / texture.width.max(texture.height) as f32;
        d.draw_texture_ex(
            texture,
            p - Vector2::new(texture.width as f32, texture.height as f32) * scale / 2.,
            0.,
            scale,
            Color::WHITE,
        );
    }
}
fn button(
    d: &mut Canvas<'_, '_>,
    rect: Rectangle,
    label: &str,
    mouse: Vector2,
    click: bool,
    enabled: bool,
) -> bool {
    let hover = rect.check_collision_point_rec(mouse);
    d.draw_rectangle_rec(
        rect,
        if enabled && hover {
            Color::new(55, 79, 87, 255)
        } else {
            PANEL
        },
    );
    d.draw_rectangle_lines_ex(
        rect,
        1.,
        if enabled {
            Color::new(72, 97, 107, 255)
        } else {
            Color::new(40, 54, 62, 255)
        },
    );
    let size = 18;
    let width = d.measure_text(label, size);
    d.draw_text(
        label,
        (rect.x + rect.width / 2.) as i32 - width / 2,
        (rect.y + rect.height / 2. - 9.) as i32,
        size,
        if enabled { TEXT } else { MUTED },
    );
    enabled && hover && click
}
fn wrapped(
    d: &mut Canvas<'_, '_>,
    text: &str,
    x: i32,
    mut y: i32,
    width: i32,
    size: i32,
    color: Color,
) {
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.into()
        } else {
            format!("{line} {word}")
        };
        if d.measure_text(&candidate, size) > width && !line.is_empty() {
            d.draw_text(&line, x, y, size, color);
            y += size + 6;
            line = word.into();
        } else {
            line = candidate;
        }
    }
    d.draw_text(&line, x, y, size, color);
}
fn draw(
    app: &mut App,
    d: &mut Canvas<'_, '_>,
    textures: &BTreeMap<(Side, Kind), Texture2D>,
    l: Layout,
    mouse: Vector2,
    click: bool,
) {
    let modal_before = app.modal;
    d.clear_background(BG);
    if !l.usable() {
        d.draw_text("Hex Chess", 24, 24, 30, TEXT);
        wrapped(
            d,
            "Please widen the window to play.",
            24,
            (l.height / 2 - 60).max(90),
            (l.width - 48).max(120),
            26,
            TEXT,
        );
        wrapped(
            d,
            "The board needs at least 1000 x 640 pixels. Your game is paused until there is enough room.",
            24,
            (l.height / 2 + 10).max(170),
            (l.width - 48).max(120),
            18,
            MUTED,
        );
        d.draw_text("Q: quit", 24, l.height - 40, 18, MUTED);
        return;
    }
    d.draw_text("Hex Chess", 30, 24, 30, TEXT);
    d.draw_text("A different kind of chess.", 31, 62, 17, MUTED);
    let hover = if app.modal == Modal::None {
        l.hex(mouse, app.flip)
    } else {
        None
    };
    let checked = app.game.position.in_check(app.game.position.turn);
    for q in -5..=5 {
        for r in -5..=5 {
            let h = Hex::new(q, r);
            if !h.on_board() {
                continue;
            }
            let p = l.point(h, app.flip);
            let palette = [
                Color::new(169, 135, 98, 255),
                Color::new(210, 182, 142, 255),
                Color::new(128, 110, 87, 255),
            ];
            let mut color = palette[(q - r).rem_euclid(3) as usize];
            if app.game.last_move.is_some_and(|m| m.from == h || m.to == h) {
                color = Color::new(150, 166, 112, 255);
            }
            if hover == Some(h) {
                color = color.brightness(0.12);
            }
            if app.selected == Some(h) {
                color = Color::new(76, 151, 149, 255);
            }
            if checked
                && app
                    .game
                    .position
                    .pieces
                    .get(&h)
                    .is_some_and(|p| p.side == app.game.position.turn && p.kind == Kind::King)
            {
                color = Color::new(186, 87, 81, 255);
            }
            d.draw_poly(p, 6, l.radius - 0.7, 0., color);
            if app.legal.iter().any(|m| m.to == h) {
                if app.game.position.pieces.contains_key(&h) {
                    d.draw_poly_lines_ex(p, 6, l.radius - 3., 0., 3., ACCENT);
                } else {
                    d.draw_circle_v(p, l.radius * 0.14, Color::new(36, 89, 78, 255));
                }
            }
            if (q.abs() == 5 || r.abs() == 5 || (q + r).abs() == 5)
                && !app.game.position.pieces.contains_key(&h)
            {
                d.draw_text(
                    &h.label(),
                    (p.x - l.radius * 0.26) as i32,
                    (p.y + l.radius * 0.37) as i32,
                    ((l.radius * 0.26) as i32).max(10),
                    Color::new(64, 57, 48, 255),
                );
            }
        }
    }
    for (&h, &piece) in &app.game.position.pieces {
        if app.animation.as_ref().is_some_and(|a| a.mv.to == h) {
            continue;
        }
        draw_piece(d, textures, piece, l.point(h, app.flip), l.radius);
    }
    if let Some(a) = &app.animation {
        let t = (a.elapsed / 0.28).clamp(0., 1.);
        let t = t * t * (3. - 2. * t);
        let from = l.point(a.mv.from, app.flip);
        let to = l.point(a.mv.to, app.flip);
        draw_piece(d, textures, a.piece, from + (to - from) * t, l.radius);
    }
    let x = l.panel_x;
    d.draw_rectangle(x - 15, 20, 310, l.height - 40, PANEL);
    d.draw_text(
        if app.ai {
            "Solo / simple bot"
        } else {
            "Local / two players"
        },
        x,
        40,
        18,
        ACCENT,
    );
    let status = match &app.game.outcome {
        Outcome::Playing => format!(
            "{} to move{}",
            app.game.position.turn.name(),
            if checked { " / CHECK" } else { "" }
        ),
        Outcome::Checkmate(side) | Outcome::Resigned(side) => format!("{} wins!", side.name()),
        Outcome::Draw(_) => "Draw".into(),
    };
    d.draw_text(&status, x, 80, 26, if checked { GOLD } else { TEXT });
    let sub = match &app.game.outcome {
        Outcome::Checkmate(_) => "Checkmate. Start a new game or undo.",
        Outcome::Resigned(_) => "By resignation. Start fresh or undo.",
        Outcome::Draw(reason) => reason,
        _ if app.ai && app.game.position.turn == Side::Black => "Black is thinking...",
        _ => "Click a piece, then its destination.",
    };
    wrapped(d, sub, x, 116, 275, 17, MUTED);
    d.draw_text("Last action", x, 180, 16, ACCENT);
    wrapped(d, &app.notice, x, 207, 270, 17, TEXT);
    let lost_white = 18
        - app
            .game
            .position
            .pieces
            .values()
            .filter(|p| p.side == Side::White)
            .count()
            .min(18);
    let lost_black = 18
        - app
            .game
            .position
            .pieces
            .values()
            .filter(|p| p.side == Side::Black)
            .count()
            .min(18);
    d.draw_text(
        &format!("Captured: White {lost_white} / Black {lost_black}"),
        x,
        277,
        17,
        MUTED,
    );
    d.draw_text("Move history", x, 323, 16, ACCENT);
    let log_rows = ((l.height - 610) / 27).clamp(2, 9) as usize;
    let start = app.game.moves.len().saturating_sub(log_rows);
    for (i, line) in app.game.moves.iter().enumerate().skip(start) {
        d.draw_text(
            &format!("{:02}  {}", i + 1, line),
            x,
            353 + ((i - start) * 27) as i32,
            15,
            TEXT,
        );
    }
    let y = l.height - 218;
    let enabled = app.modal == Modal::None;
    if button(
        d,
        Rectangle::new(x as f32, y as f32, 130., 40.),
        "Undo [U]",
        mouse,
        click,
        enabled && !app.game.history.is_empty(),
    ) {
        app.undo();
    }
    if button(
        d,
        Rectangle::new((x + 140) as f32, y as f32, 130., 40.),
        "Flip [F]",
        mouse,
        click,
        enabled,
    ) {
        app.flip = !app.flip;
    }
    if button(
        d,
        Rectangle::new(x as f32, (y + 50) as f32, 130., 40.),
        "New [N]",
        mouse,
        click,
        enabled,
    ) {
        app.modal = Modal::NewGame;
    }
    if button(
        d,
        Rectangle::new((x + 140) as f32, (y + 50) as f32, 130., 40.),
        "Mode [M]",
        mouse,
        click,
        enabled,
    ) {
        app.modal = Modal::Mode;
    }
    if button(
        d,
        Rectangle::new(x as f32, (y + 100) as f32, 130., 40.),
        "Rules [H]",
        mouse,
        click,
        enabled,
    ) {
        app.modal = Modal::Help;
    }
    if button(
        d,
        Rectangle::new((x + 140) as f32, (y + 100) as f32, 130., 40.),
        "Resign",
        mouse,
        click,
        enabled && app.game.outcome == Outcome::Playing,
    ) {
        app.modal = Modal::Resign;
    }
    d.draw_text("Esc: cancel / Q: quit", x, l.height - 52, 16, MUTED);
    if let Some(h) = hover {
        let label = app.game.position.pieces.get(&h).map_or(h.label(), |p| {
            format!("{} / {} {}", h.label(), p.side.name(), p.kind.name())
        });
        d.draw_text(&label, 30, l.height - 42, 18, MUTED);
    }
    if app.modal != Modal::None {
        draw_modal(
            app,
            d,
            textures,
            l,
            mouse,
            click && modal_before == app.modal,
        );
    }
}
fn draw_modal(
    app: &mut App,
    d: &mut Canvas<'_, '_>,
    textures: &BTreeMap<(Side, Kind), Texture2D>,
    l: Layout,
    mouse: Vector2,
    click: bool,
) {
    d.draw_rectangle(0, 0, l.width, l.height, Color::new(8, 13, 20, 210));
    let w = 680;
    let h = if app.modal == Modal::Help { 620 } else { 280 };
    let x = (l.width - w) / 2;
    let y = (l.height - h) / 2;
    d.draw_rectangle(x, y, w, h, PANEL);
    d.draw_rectangle_lines(x, y, w, h, ACCENT);
    let title = match app.modal {
        Modal::Help => "The Hex Chess field guide",
        Modal::NewGame => "Start a new game?",
        Modal::Mode => "Switch mode and start fresh?",
        Modal::Resign => "Resign this game?",
        _ => "Choose your promotion",
    };
    d.draw_text(title, x + 28, y + 28, 26, TEXT);
    match app.modal {
        Modal::Help => {
            let lines = [
                (
                    "THE GOAL",
                    "Checkmate the enemy king. You cannot make a move that leaves your own king attacked. White starts at the bottom.",
                ),
                (
                    "ROOK / BISHOP / QUEEN",
                    "Rooks follow the six edge directions. Bishops follow the six vertex diagonals (staying on one tile color). Queens combine both.",
                ),
                (
                    "KING / KNIGHT",
                    "Kings move one step along any rook or bishop direction. Knights jump two edge steps, then one in an adjacent direction.",
                ),
                (
                    "PAWNS",
                    "Advance one tile toward the opposite edge. Capture on the two adjacent forward edge directions. On the far edge, choose a queen, rook, bishop or knight.",
                ),
                (
                    "HOUSE RULES",
                    "A symmetric 18-piece army on 91 tiles. No double pawn move, en passant or castling. This is a house variant, not tournament Glinski chess.",
                ),
                (
                    "DRAWS & CONTROLS",
                    "Stalemate, threefold repetition, 50 moves without a pawn move or capture, and bare kings are draws. U undoes a full turn against the bot. F flips the board.",
                ),
            ];
            for (i, (heading, body)) in lines.iter().enumerate() {
                let yy = y + 82 + i as i32 * 78;
                d.draw_text(heading, x + 28, yy, 16, ACCENT);
                wrapped(d, body, x + 28, yy + 24, w - 56, 17, TEXT);
            }
            if button(
                d,
                Rectangle::new((x + w - 178) as f32, (y + h - 57) as f32, 150., 36.),
                "Back [Esc]",
                mouse,
                click,
                true,
            ) {
                app.modal = Modal::None;
            }
        }
        Modal::Promotion(from, to) => {
            d.draw_text(
                "Pick the piece that replaces your pawn.",
                x + 28,
                y + 77,
                19,
                MUTED,
            );
            for (i, kind) in Kind::PROMOTIONS.into_iter().enumerate() {
                let bx = x + 28 + i as i32 * 160;
                draw_piece(
                    d,
                    textures,
                    Piece {
                        side: app.game.position.turn,
                        kind,
                    },
                    Vector2::new((bx + 70) as f32, (y + 145) as f32),
                    35.,
                );
                if button(
                    d,
                    Rectangle::new(bx as f32, (y + 195) as f32, 145., 45.),
                    kind.name(),
                    mouse,
                    click,
                    true,
                ) {
                    app.modal = Modal::None;
                    app.play(Move {
                        from,
                        to,
                        promotion: Some(kind),
                    });
                }
            }
        }
        modal => {
            wrapped(
                d,
                if modal == Modal::Mode {
                    if app.ai {
                        "Switch to local two-player chess. Both players share the board."
                    } else {
                        "Switch to solo chess. You play White against a simple Black bot."
                    }
                } else if modal == Modal::Resign {
                    "Your opponent will win. You can still undo after resigning."
                } else {
                    "The current board and move history will be cleared."
                },
                x + 28,
                y + 88,
                w - 56,
                20,
                MUTED,
            );
            if button(
                d,
                Rectangle::new((x + 28) as f32, (y + 195) as f32, 220., 45.),
                "Confirm [Enter]",
                mouse,
                click,
                true,
            ) {
                confirm(app);
            }
            if button(
                d,
                Rectangle::new((x + 270) as f32, (y + 195) as f32, 220., 45.),
                "Cancel [Esc]",
                mouse,
                click,
                true,
            ) {
                app.modal = Modal::None;
            }
        }
    }
}
fn confirm(app: &mut App) {
    match app.modal {
        Modal::NewGame => app.reset(),
        Modal::Mode => {
            app.ai = !app.ai;
            app.reset();
        }
        Modal::Resign => {
            let resigned = if app.ai {
                Side::White
            } else {
                app.game.position.turn
            };
            app.game.resign(resigned);
            app.notice = format!("{} resigned.", resigned.name());
            app.clear_selection();
            app.animation = None;
            app.modal = Modal::None;
        }
        _ => {}
    }
}
fn bounded_viewport(requested: (i32, i32), actual: (i32, i32)) -> (i32, i32) {
    (requested.0.min(actual.0), requested.1.min(actual.1))
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|s| s == "--demo");
    let capture = args
        .iter()
        .position(|s| s == "--capture-dir")
        .map(|i| {
            args.get(i + 1)
                .context("--capture-dir needs a path")
                .map(PathBuf::from)
        })
        .transpose()?;
    if let Some(path) = &capture {
        std::fs::create_dir_all(path)?;
    }
    // A logical viewport lets us audit compact layouts without changing desktop settings.
    let audit_size = args
        .iter()
        .position(|s| s == "--audit-size")
        .map(|i| -> Result<(i32, i32)> {
            anyhow::ensure!(demo, "--audit-size requires --demo");
            let size = args.get(i + 1).context("--audit-size needs WIDTHxHEIGHT")?;
            let (w, h) = size.split_once('x').context("Use WIDTHxHEIGHT")?;
            let (w, h): (i32, i32) = (w.parse()?, h.parse()?);
            anyhow::ensure!(w >= 1000 && h >= 740, "Minimum audit size is 1000x740");
            Ok((w, h))
        })
        .transpose()?;
    let (mut rl, thread) = raylib::init()
        .size(1280, 900)
        .title("Hex Chess")
        .resizable()
        .msaa_4x()
        .build();
    rl.set_window_min_size(1000, 640);
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    let textures = textures(&mut rl, &thread)?;
    let font = rl
        .load_font_from_memory(
            &thread,
            ".ttf",
            include_bytes!("../assets/fonts/AdwaitaSans-Regular.ttf"),
            64,
            None,
        )
        .context("Load interface font")?;
    font.texture()
        .set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut app = App::new();
    let mut frame = 0;
    while !rl.window_should_close() {
        let dt = if demo {
            1. / 60.
        } else {
            rl.get_frame_time().min(0.1)
        };
        let actual = (rl.get_screen_width(), rl.get_screen_height());
        let (w, h) = audit_size.map_or(actual, |requested| bounded_viewport(requested, actual));
        let l = Layout::new(w, h);
        let modal_at_frame_start = app.modal;
        let mouse = rl.get_mouse_position();
        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        if rl.is_key_pressed(KeyboardKey::KEY_Q) {
            break;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            app.modal = Modal::None;
            app.clear_selection();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
            confirm(&mut app);
        }
        if app.modal == Modal::None && l.usable() {
            if rl.is_key_pressed(KeyboardKey::KEY_U) {
                app.undo();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_F) {
                app.flip = !app.flip;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_N) {
                app.modal = Modal::NewGame;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_M) {
                app.modal = Modal::Mode;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_H) {
                app.modal = Modal::Help;
            }
            if click && mouse.x < (l.panel_x - 20) as f32 {
                app.click_hex(l.hex(mouse, app.flip));
            }
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                app.clear_selection();
            }
        }
        if demo {
            demo_step(&mut app, frame);
        }
        if l.usable() {
            app.update(dt);
        }
        {
            let draw_click = click && app.modal == modal_at_frame_start;
            let mut d = rl.begin_drawing(&thread);
            draw(
                &mut app,
                &mut Canvas {
                    draw: &mut d,
                    font: &font,
                },
                &textures,
                l,
                mouse,
                draw_click,
            );
        }
        if let Some(path) = &capture
            && frame % 3 == 0
        {
            let filename = CString::new(
                path.join(format!("frame-{:04}.png", frame / 3))
                    .to_string_lossy()
                    .as_bytes(),
            )?;
            unsafe {
                let mut image = raylib::ffi::LoadImageFromScreen();
                if audit_size.is_some() {
                    // ImageFromImage does not bounds-check; ImageCrop does. A compositor
                    // resize can happen between reading the window size and GPU readback.
                    raylib::ffi::ImageCrop(
                        &mut image,
                        Rectangle::new(0., 0., w as f32, h as f32).into(),
                    );
                }
                let saved = raylib::ffi::ExportImage(image, filename.as_ptr());
                raylib::ffi::UnloadImage(image);
                anyhow::ensure!(saved, "Could not save screenshot");
            }
        }
        frame += 1;
        if demo && frame >= 1440 {
            break;
        }
    }
    Ok(())
}
/// Repeatable visual smoke test: exercises the same selection and move entry as clicks.
fn demo_step(app: &mut App, frame: u32) {
    match frame {
        60 => app.click_hex(Some(Hex::new(0, 4))),
        150 => app.click_hex(Some(Hex::new(0, 3))),
        300 => {
            let source = app
                .game
                .position
                .legal_moves()
                .iter()
                .find(|m| app.game.position.pieces[&m.from].kind == Kind::Knight)
                .map(|m| m.from);
            app.click_hex(source);
        }
        390 => {
            if let Some(mv) = app.legal.first().copied() {
                app.click_hex(Some(mv.to));
            }
        }
        540 => {
            app.flip = true;
        }
        630 => {
            app.flip = false;
            app.modal = Modal::Help;
        }
        810 => {
            app.modal = Modal::None;
            app.undo();
        }
        900 => {
            // Explicit rule-test fixture, not a continuation of the opening.
            app.ai = false;
            app.animation = None;
            app.clear_selection();
            app.game = Game::new(game::Position {
                pieces: [
                    (
                        Hex::new(4, 0),
                        Piece {
                            side: Side::White,
                            kind: Kind::King,
                        },
                    ),
                    (
                        Hex::new(0, -5),
                        Piece {
                            side: Side::Black,
                            kind: Kind::King,
                        },
                    ),
                    (
                        Hex::new(-2, -2),
                        Piece {
                            side: Side::White,
                            kind: Kind::Pawn,
                        },
                    ),
                    (
                        Hex::new(-1, -3),
                        Piece {
                            side: Side::Black,
                            kind: Kind::Rook,
                        },
                    ),
                ]
                .into_iter()
                .collect(),
                turn: Side::White,
                quiet_plies: 0,
            });
            app.notice = "Visual test position: pawn promotion.".into();
        }
        960 => app.click_hex(Some(Hex::new(-2, -2))),
        1020 => app.click_hex(Some(Hex::new(-2, -3))),
        1110 => {
            app.modal = Modal::None;
            app.play(Move {
                from: Hex::new(-2, -2),
                to: Hex::new(-2, -3),
                promotion: Some(Kind::Queen),
            });
        }
        1200 => {
            app.animation = None;
            app.clear_selection();
            app.game = Game::new(game::Position {
                pieces: [
                    (
                        Hex::new(0, -5),
                        Piece {
                            side: Side::Black,
                            kind: Kind::King,
                        },
                    ),
                    (
                        Hex::new(0, -2),
                        Piece {
                            side: Side::White,
                            kind: Kind::King,
                        },
                    ),
                    (
                        Hex::new(0, -4),
                        Piece {
                            side: Side::White,
                            kind: Kind::Queen,
                        },
                    ),
                    (
                        Hex::new(-2, -2),
                        Piece {
                            side: Side::White,
                            kind: Kind::Rook,
                        },
                    ),
                ]
                .into_iter()
                .collect(),
                turn: Side::Black,
                quiet_plies: 0,
            });
            app.notice = "Visual test position: checkmate.".into();
        }
        1320 => {
            app.modal = Modal::NewGame;
        }
        1380 => confirm(app),
        _ => {}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_tile_centers_round_trip_both_orientations() {
        for (w, h) in [(1280, 900), (1024, 768), (1600, 1000)] {
            let l = Layout::new(w, h);
            for q in -5..=5 {
                for r in -5..=5 {
                    let hex = Hex::new(q, r);
                    if hex.on_board() {
                        for flip in [false, true] {
                            assert_eq!(l.hex(l.point(hex, flip), flip), Some(hex));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn undersized_windows_pause_instead_of_shrinking_the_board() {
        assert!(!Layout::new(623, 680).usable());
        assert!(!Layout::new(1200, 600).usable());
        assert!(Layout::new(1000, 640).usable());
    }
    #[test]
    fn audit_viewport_tracks_compositor_resize() {
        assert_eq!(bounded_viewport((1000, 740), (1256, 680)), (1000, 680));
        assert_eq!(bounded_viewport((1000, 740), (1256, 1378)), (1000, 740));
        assert_eq!(bounded_viewport((1000, 740), (800, 600)), (800, 600));
    }
    #[test]
    fn piece_texture_rectangle_has_a_ten_percent_hex_inset() {
        // This includes every pixel, not just the visible body of each PNG.
        for (w, h) in [(256., 256.), (200., 300.), (400., 120.)] {
            let scale = PIECE_DIAMETER / f32::max(w, h);
            let (x, y) = (w * scale / 2., h * scale / 2.);
            assert!(y <= SQRT3 / 2. * 0.9);
            assert!(x + y / SQRT3 <= 0.9);
        }
    }
    #[test]
    fn modal_cancel_and_mode_switch_are_non_destructive_until_confirmed() {
        let mut app = App::new();
        app.modal = Modal::Mode;
        assert!(app.ai);
        assert!(app.game.position == game::Position::initial());
        confirm(&mut app);
        assert!(!app.ai);
        assert_eq!(app.modal, Modal::None);
        app.click_hex(Some(Hex::new(0, 4)));
        app.click_hex(Some(Hex::new(0, 3)));
        for _ in 0..100 {
            app.update(1. / 60.);
        }
        assert_eq!(app.game.moves.len(), 1);
        assert_eq!(app.game.position.turn, Side::Black);
        app.modal = Modal::NewGame;
        confirm(&mut app);
        assert!(app.game.position == game::Position::initial());
    }
    #[test]
    fn selection_move_bot_and_undo() {
        let mut app = App::new();
        app.click_hex(Some(Hex::new(0, -4)));
        assert!(app.selected.is_none());
        app.click_hex(Some(Hex::new(0, 4)));
        assert!(!app.legal.is_empty());
        app.click_hex(Some(Hex::new(0, 3)));
        assert_eq!(app.game.position.turn, Side::Black);
        for _ in 0..100 {
            app.update(1. / 60.);
        }
        assert_eq!(app.game.position.turn, Side::White);
        assert_eq!(app.game.moves.len(), 2);
        app.undo();
        assert!(app.game.position == game::Position::initial());
    }
}
