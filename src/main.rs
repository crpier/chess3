use std::{collections::HashMap, ops};

use anyhow::Result;
use anyhow::anyhow;
use raylib::RaylibHandle;
use raylib::RaylibThread;
use raylib::texture::Texture2D;
use raylib::{color::Color, drawing::RaylibDrawHandle, math::Vector2, prelude::RaylibDraw};

const SCREEN_WIDTH: i32 = 1440;
const SCREEN_HEIGHT: i32 = 900;

const SQRT_3: f32 = 1.73205080757;
const TILE_RADIUS: f32 = 45.;
const CENTER: Vector2 = Vector2::new(SCREEN_WIDTH as f32 / 2., SCREEN_HEIGHT as f32 / 2.);
const MAP_RADIUS: i32 = 5;
const TEXTURE_SCALE: f32 = 0.25;
const ANIMATION_DURATION: f32 = 0.3; // seconds

// Try to not have Clone+Copy
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct CubeCoord {
    q: i32,
    r: i32,
    s: i32,
}

fn round_hex(q: f32, r: f32, s: f32) -> CubeCoord {
    let mut round_q = q.round() as i32;
    let mut round_r = r.round() as i32;
    let mut round_s = s.round() as i32;

    let q_diff = (q - round_q as f32).abs();
    let r_diff = (r - round_r as f32).abs();
    let s_diff = (s - round_s as f32).abs();

    if q_diff > r_diff || q_diff > s_diff {
        round_q = -round_r - round_s;
    } else if r_diff > s_diff {
        round_r = -round_q - round_s;
    } else {
        round_s = -round_q - round_r;
    }

    CubeCoord {
        q: round_q,
        r: round_r,
        s: round_s,
    }
}

impl CubeCoord {
    fn from_vector(point: Vector2) -> Self {
        let x = ((point - CENTER) / TILE_RADIUS).x;
        let y = ((point - CENTER) / TILE_RADIUS).y;
        let q = 2. / 3. * x;
        let r = -1. / 3. * x + SQRT_3 / 3. * y;
        let s = -q - r;
        round_hex(q, r, s)
    }

    fn to_vector(&self) -> Vector2 {
        let x = 3. / 2. * self.q as f32;
        let y = SQRT_3 / 2. * self.q as f32 + SQRT_3 * self.r as f32;
        Vector2::new(x, y) * TILE_RADIUS + CENTER
    }

    fn neighbors(&self) -> [CubeCoord; 6] {
        let directions = [
            CubeCoord { q: 1, r: -1, s: 0 },
            CubeCoord { q: 1, r: 0, s: -1 },
            CubeCoord { q: 0, r: 1, s: -1 },
            CubeCoord { q: -1, r: 1, s: 0 },
            CubeCoord { q: -1, r: 0, s: 1 },
            CubeCoord { q: 0, r: -1, s: 1 },
        ];
        [
            *self + directions[0],
            *self + directions[1],
            *self + directions[2],
            *self + directions[3],
            *self + directions[4],
            *self + directions[5],
        ]
    }

    fn knight_moves(&self) -> [CubeCoord; 12] {
        // Knight moves: 2 steps in one direction, then 1 step perpendicular (60 degrees)
        // For each of the 6 main directions, move 2 steps, then 1 step in either perpendicular direction
        let directions = [
            CubeCoord { q: 1, r: -1, s: 0 },
            CubeCoord { q: 1, r: 0, s: -1 },
            CubeCoord { q: 0, r: 1, s: -1 },
            CubeCoord { q: -1, r: 1, s: 0 },
            CubeCoord { q: -1, r: 0, s: 1 },
            CubeCoord { q: 0, r: -1, s: 1 },
        ];

        [
            // Direction 0 (q: 1, r: -1, s: 0) * 2 + perpendiculars
            *self + directions[0] * 2 + directions[1],
            *self + directions[0] * 2 + directions[5],
            // Direction 1 (q: 1, r: 0, s: -1) * 2 + perpendiculars
            *self + directions[1] * 2 + directions[0],
            *self + directions[1] * 2 + directions[2],
            // Direction 2 (q: 0, r: 1, s: -1) * 2 + perpendiculars
            *self + directions[2] * 2 + directions[1],
            *self + directions[2] * 2 + directions[3],
            // Direction 3 (q: -1, r: 1, s: 0) * 2 + perpendiculars
            *self + directions[3] * 2 + directions[2],
            *self + directions[3] * 2 + directions[4],
            // Direction 4 (q: -1, r: 0, s: 1) * 2 + perpendiculars
            *self + directions[4] * 2 + directions[3],
            *self + directions[4] * 2 + directions[5],
            // Direction 5 (q: 0, r: -1, s: 1) * 2 + perpendiculars
            *self + directions[5] * 2 + directions[0],
            *self + directions[5] * 2 + directions[4],
        ]
    }
}

impl ops::Mul<i32> for CubeCoord {
    type Output = CubeCoord;
    fn mul(self, rhs: i32) -> CubeCoord {
        CubeCoord {
            q: self.q * rhs,
            r: self.r * rhs,
            s: self.s * rhs,
        }
    }
}

impl ops::Add for CubeCoord {
    type Output = CubeCoord;
    fn add(self, rhs: CubeCoord) -> CubeCoord {
        CubeCoord {
            q: self.q + rhs.q,
            r: self.r + rhs.r,
            s: self.s + rhs.s,
        }
    }
}

struct Hexagon {
    coord: CubeCoord,
    color: Color,
    highlighted: bool,
    valid_move: bool,
    selected: bool,
}

impl Hexagon {
    fn draw(&self, d: &mut RaylibDrawHandle) {
        let color = if self.selected {
            // Blue tint for selected piece
            Color::new(100, 150, 255, 255)
        } else if self.valid_move {
            // Green tint blended with base color for valid moves
            Color {
                r: self.color.r.saturating_add(30),
                g: self.color.g.saturating_add(80),
                b: self.color.b.saturating_add(30),
                a: self.color.a,
            }
        } else if self.highlighted {
            // Brighten for hover
            Color {
                r: self.color.r.saturating_add(15),
                g: self.color.g.saturating_add(15),
                b: self.color.b.saturating_add(15),
                a: self.color.a,
            }
        } else {
            self.color
        };
        d.draw_poly(self.coord.to_vector(), 6, TILE_RADIUS, 0.0, color);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PieceColor {
    Black,
    White,
}

enum PieceKind {
    BlackKing,
    BlackKnight,
    BlackRook,
    BlackPawn,
    BlackQueen,
    WhiteKing,
    WhiteKnight,
    WhiteRook,
    WhitePawn,
    WhiteQueen,
}

impl PieceKind {
    fn color(&self) -> PieceColor {
        match self {
            PieceKind::BlackKing | PieceKind::BlackKnight | PieceKind::BlackRook
            | PieceKind::BlackPawn | PieceKind::BlackQueen => PieceColor::Black,
            PieceKind::WhiteKing | PieceKind::WhiteKnight | PieceKind::WhiteRook
            | PieceKind::WhitePawn | PieceKind::WhiteQueen => PieceColor::White,
        }
    }
}

struct Piece {
    kind: PieceKind,
    coord: CubeCoord,
    texture: Texture2D,
}

impl Piece {
    fn draw(&self, d: &mut RaylibDrawHandle, position_override: Option<Vector2>) {
        let texture_vec = Vector2::new(
            self.texture.width as f32 * TEXTURE_SCALE / 2.,
            self.texture.width as f32 * TEXTURE_SCALE / 2.,
        );
        let position = position_override.unwrap_or_else(|| self.coord.to_vector());
        d.draw_texture_ex(
            &self.texture,
            position - texture_vec,
            0.,
            TEXTURE_SCALE,
            Color::WHITE,
        );
    }
}

struct Animation {
    from: CubeCoord,
    to: CubeCoord,
    progress: f32, // 0.0 to 1.0
}

struct Map {
    hexes: HashMap<CubeCoord, Hexagon>,
    // TODO: try keeping a reference to a hexagon instead
    highlighted_location: Option<CubeCoord>,
    selected_location: Option<CubeCoord>,
    pieces: HashMap<CubeCoord, Piece>,
    animation: Option<Animation>,
}

impl Map {
    fn new(map_radius: i32, rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut hexes = HashMap::new();
        let hex_colors = [
            Color::new(233, 173, 112, 255),
            Color::new(210, 140, 69, 255),
            Color::new(255, 207, 159, 255),
        ];

        for q in -map_radius..=map_radius {
            for r in -map_radius..=map_radius {
                if q + r < -map_radius || q + r > map_radius {
                    continue;
                }
                let new_coord = CubeCoord { q, r, s: -q - r };
                hexes.insert(
                    new_coord,
                    Hexagon {
                        coord: new_coord,
                        color: hex_colors[(((q + map_radius) as usize * 2
                            + (r + map_radius) as usize)
                            % hex_colors.len()) as usize],
                        // TODO: create `new` method
                        highlighted: false,
                        valid_move: false,
                        selected: false,
                    },
                );
            }
        }

        let mut pieces = HashMap::new();

        let king = Piece {
            kind: PieceKind::BlackKing,
            coord: CubeCoord { q: 0, r: 0, s: 0 },
            texture: rl.load_texture(&thread, "./assets/b_king_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 0, r: 0, s: 0 }, king);

        let black_knight = Piece {
            kind: PieceKind::BlackKnight,
            coord: CubeCoord { q: 2, r: -1, s: -1 },
            texture: rl.load_texture(&thread, "./assets/b_knight_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 2, r: -1, s: -1 }, black_knight);

        let white_knight = Piece {
            kind: PieceKind::WhiteKnight,
            coord: CubeCoord { q: -2, r: 1, s: 1 },
            texture: rl.load_texture(&thread, "./assets/w_knight_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: -2, r: 1, s: 1 }, white_knight);

        let white_king = Piece {
            kind: PieceKind::WhiteKing,
            coord: CubeCoord { q: -3, r: 0, s: 3 },
            texture: rl.load_texture(&thread, "./assets/w_king_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: -3, r: 0, s: 3 }, white_king);

        let black_rook = Piece {
            kind: PieceKind::BlackRook,
            coord: CubeCoord { q: 0, r: -3, s: 3 },
            texture: rl.load_texture(&thread, "./assets/b_rook_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 0, r: -3, s: 3 }, black_rook);

        let white_rook = Piece {
            kind: PieceKind::WhiteRook,
            coord: CubeCoord { q: 0, r: 3, s: -3 },
            texture: rl.load_texture(&thread, "./assets/w_rook_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 0, r: 3, s: -3 }, white_rook);

        let black_pawn = Piece {
            kind: PieceKind::BlackPawn,
            coord: CubeCoord { q: 1, r: -2, s: 1 },
            texture: rl.load_texture(&thread, "./assets/b_pawn_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 1, r: -2, s: 1 }, black_pawn);

        let white_pawn = Piece {
            kind: PieceKind::WhitePawn,
            coord: CubeCoord { q: -1, r: 2, s: -1 },
            texture: rl.load_texture(&thread, "./assets/w_pawn_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: -1, r: 2, s: -1 }, white_pawn);

        let black_queen = Piece {
            kind: PieceKind::BlackQueen,
            coord: CubeCoord { q: 2, r: 0, s: -2 },
            texture: rl.load_texture(&thread, "./assets/b_queen_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: 2, r: 0, s: -2 }, black_queen);

        let white_queen = Piece {
            kind: PieceKind::WhiteQueen,
            coord: CubeCoord { q: -2, r: 0, s: 2 },
            texture: rl.load_texture(&thread, "./assets/w_queen_png_shadow_256px.png").unwrap(),
        };
        pieces.insert(CubeCoord { q: -2, r: 0, s: 2 }, white_queen);

        Map {
            hexes,
            pieces,
            highlighted_location: None,
            selected_location: None,
            animation: None,
        }
    }

    fn draw(&self, d: &mut RaylibDrawHandle) {
        for hex in self.hexes.values() {
            hex.draw(d);
        }

        // Draw pieces with animation interpolation
        for piece in self.pieces.values() {
            let position_override = if let Some(ref anim) = self.animation {
                if piece.coord == anim.from {
                    // This piece is animating - interpolate its position
                    let from_vec = anim.from.to_vector();
                    let to_vec = anim.to.to_vector();
                    let interpolated = Vector2::new(
                        from_vec.x + (to_vec.x - from_vec.x) * anim.progress,
                        from_vec.y + (to_vec.y - from_vec.y) * anim.progress,
                    );
                    Some(interpolated)
                } else {
                    None
                }
            } else {
                None
            };
            piece.draw(d, position_override);
        }
    }

    fn frame_reset(&mut self) {
        if let Some(highlighted) = self.highlighted_location {
            let hex = self.hexes.get_mut(&highlighted).unwrap();
            hex.highlighted = false;
        };
        // Clear valid_move and selected flags
        for hex in self.hexes.values_mut() {
            hex.valid_move = false;
            hex.selected = false;
        }
        // Re-apply selected state and valid moves if a piece is selected
        if let Some(selected_coord) = self.selected_location {
            if let Some(hex) = self.hexes.get_mut(&selected_coord) {
                hex.selected = true;
            }
            // Re-highlight valid moves
            self.highlight_valid_moves(selected_coord);
        }
    }

    fn highlight_hex_under_cursor(&mut self, d: &RaylibDrawHandle) -> Result<()> {
        let coord = CubeCoord::from_vector(d.get_mouse_position());
        let hex = self
            .hexes
            .get_mut(&coord)
            .ok_or_else(|| anyhow!("Highlighted coord out of bounds"))?;
        hex.highlighted = true;
        self.highlighted_location = Some(coord.clone());
        Ok(())
    }

    fn handle_click(&mut self, d: &RaylibDrawHandle) {
        // Block interactions during animation
        if self.is_animating() {
            return;
        }

        let coord = CubeCoord::from_vector(d.get_mouse_position());

        // Check if clicking on a hex within the map
        if !self.hexes.contains_key(&coord) {
            return;
        }

        // Check if we have a piece selected and clicked on a valid move
        if let Some(selected_coord) = self.selected_location {
            if let Some(hex) = self.hexes.get(&coord) {
                if hex.valid_move {
                    // Move the piece to the clicked tile
                    self.move_piece(selected_coord, coord);
                    self.selected_location = None;
                    return;
                }
            }
        }

        // Check if there's a piece at this location
        if self.pieces.contains_key(&coord) {
            // Select this piece
            self.selected_location = Some(coord);
            self.highlight_valid_moves(coord);
        } else {
            // Clicked on empty hex - deselect
            self.selected_location = None;
        }
    }

    fn get_rook_moves(&self, from: CubeCoord, piece_color: PieceColor) -> Vec<CubeCoord> {
        let mut moves = Vec::new();

        // Rook moves along 6 directions (3 axes, 2 directions each)
        let directions = [
            CubeCoord { q: 1, r: -1, s: 0 },
            CubeCoord { q: -1, r: 1, s: 0 },
            CubeCoord { q: 1, r: 0, s: -1 },
            CubeCoord { q: -1, r: 0, s: 1 },
            CubeCoord { q: 0, r: 1, s: -1 },
            CubeCoord { q: 0, r: -1, s: 1 },
        ];

        // For each direction, move until hitting an obstacle or edge
        for dir in directions.iter() {
            let mut current = from + *dir;
            loop {
                // Check if this position is on the map
                if !self.hexes.contains_key(&current) {
                    break;
                }

                // Check if there's a piece here
                if let Some(piece) = self.pieces.get(&current) {
                    // If different color, can capture (add and stop)
                    if piece.kind.color() != piece_color {
                        moves.push(current);
                    }
                    // Either way, can't move further in this direction
                    break;
                }

                // Empty square - can move here
                moves.push(current);
                current = current + *dir;
            }
        }

        moves
    }

    fn get_queen_moves(&self, from: CubeCoord, piece_color: PieceColor) -> Vec<CubeCoord> {
        // In hexagonal chess, queens move exactly like rooks (along all 6 axes)
        self.get_rook_moves(from, piece_color)
    }

    fn get_pawn_moves(&self, from: CubeCoord, piece_color: PieceColor) -> Vec<CubeCoord> {
        let mut moves = Vec::new();

        // Define forward and capture directions based on color
        let (forward_dir, capture_dirs) = match piece_color {
            PieceColor::White => (
                CubeCoord { q: 0, r: 1, s: -1 },  // Direction 2
                [
                    CubeCoord { q: 1, r: 0, s: -1 },   // Direction 1
                    CubeCoord { q: -1, r: 1, s: 0 },   // Direction 3
                ]
            ),
            PieceColor::Black => (
                CubeCoord { q: 0, r: -1, s: 1 },  // Direction 5
                [
                    CubeCoord { q: -1, r: 0, s: 1 },   // Direction 4
                    CubeCoord { q: 1, r: -1, s: 0 },   // Direction 0
                ]
            ),
        };

        // Forward move: only if square is empty
        let forward_pos = from + forward_dir;
        if self.hexes.contains_key(&forward_pos) && !self.pieces.contains_key(&forward_pos) {
            moves.push(forward_pos);
        }

        // Capture moves: only if enemy piece present
        for capture_dir in capture_dirs.iter() {
            let capture_pos = from + *capture_dir;
            if let Some(piece) = self.pieces.get(&capture_pos) {
                // Can only capture enemy pieces
                if piece.kind.color() != piece_color {
                    moves.push(capture_pos);
                }
            }
        }

        moves
    }

    fn highlight_valid_moves(&mut self, piece_coord: CubeCoord) {
        // Get the piece to determine its movement pattern and color
        let (piece_kind, piece_color) = match self.pieces.get(&piece_coord) {
            Some(piece) => (&piece.kind, piece.kind.color()),
            None => return,
        };

        // Get valid move coordinates based on piece type
        let valid_coords: Vec<CubeCoord> = match piece_kind {
            PieceKind::BlackKing | PieceKind::WhiteKing => piece_coord.neighbors().to_vec(),
            PieceKind::BlackKnight | PieceKind::WhiteKnight => piece_coord.knight_moves().to_vec(),
            PieceKind::BlackRook | PieceKind::WhiteRook => self.get_rook_moves(piece_coord, piece_color),
            PieceKind::BlackQueen | PieceKind::WhiteQueen => self.get_queen_moves(piece_coord, piece_color),
            PieceKind::BlackPawn | PieceKind::WhitePawn => self.get_pawn_moves(piece_coord, piece_color),
        };

        // Highlight valid move hexes, excluding those with same-color pieces
        for coord in valid_coords.iter() {
            // Check if the hex exists on the map
            if let Some(hex) = self.hexes.get_mut(coord) {
                // Rooks, queens, and pawns have collision detection built into their move generation
                // Kings and knights still need manual filtering
                match piece_kind {
                    PieceKind::BlackRook | PieceKind::WhiteRook
                    | PieceKind::BlackQueen | PieceKind::WhiteQueen
                    | PieceKind::BlackPawn | PieceKind::WhitePawn => {
                        hex.valid_move = true;
                    }
                    _ => {
                        // Check if there's a piece at this location
                        if let Some(target_piece) = self.pieces.get(coord) {
                            // Skip if same color (can't capture own pieces)
                            if target_piece.kind.color() == piece_color {
                                continue;
                            }
                            // Different color - can capture
                        }
                        // Either empty or capturable - mark as valid move
                        hex.valid_move = true;
                    }
                }
            }
        }
    }

    fn move_piece(&mut self, from: CubeCoord, to: CubeCoord) {
        // Start animation instead of instant move
        if self.pieces.contains_key(&from) {
            self.animation = Some(Animation {
                from,
                to,
                progress: 0.0,
            });
        }
    }

    fn update_animation(&mut self, delta_time: f32) {
        if let Some(ref mut anim) = self.animation {
            anim.progress += delta_time / ANIMATION_DURATION;

            if anim.progress >= 1.0 {
                // Animation complete - finalize the move
                let from = anim.from;
                let to = anim.to;

                // Remove any piece at the destination (capture)
                self.pieces.remove(&to);

                // Move the piece to the destination
                if let Some(mut piece) = self.pieces.remove(&from) {
                    piece.coord = to;
                    self.pieces.insert(to, piece);
                }

                self.animation = None;
            }
        }
    }

    fn is_animating(&self) -> bool {
        self.animation.is_some()
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Chess I guess?")
        .vsync()
        .msaa_4x()
        .build();

    let mut map = Map::new(MAP_RADIUS, &mut rl, &thread);
    while !rl.window_should_close() {
        // Update animation
        let delta_time = rl.get_frame_time();
        map.update_animation(delta_time);

        let mut d = rl.begin_drawing(&thread);
        map.frame_reset();

        // Handle click events
        if d.is_mouse_button_pressed(raylib::consts::MouseButton::MOUSE_BUTTON_LEFT) {
            map.handle_click(&d);
        }

        let _ = map.highlight_hex_under_cursor(&d);

        map.draw(&mut d);
    }
}
