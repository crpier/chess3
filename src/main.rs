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
    fn fractional(self, q: f32, r: f32, flip: bool) -> Vector2 {
        self.center
            + Vector2::new(1.5 * q, SQRT3 * (r + q / 2.))
                * self.radius
                * if flip { -1. } else { 1. }
    }
    fn pick(self, game: &Game, p: Vector2, flip: bool) -> Option<Hex> {
        game.units()
            .iter()
            .filter_map(|u| {
                let (q, r) = u.axial();
                let distance = (self.fractional(q, r, flip) - p).length();
                (distance < self.radius * 0.6).then_some((distance, u.anchor))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, h)| h)
            .or_else(|| self.hex(p, flip))
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
    Movement,
    NewGame,
    Mode,
    Resign(Side),
}
struct App {
    game: Game,
    selected: Option<Hex>,
    legal: Vec<Move>,
    black_selected: Option<Hex>,
    black_legal: Vec<Move>,
    black_cursor: Hex,
    inspected: Side,
    promotion: [Kind; 2],
    ai: bool,
    flip: bool,
    modal: Modal,
    paused: bool,
    focus_paused: bool,
    ai_wait: f32,
    fraction_ms: f64,
    notice: String,
}
impl App {
    fn new() -> Self {
        Self {
            game: Game::default(),
            selected: None,
            legal: vec![],
            black_selected: None,
            black_legal: vec![],
            black_cursor: Hex::new(0, -4),
            inspected: Side::White,
            promotion: [Kind::Queen; 2],
            ai: true,
            flip: false,
            modal: Modal::None,
            paused: false,
            focus_paused: false,
            ai_wait: 0.,
            fraction_ms: 0.,
            notice: "Both armies start together after 3 seconds.".into(),
        }
    }
    fn frozen(&self) -> bool {
        self.paused || self.focus_paused || self.modal != Modal::None
    }
    fn clear_selection(&mut self) {
        self.selected = None;
        self.legal.clear();
        self.black_selected = None;
        self.black_legal.clear();
    }
    fn reset(&mut self) {
        let (ai, flip) = (self.ai, self.flip);
        *self = Self::new();
        self.ai = ai;
        self.flip = flip;
    }
    fn play(&mut self, mv: Move) {
        match self.game.submit(mv) {
            Ok(()) => {
                let side = self.game.position.pieces[&mv.from].side;
                self.notice = format!(
                    "{} ordered {} toward {}.",
                    side.name(),
                    self.game.position.pieces[&mv.from].kind.name(),
                    mv.to.label()
                );
                if side == Side::White {
                    self.selected = None;
                    self.legal.clear();
                } else {
                    self.black_selected = None;
                    self.black_legal.clear();
                }
            }
            Err(reason) => self.notice = reason.message().into(),
        }
    }
    fn click_hex(&mut self, hex: Option<Hex>) {
        self.click_side(Side::White, hex);
    }
    fn click_side(&mut self, side: Side, hex: Option<Hex>) {
        if self.frozen()
            || self.game.outcome != Outcome::Playing
            || (side == Side::Black && self.ai)
        {
            return;
        }
        let selected = if side == Side::White {
            self.selected
        } else {
            self.black_selected
        };
        let Some(hex) = hex else {
            if side == Side::White {
                self.selected = None;
                self.legal.clear();
            } else {
                self.black_selected = None;
                self.black_legal.clear();
            }
            return;
        };
        // Revalidate against the live board, not yesterday's highlighted destinations.
        if let Some(from) = selected.filter(|h| {
            self.game
                .position
                .pieces
                .get(h)
                .is_some_and(|p| p.side == side)
        }) {
            let promotion = self.promotion[if side == Side::White { 0 } else { 1 }];
            if let Some(mut mv) = self
                .game
                .legal_from(from)
                .into_iter()
                .find(|m| m.to == hex && m.promotion.is_none_or(|k| k == promotion))
            {
                if self.game.position.pieces[&from].kind == Kind::Pawn {
                    mv.promotion = Some(promotion);
                }
                self.play(mv);
                return;
            }
        }
        if self
            .game
            .position
            .pieces
            .get(&hex)
            .is_some_and(|p| p.side == side)
        {
            let next = (selected != Some(hex)).then_some(hex);
            self.inspected = side;
            if side == Side::White {
                self.selected = next;
            } else {
                self.black_selected = next;
            }

            self.notice = format!(
                "{} {} at {}: {}",
                side.name(),
                self.game.position.pieces[&hex].kind.name(),
                hex.label(),
                "choose a destination, even far away."
            );
            self.refresh_selection();
        } else {
            self.notice =
                "Select your piece, then a destination. Rejected clicks are never queued.".into();
        }
    }
    fn refresh_selection(&mut self) {
        if self.selected.is_some_and(|h| {
            !self
                .game
                .position
                .pieces
                .get(&h)
                .is_some_and(|p| p.side == Side::White)
        }) {
            self.selected = None;
        }
        if self.black_selected.is_some_and(|h| {
            !self
                .game
                .position
                .pieces
                .get(&h)
                .is_some_and(|p| p.side == Side::Black)
        }) {
            self.black_selected = None;
        }
        self.legal = self.selected.map_or(vec![], |h| self.game.legal_from(h));
        self.black_legal = self
            .black_selected
            .map_or(vec![], |h| self.game.legal_from(h));
    }
    fn update(&mut self, dt: f32) {
        if self.frozen() {
            return;
        }
        self.fraction_ms += dt as f64 * 1000.;
        let ms = self.fraction_ms.floor() as u64;
        self.fraction_ms -= ms as f64;
        let white = self
            .selected
            .and_then(|h| self.game.unit_at(h))
            .map(|u| u.id);
        let black = self
            .black_selected
            .and_then(|h| self.game.unit_at(h))
            .map(|u| u.id);
        let previous = self.game.moves.len();
        self.game.advance(ms);
        self.selected = white.and_then(|id| self.game.hex_of(id));
        self.black_selected = black.and_then(|id| self.game.hex_of(id));
        if self.game.moves.len() != previous {
            self.notice = self.game.moves.last().cloned().unwrap_or_default();
        }
        self.refresh_selection();
        if self.ai && self.game.outcome == Outcome::Playing {
            self.ai_wait += dt;
            if self.ai_wait >= 0.35 {
                self.ai_wait = 0.;
                self.game.attack_army(Side::Black);
            }
        }
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
    d.draw_text("March. Engage. Reinforce.", 31, 62, 17, MUTED);
    let hover = if app.modal == Modal::None {
        l.hex(mouse, app.flip)
    } else {
        None
    };
    let white_ready = !app.frozen() && app.game.now() >= game::START_DELAY;
    let black_ready = white_ready;
    let checked = app
        .game
        .units()
        .iter()
        .any(|u| u.piece.kind == Kind::King && u.health < u.piece.kind.stats().health * 0.5);
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
            if app.black_selected == Some(h) {
                color = Color::new(57, 100, 146, 255);
            }
            if app.game.unit_at(h).is_some_and(|u| {
                u.piece.kind == Kind::King && u.health < u.piece.kind.stats().health * 0.5
            }) {
                color = Color::new(186, 87, 81, 255);
            }
            d.draw_poly(p, 6, l.radius - 0.7, 0., color);
            let white_target = app.legal.iter().any(|m| m.to == h);
            let black_target = app.black_legal.iter().any(|m| m.to == h);
            if white_target || black_target {
                let tint = if white_target {
                    if white_ready { ACCENT } else { MUTED }
                } else if black_ready {
                    Color::new(134, 181, 240, 255)
                } else {
                    MUTED
                };
                if app.game.position.pieces.contains_key(&h) {
                    d.draw_poly_lines_ex(p, 6, l.radius - 3., 0., 3., tint);
                } else {
                    d.draw_circle_v(p, l.radius * 0.14, tint);
                }
                if white_target && black_target {
                    d.draw_circle_lines(
                        p.x as i32,
                        p.y as i32,
                        l.radius * 0.23,
                        if black_ready {
                            Color::new(134, 181, 240, 255)
                        } else {
                            MUTED
                        },
                    );
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
    // Physical positions are canonical. Grid anchors are only routing waypoints.
    for order in app.game.orders() {
        if let Some(u) = app.game.unit_at(order.mv.from) {
            let (q, r) = u.axial();
            let from = l.fractional(q, r, app.flip);
            let to = l.point(order.mv.to, app.flip);
            let tint = if order.piece.side == Side::White {
                GOLD
            } else {
                Color::new(134, 181, 240, 255)
            };
            let selected = app.selected == Some(u.anchor) || app.black_selected == Some(u.anchor);
            if selected {
                d.draw_line_ex(from, to, 1., tint.alpha(0.65));
            }
            d.draw_poly_lines_ex(
                to,
                6,
                l.radius * 0.18,
                0.,
                1.,
                tint.alpha(if selected { 1. } else { 0.35 }),
            );
        }
    }
    for u in app.game.units() {
        let (q, r) = u.axial();
        let p = l.fractional(q, r, app.flip);
        for (id, healing) in [(u.target, false), (u.healing, true)] {
            if let Some(v) = id.and_then(|id| app.game.units().iter().find(|v| v.id == id)) {
                let (q, r) = v.axial();
                let to = l.fractional(q, r, app.flip);
                let tint = if healing {
                    ACCENT
                } else if u.piece.side == Side::White {
                    GOLD
                } else {
                    Color::new(134, 181, 240, 255)
                };
                d.draw_line_ex(p, to, if healing { 1. } else { 2. }, tint.alpha(0.65));
            }
        }
    }
    for u in app.game.units() {
        let (q, r) = u.axial();
        let p = l.fractional(q, r, app.flip);
        if app.selected == Some(u.anchor) || app.black_selected == Some(u.anchor) {
            d.draw_circle_lines(
                p.x as i32,
                p.y as i32,
                l.radius * 0.65,
                if u.piece.side == Side::White {
                    ACCENT
                } else {
                    Color::new(134, 181, 240, 255)
                },
            );
        }
        draw_piece(d, textures, u.piece, p, l.radius);
        let width = l.radius * 0.8;
        let y = p.y + l.radius * 0.66;
        d.draw_rectangle((p.x - width / 2.) as i32, y as i32, width as i32, 4, PANEL);
        d.draw_rectangle(
            (p.x - width / 2.) as i32,
            y as i32,
            (width * (u.health / u.piece.kind.stats().health) as f32) as i32,
            4,
            if u.health / u.piece.kind.stats().health < 0.3 {
                Color::new(237, 107, 101, 255)
            } else {
                ACCENT
            },
        );
    }
    if !app.ai {
        d.draw_poly_lines_ex(
            l.point(app.black_cursor, app.flip),
            6,
            l.radius - 2.,
            0.,
            2.,
            Color::new(134, 181, 240, 255),
        );
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
        Outcome::Playing if app.frozen() => "Paused".into(),
        Outcome::Playing if app.game.now() < game::START_DELAY => format!(
            "Starts in {:.1}s",
            (game::START_DELAY - app.game.now()) as f32 / 1000.
        ),
        Outcome::Playing => format!(
            "Live / {:02}:{:02}",
            app.game.now() / 60000,
            (app.game.now() / 1000) % 60
        ),
        Outcome::KingDefeated(side) | Outcome::Resigned(side) => format!("{} wins!", side.name()),
        Outcome::Draw(_) => "Draw".into(),
    };
    d.draw_text(&status, x, 80, 26, if checked { GOLD } else { TEXT });
    if app.game.outcome != Outcome::Playing {
        wrapped(
            d,
            match &app.game.outcome {
                Outcome::KingDefeated(_) => "King defeated. Start a new game to play again.",
                Outcome::Resigned(_) => "By resignation. Start a new game to play again.",
                Outcome::Draw(reason) => reason,
                _ => "",
            },
            x,
            116,
            275,
            17,
            MUTED,
        );
    } else {
        for (i, side) in [Side::White, Side::Black].into_iter().enumerate() {
            let marching = app
                .game
                .units()
                .iter()
                .filter(|u| u.piece.side == side && u.marching())
                .count();
            let fighting = app
                .game
                .units()
                .iter()
                .filter(|u| u.piece.side == side && u.target.is_some())
                .count();
            let state = format!("{marching} marching / {fighting} fighting");
            d.draw_text(
                &format!("{}: {}", side.name(), state),
                x,
                116 + i as i32 * 25,
                17,
                ACCENT,
            );
        }
    }
    let inspected = if app.inspected == Side::White {
        app.selected.or(app.black_selected)
    } else {
        app.black_selected.or(app.selected)
    }
    .and_then(|h| app.game.position.pieces.get(&h).map(|p| (h, *p)));
    d.draw_text(
        if inspected.is_some() {
            "Selected unit"
        } else {
            "Last action"
        },
        x,
        180,
        16,
        ACCENT,
    );
    let hint = inspected.and_then(|(h, p)| {
        app.game.unit_at(h).map(|u| {
            format!(
                "{} {:.0}/{:.0} HP. {}",
                p.kind.name(),
                u.health,
                p.kind.stats().health,
                p.kind.movement()
            )
        })
    });
    wrapped(
        d,
        hint.as_deref().unwrap_or(&app.notice),
        x,
        207,
        270,
        17,
        TEXT,
    );
    let lost_white = app.game.captured_count(Side::White);
    let lost_black = app.game.captured_count(Side::Black);
    d.draw_text(
        &format!("Lost: White {lost_white} / Black {lost_black}"),
        x,
        277,
        17,
        MUTED,
    );
    d.draw_text("Battle log", x, 323, 16, ACCENT);
    let log_rows = ((l.height - 610) / 27).clamp(2, 9) as usize;
    let start = app.game.moves.len().saturating_sub(log_rows);
    for (i, line) in app.game.moves.iter().enumerate().skip(start) {
        let mut label = line.clone();
        while d.measure_text(&label, 15) > 270 {
            label.pop();
        }
        if label.len() < line.len() {
            for _ in 0..3 {
                label.pop();
            }
            label.push_str("...");
        }
        d.draw_text(&label, x, 353 + ((i - start) * 27) as i32, 15, TEXT);
    }
    let y = l.height - 218;
    let enabled = app.modal == Modal::None;
    if button(
        d,
        Rectangle::new(x as f32, y as f32, 130., 40.),
        if app.paused {
            "Resume [P]"
        } else {
            "Pause [P]"
        },
        mouse,
        click,
        enabled && app.game.outcome == Outcome::Playing,
    ) {
        app.paused = !app.paused;
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
        app.modal = Modal::Resign(Side::White);
    }
    d.draw_text(
        &if app.ai {
            format!("Promote: {} [1-4]", app.promotion[0].name())
        } else {
            let symbol = |kind| match kind {
                Kind::Queen => "Q",
                Kind::Rook => "R",
                Kind::Bishop => "B",
                Kind::Knight => "N",
                _ => "?",
            };
            format!(
                "Promotion W:{} / B:{} [1-4]",
                symbol(app.promotion[0]),
                symbol(app.promotion[1])
            )
        },
        x,
        l.height - 70,
        15,
        MUTED,
    );
    d.draw_text(
        if app.ai {
            "A: advance / S: stop / Q: quit"
        } else {
            "Black: arrows + , . / Space"
        },
        x,
        l.height - 48,
        15,
        MUTED,
    );
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
fn draw_movement_guide(d: &mut Canvas<'_, '_>, x: i32, y: i32) {
    d.draw_text(
        "Green = attack range. Blue = healing. Bodies block ranged fire.",
        x + 28,
        y + 65,
        15,
        MUTED,
    );
    let cards = [
        (
            Kind::King,
            "Commander / 180 HP",
            "Protect him to stay alive",
        ),
        (
            Kind::Bishop,
            "Heal 4 HP/s within 2 tiles",
            "Weak melee / mobile support",
        ),
        (
            Kind::Knight,
            "Fast march / melee fighter",
            "Vaults over friendly traffic",
        ),
        (
            Kind::Rook,
            "22 damage/s / range 3.2",
            "Slow / needs clear sightline",
        ),
        (
            Kind::Queen,
            "24 damage/s / range 2.6",
            "Mobile ranged support",
        ),
        (
            Kind::Pawn,
            "18 damage/s / close range",
            "Forward march / frontline",
        ),
    ];
    for (i, (kind, line1, line2)) in cards.into_iter().enumerate() {
        let cx = x + 28 + (i % 3) as i32 * 208;
        let cy = y + 96 + (i / 3) as i32 * 232;
        d.draw_rectangle(cx, cy, 198, 224, Color::new(35, 47, 58, 255));
        d.draw_text(kind.name(), cx + 12, cy + 10, 19, TEXT);
        let center = Vector2::new((cx + 99) as f32, (cy + 104) as f32);
        for q in -3..=3 {
            for r in -3..=3 {
                let h = Hex::new(q, r);
                if h.distance(Hex::new(0, 0)) > 3 {
                    continue;
                }
                let p = center
                    + Vector2::new(1.5 * q as f32, SQRT3 * (r as f32 + q as f32 / 2.)) * 10.5;
                let tint = if h == Hex::new(0, 0) {
                    GOLD
                } else if ((q * q + q * r + r * r) as f64).sqrt() <= kind.stats().range {
                    ACCENT
                } else {
                    Color::new(53, 68, 78, 255)
                };
                d.draw_poly(p, 6, 9.8, 0., tint);
            }
        }
        if kind == Kind::Bishop {
            for q in -2..=2 {
                for r in -2..=2 {
                    if q == 0 && r == 0 {
                        continue;
                    }
                    if ((q * q + q * r + r * r) as f64).sqrt() > 2.1 {
                        continue;
                    }
                    let p = center
                        + Vector2::new(1.5 * q as f32, SQRT3 * (r as f32 + q as f32 / 2.)) * 10.5;
                    d.draw_poly_lines_ex(p, 6, 9.8, 0., 1., Color::new(134, 181, 240, 255));
                }
            }
        }
        d.draw_circle_v(center, 3.5, PANEL);
        d.draw_text(line1, cx + 10, cy + 183, 14, TEXT);
        d.draw_text(line2, cx + 10, cy + 203, 14, MUTED);
    }
}
fn draw_modal(
    app: &mut App,
    d: &mut Canvas<'_, '_>,
    _textures: &BTreeMap<(Side, Kind), Texture2D>,
    l: Layout,
    mouse: Vector2,
    click: bool,
) {
    d.draw_rectangle(0, 0, l.width, l.height, Color::new(8, 13, 20, 210));
    let w = 680;
    let h = if matches!(app.modal, Modal::Help | Modal::Movement) {
        620
    } else {
        280
    };
    let x = (l.width - w) / 2;
    let y = (l.height - h) / 2;
    d.draw_rectangle(x, y, w, h, PANEL);
    d.draw_rectangle_lines(x, y, w, h, ACCENT);
    let title = match app.modal {
        Modal::Help => "The Hex Chess field guide",
        Modal::NewGame => "Start a new game?",
        Modal::Mode => "Switch mode and start fresh?",
        Modal::Resign(Side::White) => "Resign White?",
        Modal::Resign(Side::Black) => "Resign Black?",
        Modal::Movement => "Army roles and combat reach",
        _ => "Choose your promotion",
    };
    d.draw_text(title, x + 28, y + 28, 26, TEXT);
    match app.modal {
        Modal::Help | Modal::Movement => {
            if app.modal == Modal::Movement {
                draw_movement_guide(d, x, y);
            } else {
                let lines = [
                    (
                        "A BATTLE, NOT TIMED TURNS",
                        "All pieces move and fight together. Reduce the enemy king to zero health. Orders set destinations, not individual attacks.",
                    ),
                    (
                        "PERSISTENT ORDERS / A TO ADVANCE",
                        "Click a piece and any reachable tile. It keeps marching there. A sends the whole army to seek enemies; Shift+A for Black in local play.",
                    ),
                    (
                        "AUTOMATIC COMBAT",
                        "Nearby enemies fight continuously, even when you give no orders. Rooks and queens need clear sightlines. Bishops heal one nearby ally.",
                    ),
                    (
                        "MOVEMENT / TRAFFIC",
                        "Routes follow touching tiles. Pawns can only advance into their front fan. Knights vault friendly traffic. Other pieces walk around bodies.",
                    ),
                    (
                        "RETREAT / NO CLICK-SPAM BONUS",
                        "Retarget at any time; velocity turns gradually. S stops a selected unit (Shift+S Black). Clicks never reset momentum, health or damage.",
                    ),
                    (
                        "LOCAL PLAY / PAUSE / PROMOTION",
                        "White mouse; Black arrows and Space. P, menus and focus loss pause. 1-4 preset promotion at the far rim (Shift for Black). Q quits.",
                    ),
                ];
                for (i, (heading, body)) in lines.iter().enumerate() {
                    let yy = y + 82 + i as i32 * 78;
                    d.draw_text(heading, x + 28, yy, 16, ACCENT);
                    wrapped(d, body, x + 28, yy + 24, w - 56, 17, TEXT);
                }
            }
            if button(
                d,
                Rectangle::new((x + 28) as f32, (y + h - 57) as f32, 180., 36.),
                if app.modal == Modal::Movement {
                    "Battle rules"
                } else {
                    "Army guide"
                },
                mouse,
                click,
                true,
            ) {
                app.modal = if app.modal == Modal::Movement {
                    Modal::Help
                } else {
                    Modal::Movement
                };
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
        modal => {
            wrapped(
                d,
                if modal == Modal::Mode {
                    if app.ai {
                        "Switch to local two-player chess. Both players share the board."
                    } else {
                        "Switch to solo chess. You play White against a simple Black bot."
                    }
                } else if matches!(modal, Modal::Resign(_)) {
                    "Your opponent will win. Committed orders will be cleared."
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
        Modal::Resign(resigned) => {
            app.game.resign(resigned);
            app.notice = format!("{} resigned.", resigned.name());
            app.clear_selection();
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
    let capture_on_demand = args.iter().any(|s| s == "--capture-on-demand");
    let capture = args
        .iter()
        .position(|s| s == "--capture-dir")
        .map(|i| {
            args.get(i + 1)
                .context("--capture-dir needs a path")
                .map(PathBuf::from)
        })
        .transpose()?;
    anyhow::ensure!(
        !capture_on_demand || capture.is_some(),
        "--capture-on-demand requires --capture-dir"
    );
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
    let mut undersized_frames = 0;
    while !rl.window_should_close() {
        let dt = if demo {
            1. / 60.
        } else {
            rl.get_frame_time().min(0.1)
        };
        let actual = (rl.get_screen_width(), rl.get_screen_height());
        let (w, h) = audit_size.map_or(actual, |requested| bounded_viewport(requested, actual));
        let l = Layout::new(w, h);
        app.focus_paused = !demo && !rl.is_window_focused();
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
            if rl.is_key_pressed(KeyboardKey::KEY_P) {
                app.paused = !app.paused;
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
            if rl.is_key_pressed(KeyboardKey::KEY_R) {
                let black = !app.ai
                    && (rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                        || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT));
                app.modal = Modal::Resign(if black { Side::Black } else { Side::White });
            }
            for (key, kind) in [
                KeyboardKey::KEY_ONE,
                KeyboardKey::KEY_TWO,
                KeyboardKey::KEY_THREE,
                KeyboardKey::KEY_FOUR,
            ]
            .into_iter()
            .zip(Kind::PROMOTIONS)
            {
                if rl.is_key_pressed(key) {
                    let black = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                        || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
                    app.promotion[usize::from(black)] = kind;
                    app.game
                        .set_promotion(if black { Side::Black } else { Side::White }, kind);
                    app.notice = format!(
                        "{} promotion preset: {}",
                        if black { "Black" } else { "White" },
                        kind.name()
                    );
                }
            }
            if !app.ai && !app.frozen() {
                for (key, delta) in [
                    (KeyboardKey::KEY_LEFT, Hex::new(-1, 0)),
                    (KeyboardKey::KEY_RIGHT, Hex::new(1, 0)),
                    (KeyboardKey::KEY_UP, Hex::new(0, -1)),
                    (KeyboardKey::KEY_DOWN, Hex::new(0, 1)),
                    (KeyboardKey::KEY_COMMA, Hex::new(-1, 1)),
                    (KeyboardKey::KEY_PERIOD, Hex::new(1, -1)),
                ] {
                    if rl.is_key_pressed(key) || rl.is_key_pressed_repeat(key) {
                        let delta = if app.flip { delta.scale(-1) } else { delta };
                        let next = app.black_cursor.add(delta);
                        if next.on_board() {
                            app.black_cursor = next;
                        }
                    }
                }
                if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
                    app.click_side(Side::Black, Some(app.black_cursor));
                }
                if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                    app.black_selected = None;
                    app.black_legal.clear();
                }
            }
            if click && mouse.x < (l.panel_x - 20) as f32 {
                app.click_hex(l.pick(&app.game, mouse, app.flip));
            }
            let black = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
            if rl.is_key_pressed(KeyboardKey::KEY_A) && (!black || !app.ai) {
                app.game
                    .attack_army(if black { Side::Black } else { Side::White });
            }
            if rl.is_key_pressed(KeyboardKey::KEY_S)
                && let Some(h) = if black {
                    app.black_selected
                } else {
                    app.selected
                }
            {
                app.game.stop(h);
            }
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                app.selected = None;
                app.legal.clear();
            }
        }
        if demo && l.usable() {
            demo_step(&mut app, frame);
        }
        if demo && !l.usable() {
            undersized_frames += 1;
            anyhow::ensure!(
                undersized_frames < 300,
                "Demo needs a window at least 1000x640. Widen the window and retry."
            );
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
            && ((!capture_on_demand && frame % 3 == 0)
                || (capture_on_demand && rl.is_key_pressed(KeyboardKey::KEY_F10)))
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
        if !demo || l.usable() {
            frame += 1;
        }
        if demo && frame >= 1800 {
            break;
        }
    }
    Ok(())
}
/// Deterministic rendered audit, including clearly labelled artificial rule fixtures.
fn demo_step(app: &mut App, frame: u32) {
    match frame {
        200 => app.click_hex(Some(Hex::new(0, 4))),
        230 => app.click_hex(Some(Hex::new(0, 3))),
        240 => app.game.attack_army(Side::White),
        400 => {
            let source = app
                .game
                .position
                .pieces
                .iter()
                .find(|(_, p)| p.side == Side::White && p.kind == Kind::Knight)
                .map(|(&h, _)| h);
            app.click_hex(source);
        }
        430 => {
            if let Some(m) = app.legal.first().copied() {
                app.click_hex(Some(m.to));
            }
        }
        540 => app.flip = true,
        630 => {
            app.flip = false;
            app.modal = Modal::Help;
        }
        720 => app.modal = Modal::Movement,
        810 => {
            app.modal = Modal::None;
            app.paused = true;
        }
        870 => app.paused = false,
        900 => {
            app.ai = false;
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
            });
            app.game.advance(game::START_DELAY);
            app.notice = "Visual fixture: live promotion; no blocking dialog.".into();
        }
        960 => app.click_hex(Some(Hex::new(-2, -2))),
        1000 => app.click_hex(Some(Hex::new(-2, -3))),
        1140 => {
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
            });
            app.game.advance(game::START_DELAY);
            app.notice = "Visual fixture: continuous damage defeats the king.".into();
            app.game.attack_army(Side::White);
        }
        1160 => app.click_hex(Some(Hex::new(0, -4))),
        1180 => app.click_hex(Some(Hex::new(0, -5))),
        1560 => app.modal = Modal::NewGame,
        1620 => confirm(app),
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
    fn local_orders_and_pause_do_not_serialize_armies() {
        let mut app = App::new();
        app.ai = false;
        app.game.advance(game::START_DELAY);
        app.click_hex(Some(Hex::new(0, 4)));
        app.click_hex(Some(Hex::new(0, 0)));
        app.click_hex(Some(Hex::new(-1, 4)));
        app.click_hex(Some(Hex::new(-1, 0)));
        app.click_side(Side::Black, Some(Hex::new(0, -4)));
        app.click_side(Side::Black, Some(Hex::new(0, 0)));
        assert_eq!(app.game.orders().len(), 3);
        let before = app
            .game
            .units()
            .iter()
            .map(|u| (u.id, u.x, u.y, u.health))
            .collect::<Vec<_>>();
        app.modal = Modal::Help;
        app.update(10.);
        assert_eq!(
            before,
            app.game
                .units()
                .iter()
                .map(|u| (u.id, u.x, u.y, u.health))
                .collect::<Vec<_>>()
        );
        app.modal = Modal::None;
        app.update(0.2);
        assert_ne!(
            before,
            app.game
                .units()
                .iter()
                .map(|u| (u.id, u.x, u.y, u.health))
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn moving_pieces_can_be_picked_and_selection_tracks_identity() {
        let mut app = App::new();
        app.ai = false;
        app.game.advance(game::START_DELAY);
        let h = Hex::new(0, 4);
        let id = app.game.unit_at(h).unwrap().id;
        app.game
            .submit(Move {
                from: h,
                to: Hex::new(0, 0),
                promotion: None,
            })
            .unwrap();
        app.selected = Some(h);
        app.update(1.8);
        let anchor = app.game.hex_of(id).unwrap();
        assert_eq!(app.selected, Some(anchor));
        assert_ne!(anchor, h);
        let u = app.game.unit_at(anchor).unwrap();
        let (q, r) = u.axial();
        let l = Layout::new(1280, 900);
        for flip in [false, true] {
            assert_eq!(
                l.pick(&app.game, l.fractional(q, r, flip), flip),
                Some(anchor)
            );
        }
    }
    #[test]
    fn focus_pause_and_bot_army_advance() {
        let mut app = App::new();
        app.focus_paused = true;
        app.update(10.);
        assert_eq!(app.game.now(), 0);
        app.focus_paused = false;
        app.update(3.1);
        app.update(0.4);
        assert!(app.game.orders().len() > 1);
    }
}
