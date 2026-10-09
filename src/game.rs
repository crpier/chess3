//! Rendering-independent rules for the documented Hex Chess house variant.
use std::collections::BTreeMap;

pub const RADIUS: i32 = 5;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}
impl Hex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }
    pub fn add(self, d: Self) -> Self {
        Self::new(self.q + d.q, self.r + d.r)
    }
    pub fn scale(self, n: i32) -> Self {
        Self::new(self.q * n, self.r * n)
    }
    pub fn on_board(self) -> bool {
        self.q.abs().max(self.r.abs()).max((self.q + self.r).abs()) <= RADIUS
    }
    pub fn label(self) -> String {
        format!(
            "{}{}",
            (b'a' + (self.q + 5) as u8) as char,
            self.r - (-5).max(-5 - self.q) + 1
        )
    }
}
pub const EDGES: [Hex; 6] = [
    Hex::new(1, -1),
    Hex::new(1, 0),
    Hex::new(0, 1),
    Hex::new(-1, 1),
    Hex::new(-1, 0),
    Hex::new(0, -1),
];
pub const DIAGONALS: [Hex; 6] = [
    Hex::new(2, -1),
    Hex::new(1, 1),
    Hex::new(-1, 2),
    Hex::new(-2, 1),
    Hex::new(-1, -1),
    Hex::new(1, -2),
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    White,
    Black,
}
impl Side {
    pub fn other(self) -> Self {
        if self == Self::White {
            Self::Black
        } else {
            Self::White
        }
    }
    pub fn name(self) -> &'static str {
        if self == Self::White {
            "White"
        } else {
            "Black"
        }
    }
    pub fn forward(self) -> Hex {
        Hex::new(0, if self == Self::White { -1 } else { 1 })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}
impl Kind {
    pub const PROMOTIONS: [Self; 4] = [Self::Queen, Self::Rook, Self::Bishop, Self::Knight];
    pub fn name(self) -> &'static str {
        match self {
            Self::Pawn => "Pawn",
            Self::Knight => "Knight",
            Self::Bishop => "Bishop",
            Self::Rook => "Rook",
            Self::Queen => "Queen",
            Self::King => "King",
        }
    }
    pub fn value(self) -> i32 {
        match self {
            Self::Pawn => 100,
            Self::Knight => 320,
            Self::Bishop => 350,
            Self::Rook => 500,
            Self::Queen => 950,
            Self::King => 20000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub side: Side,
    pub kind: Kind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: Hex,
    pub to: Hex,
    pub promotion: Option<Kind>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Checkmate(Side),
    Resigned(Side),
    Draw(&'static str),
}
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    pub pieces: BTreeMap<Hex, Piece>,
    pub turn: Side,
    pub quiet_plies: u32,
}
impl Position {
    pub fn initial() -> Self {
        let mut pieces = BTreeMap::new();
        let back = [
            Kind::Rook,
            Kind::Knight,
            Kind::Bishop,
            Kind::Queen,
            Kind::King,
            Kind::Bishop,
            Kind::Knight,
            Kind::Bishop,
            Kind::Rook,
        ];
        for (i, kind) in back.into_iter().enumerate() {
            let q = i as i32 - 4;
            let r = 5.min(5 - q);
            for (side, sign) in [(Side::White, 1), (Side::Black, -1)] {
                pieces.insert(Hex::new(q * sign, r * sign), Piece { side, kind });
                pieces.insert(
                    Hex::new(q * sign, (r - 1) * sign),
                    Piece {
                        side,
                        kind: Kind::Pawn,
                    },
                );
            }
        }
        Self {
            pieces,
            turn: Side::White,
            quiet_plies: 0,
        }
    }
    fn pawn_captures(side: Side) -> [Hex; 2] {
        if side == Side::White {
            [Hex::new(1, -1), Hex::new(-1, 0)]
        } else {
            [Hex::new(-1, 1), Hex::new(1, 0)]
        }
    }
    fn rays(&self, from: Hex, side: Side, dirs: &[Hex]) -> Vec<Hex> {
        let mut result = Vec::new();
        for &dir in dirs {
            let mut to = from.add(dir);
            while to.on_board() {
                if let Some(piece) = self.pieces.get(&to) {
                    if piece.side != side {
                        result.push(to);
                    }
                    break;
                }
                result.push(to);
                to = to.add(dir);
            }
        }
        result
    }
    fn targets(&self, from: Hex, piece: Piece, attacks: bool) -> Vec<Hex> {
        let mut result = match piece.kind {
            Kind::Rook => self.rays(from, piece.side, &EDGES),
            Kind::Bishop => self.rays(from, piece.side, &DIAGONALS),
            Kind::Queen => self.rays(from, piece.side, &[EDGES, DIAGONALS].concat()),
            Kind::King => [EDGES, DIAGONALS]
                .concat()
                .into_iter()
                .map(|d| from.add(d))
                .collect(),
            Kind::Knight => (0..6)
                .flat_map(|i| {
                    [
                        from.add(EDGES[i].scale(2)).add(EDGES[(i + 1) % 6]),
                        from.add(EDGES[i].scale(2)).add(EDGES[(i + 5) % 6]),
                    ]
                })
                .collect(),
            Kind::Pawn => {
                let mut targets = Vec::new();
                if !attacks {
                    let forward = from.add(piece.side.forward());
                    if forward.on_board() && !self.pieces.contains_key(&forward) {
                        targets.push(forward);
                    }
                }
                for dir in Self::pawn_captures(piece.side) {
                    let to = from.add(dir);
                    if attacks || self.pieces.get(&to).is_some_and(|p| p.side != piece.side) {
                        targets.push(to);
                    }
                }
                targets
            }
        };
        result.retain(|h| {
            h.on_board() && (attacks || !self.pieces.get(h).is_some_and(|p| p.side == piece.side))
        });
        result
    }
    pub fn attacked(&self, hex: Hex, by: Side) -> bool {
        self.pieces.iter().any(|(&from, &piece)| {
            piece.side == by && self.targets(from, piece, true).contains(&hex)
        })
    }
    pub fn in_check(&self, side: Side) -> bool {
        self.pieces
            .iter()
            .find(|(_, p)| p.side == side && p.kind == Kind::King)
            .is_none_or(|(&h, _)| self.attacked(h, side.other()))
    }
    pub fn legal_from(&self, from: Hex) -> Vec<Move> {
        let Some(&piece) = self.pieces.get(&from) else {
            return Vec::new();
        };
        if piece.side != self.turn {
            return Vec::new();
        }
        let mut moves = Vec::new();
        for to in self.targets(from, piece, false) {
            if self.pieces.get(&to).is_some_and(|p| p.kind == Kind::King) {
                continue;
            }
            let promotes = piece.kind == Kind::Pawn && !to.add(piece.side.forward()).on_board();
            let options = if promotes {
                Kind::PROMOTIONS.map(Some).to_vec()
            } else {
                vec![None]
            };
            for promotion in options {
                let mv = Move {
                    from,
                    to,
                    promotion,
                };
                let mut next = self.clone();
                next.apply_unchecked(mv);
                if !next.in_check(piece.side) {
                    moves.push(mv);
                }
            }
        }
        moves
    }
    pub fn legal_moves(&self) -> Vec<Move> {
        self.pieces
            .keys()
            .flat_map(|&h| self.legal_from(h))
            .collect()
    }
    fn apply_unchecked(&mut self, mv: Move) {
        let mut piece = self.pieces.remove(&mv.from).expect("move source");
        let capture = self.pieces.remove(&mv.to).is_some();
        self.quiet_plies = if capture || piece.kind == Kind::Pawn {
            0
        } else {
            self.quiet_plies + 1
        };
        if let Some(kind) = mv.promotion {
            piece.kind = kind;
        }
        self.pieces.insert(mv.to, piece);
        self.turn = self.turn.other();
    }
}
#[derive(Clone)]
pub struct Game {
    pub position: Position,
    pub history: Vec<Position>,
    pub moves: Vec<String>,
    pub last_move: Option<Move>,
    pub outcome: Outcome,
}
impl Default for Game {
    fn default() -> Self {
        Self::new(Position::initial())
    }
}
impl Game {
    pub fn new(position: Position) -> Self {
        let mut game = Self {
            position,
            history: Vec::new(),
            moves: Vec::new(),
            last_move: None,
            outcome: Outcome::Playing,
        };
        game.refresh_outcome();
        game
    }
    pub fn play(&mut self, mv: Move) -> bool {
        if self.outcome != Outcome::Playing || !self.position.legal_from(mv.from).contains(&mv) {
            return false;
        }
        let piece = self.position.pieces[&mv.from];
        let capture = self.position.pieces.contains_key(&mv.to);
        self.history.push(self.position.clone());
        self.position.apply_unchecked(mv);
        self.moves.push(format!(
            "{} {} {}{}{}{}",
            piece.side.name(),
            piece.kind.name(),
            mv.from.label(),
            if capture { " x " } else { " - " },
            mv.to.label(),
            mv.promotion
                .map_or(String::new(), |k| format!(" = {}", k.name()))
        ));
        self.last_move = Some(mv);
        self.refresh_outcome();
        true
    }
    pub fn resign(&mut self, side: Side) {
        if self.outcome == Outcome::Playing {
            self.outcome = Outcome::Resigned(side.other());
        }
    }
    pub fn undo(&mut self) -> bool {
        if matches!(self.outcome, Outcome::Resigned(_)) {
            self.refresh_outcome();
            return true;
        }
        if let Some(position) = self.history.pop() {
            self.position = position;
            self.moves.pop();
            self.last_move = None;
            self.refresh_outcome();
            true
        } else {
            false
        }
    }
    fn refresh_outcome(&mut self) {
        self.outcome = if self.position.legal_moves().is_empty() {
            if self.position.in_check(self.position.turn) {
                Outcome::Checkmate(self.position.turn.other())
            } else {
                Outcome::Draw("Stalemate")
            }
        } else if self.position.quiet_plies >= 100 {
            Outcome::Draw("50-move rule")
        } else if self.position.pieces.values().all(|p| p.kind == Kind::King) {
            Outcome::Draw("Bare kings")
        } else if self
            .history
            .iter()
            .filter(|p| p.turn == self.position.turn && p.pieces == self.position.pieces)
            .count()
            >= 2
        {
            Outcome::Draw("Threefold repetition")
        } else {
            Outcome::Playing
        };
    }
    /// A deliberately small one-ply opponent: captures, safety, checks, then variety.
    pub fn ai_move(&self) -> Option<Move> {
        self.position.legal_moves().into_iter().max_by_key(|mv| {
            let moving = self.position.pieces[&mv.from];
            let mut next = self.position.clone();
            next.apply_unchecked(*mv);
            let capture = self
                .position
                .pieces
                .get(&mv.to)
                .map_or(0, |p| p.kind.value());
            let promotion = mv.promotion.map_or(0, |k| k.value() - 100);
            let danger = if next.attacked(mv.to, moving.side.other()) {
                moving.kind.value() / 2
            } else {
                0
            };
            let variety = (mv.from.q * 17
                + mv.from.r * 31
                + mv.to.q * 43
                + mv.to.r * 13
                + self.moves.len() as i32 * 7)
                .rem_euclid(23);
            capture + promotion - danger + if next.in_check(next.turn) { 40 } else { 0 } + variety
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn position(pieces: &[(i32, i32, Side, Kind)], turn: Side) -> Position {
        Position {
            pieces: pieces
                .iter()
                .map(|&(q, r, side, kind)| (Hex::new(q, r), Piece { side, kind }))
                .collect(),
            turn,
            quiet_plies: 0,
        }
    }
    #[test]
    fn initial_board_and_turn() {
        let p = Position::initial();
        assert_eq!(p.pieces.len(), 36);
        assert!(p.pieces.keys().all(|h| h.on_board()));
        assert!(!p.in_check(Side::White));
        assert!(!p.in_check(Side::Black));
        assert!(p.legal_from(Hex::new(0, -5)).is_empty());
        assert!(!p.legal_moves().is_empty());
    }
    #[test]
    fn rook_blocking_and_capture() {
        let p = position(
            &[
                (0, 5, Side::White, Kind::King),
                (0, -5, Side::Black, Kind::King),
                (0, 0, Side::White, Kind::Rook),
                (0, -2, Side::Black, Kind::Pawn),
                (1, 0, Side::White, Kind::Pawn),
            ],
            Side::White,
        );
        let to: Vec<_> = p.legal_from(Hex::new(0, 0)).iter().map(|m| m.to).collect();
        assert!(to.contains(&Hex::new(0, -2)));
        assert!(!to.contains(&Hex::new(0, -3)));
        assert!(!to.contains(&Hex::new(1, 0)));
    }
    #[test]
    fn bishop_and_queen_diagonals() {
        let p = position(
            &[
                (0, 5, Side::White, Kind::King),
                (0, -5, Side::Black, Kind::King),
                (0, 0, Side::White, Kind::Bishop),
            ],
            Side::White,
        );
        assert!(
            p.legal_from(Hex::new(0, 0))
                .iter()
                .any(|m| m.to == Hex::new(2, -1))
        );
        assert!(
            !p.legal_from(Hex::new(0, 0))
                .iter()
                .any(|m| m.to == Hex::new(1, 0))
        );
        let mut q = p.clone();
        q.pieces.get_mut(&Hex::new(0, 0)).unwrap().kind = Kind::Queen;
        assert!(
            q.legal_from(Hex::new(0, 0))
                .iter()
                .any(|m| m.to == Hex::new(1, 0))
        );
    }
    #[test]
    fn pawn_attacks_not_forward_and_promotes() {
        let p = position(
            &[
                (4, 0, Side::White, Kind::King),
                (0, -5, Side::Black, Kind::King),
                (-2, -2, Side::White, Kind::Pawn),
                (-1, -3, Side::Black, Kind::Rook),
            ],
            Side::White,
        );
        assert!(p.attacked(Hex::new(-1, -3), Side::White));
        assert!(!p.attacked(Hex::new(-2, -3), Side::White));
        let moves = p.legal_from(Hex::new(-2, -2));
        assert_eq!(moves.iter().filter(|m| m.to == Hex::new(-2, -3)).count(), 4);
        let mut g = Game::new(p);
        assert!(g.play(Move {
            from: Hex::new(-2, -2),
            to: Hex::new(-2, -3),
            promotion: Some(Kind::Knight)
        }));
        assert_eq!(g.position.pieces[&Hex::new(-2, -3)].kind, Kind::Knight);
    }
    #[test]
    fn pinned_piece_cannot_expose_king() {
        let p = position(
            &[
                (0, 3, Side::White, Kind::King),
                (4, -5, Side::Black, Kind::King),
                (0, 2, Side::White, Kind::Rook),
                (0, -2, Side::Black, Kind::Rook),
            ],
            Side::White,
        );
        assert!(
            !p.legal_from(Hex::new(0, 2))
                .iter()
                .any(|m| m.to == Hex::new(1, 2))
        );
    }
    #[test]
    fn king_cannot_enter_check_or_capture_king() {
        let p = position(
            &[
                (0, 0, Side::White, Kind::King),
                (0, -2, Side::Black, Kind::King),
            ],
            Side::White,
        );
        assert!(
            !p.legal_from(Hex::new(0, 0))
                .iter()
                .any(|m| m.to == Hex::new(0, -1))
        );
        assert!(!p.legal_moves().iter().any(|m| m.to == Hex::new(0, -2)));
    }
    #[test]
    fn undo_restores_whole_position() {
        let mut g = Game::default();
        let before = g.position.clone();
        let mv = g.position.legal_moves()[0];
        assert!(g.play(mv));
        assert_eq!(g.position.turn, Side::Black);
        assert!(g.undo());
        assert!(g.position == before);
        assert!(g.moves.is_empty());
    }
    #[test]
    fn mate_and_stalemate() {
        // Surround the corner king with covered escape squares.
        let mate = position(
            &[
                (0, -5, Side::Black, Kind::King),
                (0, -2, Side::White, Kind::King),
                (0, -4, Side::White, Kind::Queen),
                (-2, -2, Side::White, Kind::Rook),
            ],
            Side::Black,
        );
        assert_eq!(Game::new(mate).outcome, Outcome::Checkmate(Side::White));
        let stale = position(
            &[
                (0, -5, Side::Black, Kind::King),
                (0, -2, Side::White, Kind::King),
                (-5, 1, Side::White, Kind::Queen),
                (-5, 4, Side::White, Kind::Queen),
            ],
            Side::Black,
        );
        assert!(!stale.in_check(Side::Black));
        assert_eq!(Game::new(stale).outcome, Outcome::Draw("Stalemate"));
    }
    #[test]
    fn bishops_start_on_three_colors() {
        let p = Position::initial();
        for side in [Side::White, Side::Black] {
            let colors: std::collections::BTreeSet<_> = p
                .pieces
                .iter()
                .filter(|(_, p)| p.side == side && p.kind == Kind::Bishop)
                .map(|(h, _)| (h.q - h.r).rem_euclid(3))
                .collect();
            assert_eq!(colors.len(), 3);
        }
    }
    #[test]
    fn resignation_can_be_undone_without_a_move() {
        let mut g = Game::default();
        g.resign(Side::White);
        assert_eq!(g.outcome, Outcome::Resigned(Side::Black));
        assert!(g.undo());
        assert_eq!(g.outcome, Outcome::Playing);
        assert!(g.position == Position::initial());
    }
    #[test]
    fn repetition_and_fifty_move_draws() {
        let p = position(
            &[
                (0, 4, Side::White, Kind::King),
                (0, -4, Side::Black, Kind::King),
                (4, -2, Side::White, Kind::Rook),
            ],
            Side::White,
        );
        let mut g = Game::new(p);
        let cycle = [
            (Hex::new(0, 4), Hex::new(0, 3)),
            (Hex::new(0, -4), Hex::new(0, -3)),
            (Hex::new(0, 3), Hex::new(0, 4)),
            (Hex::new(0, -3), Hex::new(0, -4)),
        ];
        for _ in 0..2 {
            for (from, to) in cycle {
                assert!(g.play(Move {
                    from,
                    to,
                    promotion: None
                }));
            }
        }
        assert_eq!(g.outcome, Outcome::Draw("Threefold repetition"));
        assert!(g.undo());
        assert_eq!(g.outcome, Outcome::Playing);
        let mut p = g.position.clone();
        p.quiet_plies = 100;
        assert_eq!(Game::new(p).outcome, Outcome::Draw("50-move rule"));
    }
    #[test]
    fn ai_games_keep_kings_and_terminate() {
        let mut g = Game::default();
        for _ in 0..180 {
            let Some(mv) = g.ai_move() else { break };
            if !g.play(mv) {
                break;
            }
            assert_eq!(
                g.position
                    .pieces
                    .values()
                    .filter(|p| p.kind == Kind::King)
                    .count(),
                2
            );
        }
    }
}
