//! Rendering-independent rules for the documented Hex Chess house variant.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const RADIUS: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuLevel {
    Easy,
    Normal,
    Hard,
}
impl CpuLevel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Easy => Self::Normal,
            Self::Normal => Self::Hard,
            Self::Hard => Self::Easy,
        }
    }
    pub fn decision_seconds(self) -> f32 {
        match self {
            Self::Easy => 2.4,
            Self::Normal => 1.5,
            Self::Hard => 0.65,
        }
    }
}

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
    pub fn distance(self, other: Self) -> i32 {
        let (q, r) = (self.q - other.q, self.r - other.r);
        q.abs().max(r.abs()).max((q + r).abs())
    }
    pub fn on_board(self) -> bool {
        // Short-circuit before summing, so malformed integer coordinates cannot overflow.
        (-RADIUS..=RADIUS).contains(&self.q)
            && (-RADIUS..=RADIUS).contains(&self.r)
            && (-RADIUS..=RADIUS).contains(&(self.q + self.r))
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
    pub fn movement(self) -> &'static str {
        match self {
            Self::King => "One touching tile in any direction.",
            Self::Pawn => "One step into the forward fan; move or capture alike.",
            Self::Knight => "Jump exactly two tiles away; ignore intervening pieces.",
            Self::Bishop => "Walk up to two touching steps. May turn; cannot jump.",
            Self::Rook => "Slide along a straight row; stop at the first piece.",
            Self::Queen => "Rook slides or a short two-step bishop walk.",
        }
    }
    fn value(self) -> i32 {
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
    KingCaptured(Side),
    Resigned(Side),
    Draw(&'static str),
}
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    pub pieces: BTreeMap<Hex, Piece>,
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
        Self { pieces }
    }
}

impl Position {
    fn rays(&self, from: Hex, dirs: &[Hex]) -> Vec<Hex> {
        let mut result = vec![];
        for &dir in dirs {
            let mut h = from.add(dir);
            while h.on_board() {
                result.push(h);
                if self.pieces.contains_key(&h) {
                    break;
                }
                h = h.add(dir);
            }
        }
        result
    }
    fn walks(&self, from: Hex) -> Vec<Hex> {
        let mut result = BTreeSet::new();
        for dir in EDGES {
            let mid = from.add(dir);
            if !mid.on_board() {
                continue;
            }
            result.insert(mid);
            if self.pieces.contains_key(&mid) {
                continue;
            }
            for next in EDGES {
                let to = mid.add(next);
                if to != from && to.on_board() {
                    result.insert(to);
                }
            }
        }
        result.into_iter().collect()
    }
    pub fn legal_from(&self, from: Hex) -> Vec<Move> {
        let Some(piece) = self.pieces.get(&from) else {
            return vec![];
        };
        let mut targets = match piece.kind {
            Kind::Rook => self.rays(from, &EDGES),
            Kind::Bishop => self.walks(from),
            Kind::Queen => [self.rays(from, &EDGES), self.walks(from)].concat(),
            Kind::King => EDGES.iter().map(|&d| from.add(d)).collect(),
            Kind::Knight => (0..6)
                .flat_map(|i| {
                    [
                        from.add(EDGES[i].scale(2)),
                        from.add(EDGES[i]).add(EDGES[(i + 1) % 6]),
                    ]
                })
                .collect(),
            Kind::Pawn => {
                let sign = if piece.side == Side::White { 1 } else { -1 };
                [Hex::new(0, -1), Hex::new(1, -1), Hex::new(-1, 0)]
                    .into_iter()
                    .map(|d| from.add(d.scale(sign)))
                    .collect()
            }
        };
        targets
            .retain(|h| h.on_board() && !self.pieces.get(h).is_some_and(|p| p.side == piece.side));
        targets.sort_unstable();
        targets.dedup();
        targets
            .into_iter()
            .flat_map(|to| {
                let promotes = piece.kind == Kind::Pawn && !to.add(piece.side.forward()).on_board();
                if promotes {
                    Kind::PROMOTIONS
                        .into_iter()
                        .map(|kind| Move {
                            from,
                            to,
                            promotion: Some(kind),
                        })
                        .collect::<Vec<_>>()
                } else {
                    vec![Move {
                        from,
                        to,
                        promotion: None,
                    }]
                }
            })
            .collect()
    }
    fn path(&self, mv: Move) -> VecDeque<Hex> {
        let kind = self.pieces[&mv.from].kind;
        if kind == Kind::Knight {
            return VecDeque::from([mv.to]);
        }
        for dir in EDGES {
            for n in 1..=10 {
                if mv.from.add(dir.scale(n)) == mv.to {
                    return (1..=n).map(|i| mv.from.add(dir.scale(i))).collect();
                }
            }
        }
        // The only remaining geometry is a two-step bishop bend.
        let via = EDGES
            .into_iter()
            .map(|d| mv.from.add(d))
            .find(|h| h.on_board() && !self.pieces.contains_key(h) && h.distance(mv.to) == 1)
            .expect("legal bend has a clear path");
        VecDeque::from([via, mv.to])
    }
    pub fn attacked(&self, h: Hex, by: Side) -> bool {
        self.pieces
            .iter()
            .filter(|(_, p)| p.side == by)
            .any(|(&from, _)| self.legal_from(from).iter().any(|m| m.to == h))
    }
    pub fn in_check(&self, side: Side) -> bool {
        self.pieces
            .iter()
            .find(|(_, p)| p.side == side && p.kind == Kind::King)
            .is_some_and(|(&h, _)| self.attacked(h, side.other()))
    }
}
pub const START_DELAY: u64 = 3000;
pub const MOVE_LIMIT: usize = 2;
const STEP: u64 = 20;
const QUIET: u64 = 180_000;
const ROOT3: f64 = 1.7320508075688772;
// Collision is a close crossing of centers, not the broad edges of artwork.
// A quarter tile lets a lateral escape succeed without an instant response.
const CONTACT: f64 = 0.25;
fn point(h: Hex) -> (f64, f64) {
    (h.q as f64 * ROOT3 / 2., h.r as f64 + h.q as f64 / 2.)
}
fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
#[derive(Clone)]
pub struct Unit {
    pub id: u64,
    pub anchor: Hex,
    pub piece: Piece,
    pub x: f64,
    pub y: f64,
    route: VecDeque<Hex>,
    goal: Option<Hex>,
    speed: f64,
    promotion: Kind,
}
impl Unit {
    pub fn axial(&self) -> (f32, f32) {
        let q = self.x * 2. / ROOT3;
        (q as f32, (self.y - q / 2.) as f32)
    }
    pub fn tile(&self) -> Hex {
        let q = self.x * 2. / ROOT3;
        let r = self.y - q / 2.;
        let s = -q - r;
        let (mut iq, mut ir, is) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((q - iq).abs(), (r - ir).abs(), (s - is).abs());
        if dq > dr && dq > ds {
            iq = -ir - is;
        } else if dr > ds {
            ir = -iq - is;
        }
        Hex::new(iq as i32, ir as i32)
    }
    pub fn marching(&self) -> bool {
        !self.route.is_empty()
    }
    /// The promotion committed with this pawn's current destination, not its preset.
    pub fn promoting_to(&self) -> Option<Kind> {
        if self.piece.kind != Kind::Pawn {
            return None;
        }
        self.goal
            .filter(|h| !h.add(self.piece.side.forward()).on_board())
            .map(|_| self.promotion)
    }
    /// Planned travel in simulation ticks; captures or death can end it earlier.
    pub fn remaining_ms(&self) -> u64 {
        if !self.marching() {
            return 0;
        }
        let mut previous = self.pos();
        let mut remaining = 0.;
        for &h in &self.route {
            let next = point(h);
            remaining += distance(previous, next);
            previous = next;
        }
        // Avoid adding a whole tick for floating-point noise at exact boundaries.
        ((remaining / self.speed / (STEP as f64 / 1000.) - 1e-8)
            .ceil()
            .max(0.) as u64)
            * STEP
    }
    pub fn waypoints(&self) -> impl Iterator<Item = Hex> + '_ {
        self.route.iter().copied()
    }
    fn pos(&self) -> (f64, f64) {
        (self.x, self.y)
    }
    fn airborne(&self) -> bool {
        self.piece.kind == Kind::Knight
            && self
                .goal
                .is_some_and(|h| distance(self.pos(), point(h)) > CONTACT)
    }
}
#[derive(Clone, Copy)]
pub struct Order {
    pub id: u64,
    pub mv: Move,
    pub piece: Piece,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Finished,
    Countdown,
    Committed,
    Full,
    Illegal,
}
impl Rejection {
    pub fn message(self) -> &'static str {
        match self {
            Self::Finished => "The game has ended.",
            Self::Countdown => "Move rejected: the opening countdown was still running.",
            Self::Committed => {
                "Move rejected: that piece was already committed. Nothing was queued."
            }
            Self::Full => "Move rejected: both slots were occupied. Nothing was queued.",
            Self::Illegal => "Move rejected: that destination or lane was unavailable.",
        }
    }
}
/// Two independent committed moves per side. No post-move cooldown, passive damage,
/// healing, army advance or cancellation. Physical travel and interception run together.
pub struct Game {
    pub position: Position,
    pub moves: Vec<String>,
    pub outcome: Outcome,
    units: Vec<Unit>,
    now: u64,
    simulated: u64,
    last_progress: u64,
    losses: [usize; 2],
}
fn index(side: Side) -> usize {
    usize::from(side == Side::Black)
}
impl Default for Game {
    fn default() -> Self {
        Self::new(Position::initial())
    }
}
impl Game {
    pub fn new(position: Position) -> Self {
        let units = position
            .pieces
            .iter()
            .enumerate()
            .map(|(id, (&anchor, &piece))| {
                let (x, y) = point(anchor);
                Unit {
                    id: id as u64,
                    anchor,
                    piece,
                    x,
                    y,
                    route: VecDeque::new(),
                    goal: None,
                    speed: 0.,
                    promotion: Kind::Queen,
                }
            })
            .collect();
        let mut g = Self {
            position,
            units,
            moves: vec![],
            outcome: Outcome::Playing,
            now: 0,
            simulated: 0,
            last_progress: START_DELAY,
            losses: [0; 2],
        };
        g.refresh_outcome();
        g
    }
    pub fn now(&self) -> u64 {
        self.now
    }
    pub fn units(&self) -> &[Unit] {
        &self.units
    }
    pub fn unit(&self, id: u64) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }
    pub fn captured_count(&self, side: Side) -> usize {
        self.losses[index(side)]
    }
    fn committed_threat_at(&self, side: Side, h: Hex) -> bool {
        self.units
            .iter()
            .any(|enemy| enemy.piece.side != side && enemy.route.contains(&h))
    }
    /// Advisory danger includes committed attacks removed from the static board.
    /// It never rejects an escape or changes capture resolution.
    pub fn king_in_danger(&self, side: Side) -> bool {
        if self.outcome != Outcome::Playing {
            return false;
        }
        self.position.in_check(side)
            || self
                .units
                .iter()
                .filter(|u| u.piece.side == side && u.piece.kind == Kind::King && !u.marching())
                .any(|king| self.committed_threat_at(side, king.anchor))
    }
    pub fn active(&self, side: Side) -> usize {
        self.units
            .iter()
            .filter(|u| u.piece.side == side && u.marching())
            .count()
    }
    pub fn orders(&self) -> Vec<Order> {
        self.units
            .iter()
            .filter_map(|u| {
                u.goal.map(|to| Order {
                    id: u.id,
                    mv: Move {
                        from: u.anchor,
                        to,
                        promotion: u.promoting_to(),
                    },
                    piece: u.piece,
                })
            })
            .collect()
    }
    fn planning(&self, side: Side) -> Position {
        let mut pieces: BTreeMap<Hex, Piece> = self
            .units
            .iter()
            .filter(|u| !u.marching())
            .map(|u| (u.anchor, u.piece))
            .collect();
        // Reservations must dominate newly arrived enemies, regardless of unit ID.
        // Otherwise a new enemy can accidentally reopen an already committed lane
        // and let two friendly movers land on the same square.
        for u in self
            .units
            .iter()
            .filter(|u| u.piece.side == side && u.marching())
        {
            for &h in &u.route {
                pieces.insert(h, u.piece);
            }
        }
        Position { pieces }
    }
    pub fn legal_from(&self, id: u64) -> Vec<Move> {
        let Some(u) = self.unit(id) else {
            return vec![];
        };
        if self.outcome != Outcome::Playing || u.marching() {
            return vec![];
        }
        self.planning(u.piece.side).legal_from(u.anchor)
    }
    pub fn submit(&mut self, id: u64, mv: Move) -> Result<(), Rejection> {
        if self.outcome != Outcome::Playing {
            return Err(Rejection::Finished);
        }
        if self.now < START_DELAY {
            return Err(Rejection::Countdown);
        }
        let u = self
            .unit(id)
            .filter(|u| u.anchor == mv.from)
            .ok_or(Rejection::Illegal)?;
        if u.marching() {
            return Err(Rejection::Committed);
        }
        if self.active(u.piece.side) >= MOVE_LIMIT {
            return Err(Rejection::Full);
        }
        if !mv.to.on_board() {
            return Err(Rejection::Illegal);
        }
        let mut mv = mv;
        if u.piece.kind == Kind::Pawn
            && !mv.to.add(u.piece.side.forward()).on_board()
            && mv.promotion.is_none()
        {
            mv.promotion = Some(u.promotion);
        }
        let planning = self.planning(u.piece.side);
        if !planning.legal_from(mv.from).contains(&mv) {
            return Err(Rejection::Illegal);
        }
        let route = planning.path(mv);
        let mut previous = point(mv.from);
        let mut length = 0.;
        for &h in &route {
            let p = point(h);
            length += distance(previous, p);
            previous = p;
        }
        let duration = match u.piece.kind {
            Kind::King => 0.8,
            Kind::Knight => 1.2,
            _ => route.len() as f64 * 1.2,
        };
        let u = self.units.iter_mut().find(|u| u.id == id).unwrap();
        u.route = route;
        u.goal = Some(mv.to);
        u.speed = length / duration;
        if let Some(kind) = mv.promotion {
            u.promotion = kind;
        }
        self.position.pieces.remove(&mv.from);
        Ok(())
    }
    pub fn set_promotion(&mut self, side: Side, kind: Kind) {
        if Kind::PROMOTIONS.contains(&kind) {
            for u in &mut self.units {
                if u.piece.side == side && !u.marching() {
                    u.promotion = kind;
                }
            }
        }
    }
    pub fn advance(&mut self, ms: u64) {
        if self.outcome != Outcome::Playing {
            return;
        }
        let target = self.now.saturating_add(ms);
        while self.simulated.saturating_add(STEP) <= target {
            self.simulated += STEP;
            self.now = self.simulated;
            if self.now > START_DELAY {
                self.tick();
            }
            self.refresh_outcome();
            if self.outcome != Outcome::Playing {
                return;
            }
        }
        self.now = target;
    }
    fn tick(&mut self) {
        let snapshot = self.units.clone();
        let dt = STEP as f64 / 1000.;
        for u in &mut self.units {
            if let Some(&next) = u.route.front() {
                let p = point(next);
                let d = distance(u.pos(), p);
                let step = (u.speed * dt).min(d);
                if d > 0.000001 {
                    u.x += (p.0 - u.x) * step / d;
                    u.y += (p.1 - u.y) * step / d;
                }
            }
        }
        let mut dead = BTreeSet::new();
        let mut interceptions = vec![];
        // Swept collision prevents a frame's movement from skipping a crossing.
        for i in 0..snapshot.len() {
            for j in i + 1..snapshot.len() {
                let (a, b) = (&snapshot[i], &snapshot[j]);
                if a.piece.side == b.piece.side {
                    continue;
                }
                let (pa, pb) = (self.units[i].pos(), self.units[j].pos());
                let old = (a.x - b.x, a.y - b.y);
                let delta = ((pa.0 - pb.0) - old.0, (pa.1 - pb.1) - old.1);
                let len = delta.0 * delta.0 + delta.1 * delta.1;
                let t = if len > 0. {
                    (-(old.0 * delta.0 + old.1 * delta.1) / len).clamp(0., 1.)
                } else {
                    0.
                };
                if (old.0 + t * delta.0).hypot(old.1 + t * delta.1) > CONTACT {
                    continue;
                }
                match (a.marching(), b.marching()) {
                    (true, true) => {
                        if a.airborne() || b.airborne() {
                            continue;
                        }
                        dead.insert(a.id);
                        dead.insert(b.id);
                    }
                    (true, false)
                        if a.route.front() == Some(&b.anchor)
                            && distance(pa, point(b.anchor)) < 0.00001 =>
                    {
                        dead.insert(b.id);
                        interceptions.push((a.id, b.anchor));
                    }
                    (false, true)
                        if b.route.front() == Some(&a.anchor)
                            && distance(pb, point(a.anchor)) < 0.00001 =>
                    {
                        dead.insert(a.id);
                        interceptions.push((b.id, a.anchor));
                    }
                    _ => {}
                }
            }
        }
        // A capture completes at the occupied waypoint, never at a sprite's edge.
        // A newly arrived blocker stops a slide rather than being passed through.
        for (id, h) in interceptions {
            if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                u.route = VecDeque::from([h]);
                u.goal = Some(h);
            }
        }
        for (u, moved) in snapshot.iter().zip(&self.units) {
            if dead.contains(&u.id) {
                self.losses[index(u.piece.side)] += 1;
                self.last_progress = self.now;
                self.moves.push(format!(
                    "{} {} {} {}",
                    u.piece.side.name(),
                    u.piece.kind.name(),
                    if u.marching() {
                        "intercepted near"
                    } else {
                        "captured at"
                    },
                    moved.tile().label()
                ));
            }
        }
        self.units.retain(|u| !dead.contains(&u.id));
        for u in &mut self.units {
            if let Some(&next) = u.route.front()
                && distance(u.pos(), point(next)) < 0.00001
            {
                (u.x, u.y) = point(next);
                u.anchor = next;
                u.route.pop_front();
                if u.piece.kind == Kind::Pawn {
                    self.last_progress = self.now;
                }
                if u.route.is_empty() {
                    u.goal = None;
                    if u.piece.kind == Kind::Pawn
                        && !u.anchor.add(u.piece.side.forward()).on_board()
                    {
                        u.piece.kind = u.promotion;
                        self.moves.push(format!(
                            "{} promoted at {}",
                            u.piece.side.name(),
                            u.anchor.label()
                        ));
                    }
                    self.moves.push(format!(
                        "{} {} arrived at {}",
                        u.piece.side.name(),
                        u.piece.kind.name(),
                        u.anchor.label()
                    ));
                }
            }
        }
        self.position.pieces = self
            .units
            .iter()
            .filter(|u| !u.marching())
            .map(|u| (u.anchor, u.piece))
            .collect();
    }
    fn refresh_outcome(&mut self) {
        let has = |s| {
            self.units
                .iter()
                .any(|u| u.piece.side == s && u.piece.kind == Kind::King)
        };
        self.outcome = match (has(Side::White), has(Side::Black)) {
            (false, false) => Outcome::Draw("Both kings captured simultaneously"),
            (false, true) => Outcome::KingCaptured(Side::Black),
            (true, false) => Outcome::KingCaptured(Side::White),
            _ if self.now >= self.last_progress + QUIET => {
                Outcome::Draw("3 minutes without a capture or pawn advance")
            }
            _ => Outcome::Playing,
        };
    }
    pub fn resign(&mut self, side: Side) {
        if self.outcome == Outcome::Playing {
            self.outcome = Outcome::Resigned(side.other());
        }
    }
    /// Easy voluntarily uses one slot and short, varied legal moves without tactical scoring.
    /// Normal and Hard share the tactical policy; their reaction cadence differs.
    pub fn cpu_move(&self, side: Side, level: CpuLevel) -> Option<(u64, Move)> {
        if level != CpuLevel::Easy {
            return self.ai_move(side);
        }
        if self.now < START_DELAY || self.outcome != Outcome::Playing || self.active(side) > 0 {
            return None;
        }
        let choices: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.piece.side == side && !u.marching())
            .flat_map(|u| self.legal_from(u.id).into_iter().map(move |m| (u.id, m)))
            .filter(|(_, m)| m.from.distance(m.to) <= 2)
            .collect();
        if choices.is_empty() {
            return None;
        }
        let choice =
            (self.now / STEP).wrapping_add(self.moves.len() as u64 * 17) % choices.len() as u64;
        Some(choices[choice as usize])
    }
    pub fn ai_move(&self, side: Side) -> Option<(u64, Move)> {
        if self.now < START_DELAY
            || self.outcome != Outcome::Playing
            || self.active(side) >= MOVE_LIMIT
        {
            return None;
        }
        self.units
            .iter()
            .filter(|u| u.piece.side == side && !u.marching())
            .flat_map(|u| self.legal_from(u.id).into_iter().map(move |m| (u.id, m)))
            .max_by_key(|(_, m)| {
                let piece = self.position.pieces[&m.from];
                let mut next = self.position.clone();
                next.pieces.remove(&m.from);
                next.pieces.insert(m.to, piece);
                let capture = self
                    .units
                    .iter()
                    .filter(|u| !u.marching() && u.anchor == m.to)
                    .map(|u| u.piece.kind.value())
                    .sum::<i32>();
                if self
                    .position
                    .pieces
                    .get(&m.to)
                    .is_some_and(|p| p.kind == Kind::King)
                {
                    return i32::MAX / 2;
                }
                let danger = if next.attacked(m.to, side.other()) {
                    piece.kind.value() / 2
                } else {
                    0
                };
                let incoming = next.pieces.iter().any(|(&h, p)| {
                    p.side == side && p.kind == Kind::King && self.committed_threat_at(side, h)
                });
                let safety = if next.in_check(side) || incoming {
                    -20000
                } else {
                    0
                };
                let variety = (m.from.q * 17
                    + m.from.r * 31
                    + m.to.q * 43
                    + m.to.r * 13
                    + self.moves.len() as i32 * 7)
                    .rem_euclid(23);
                capture - danger + safety + variety + m.promotion.map_or(0, |k| k.value() - 100)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(extra: &[(i32, i32, Side, Kind)]) -> Game {
        let mut pieces = BTreeMap::from([
            (
                Hex::new(0, 5),
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
        ]);
        for &(q, r, side, kind) in extra {
            pieces.insert(Hex::new(q, r), Piece { side, kind });
        }
        let mut g = Game::new(Position { pieces });
        g.advance(START_DELAY);
        g
    }
    fn mv(from: (i32, i32), to: (i32, i32)) -> Move {
        Move {
            from: Hex::new(from.0, from.1),
            to: Hex::new(to.0, to.1),
            promotion: None,
        }
    }
    fn id(g: &Game, side: Side, h: Hex) -> u64 {
        g.units
            .iter()
            .find(|u| u.piece.side == side && u.anchor == h)
            .unwrap()
            .id
    }
    fn command(
        g: &mut Game,
        side: Side,
        from: (i32, i32),
        to: (i32, i32),
    ) -> Result<(), Rejection> {
        g.submit(id(g, side, Hex::new(from.0, from.1)), mv(from, to))
    }
    fn state(g: &Game) -> Vec<(u64, Hex, f64, f64, bool)> {
        g.units
            .iter()
            .map(|u| (u.id, u.anchor, u.x, u.y, u.marching()))
            .collect()
    }
    #[test]
    fn easy_cpu_commits_only_one_legal_move_while_harder_levels_can_use_two() {
        let mut g = Game::default();
        assert!(g.cpu_move(Side::Black, CpuLevel::Easy).is_none());
        g.advance(START_DELAY);
        let (id, mv) = g.cpu_move(Side::Black, CpuLevel::Easy).unwrap();
        assert!(g.legal_from(id).contains(&mv));
        g.submit(id, mv).unwrap();
        assert!(g.cpu_move(Side::Black, CpuLevel::Easy).is_none());
        assert!(g.cpu_move(Side::Black, CpuLevel::Normal).is_some());
        assert!(g.cpu_move(Side::Black, CpuLevel::Hard).is_some());
        g.advance(2400);
        assert_eq!(g.active(Side::Black), 0);
        assert!(g.cpu_move(Side::Black, CpuLevel::Easy).is_some());
        g.resign(Side::Black);
        assert!(g.cpu_move(Side::Black, CpuLevel::Easy).is_none());
    }
    #[test]
    fn two_slots_are_independent_and_a_thousand_third_orders_are_rejected() {
        let mut g = Game::default();
        assert_eq!(
            command(&mut g, Side::White, (0, 4), (0, 3)),
            Err(Rejection::Countdown)
        );
        g.advance(START_DELAY);
        command(&mut g, Side::White, (0, 4), (0, 3)).unwrap();
        command(&mut g, Side::White, (1, 3), (1, 2)).unwrap();
        let before = state(&g);
        for _ in 0..1000 {
            assert_eq!(
                command(&mut g, Side::White, (-3, 5), (-5, 5)),
                Err(Rejection::Full)
            );
        }
        assert_eq!(state(&g), before);
        assert_eq!(g.active(Side::White), 2);
        command(&mut g, Side::Black, (0, -4), (0, -3)).unwrap();
        command(&mut g, Side::Black, (-1, -3), (-1, -2)).unwrap();
        assert_eq!(g.orders().len(), 4);
        g.advance(1199);
        assert_eq!(g.active(Side::White), 2);
        g.advance(1);
        assert_eq!(g.active(Side::White), 0);
        command(&mut g, Side::White, (0, 3), (0, 2)).unwrap(); // No post-move cooldown.
    }
    #[test]
    fn commitment_cannot_be_cancelled_or_redirected_by_spam() {
        let mut a = fixture(&[(0, 3, Side::White, Kind::Rook)]);
        let mut b = fixture(&[(0, 3, Side::White, Kind::Rook)]);
        command(&mut a, Side::White, (0, 3), (0, 0)).unwrap();
        command(&mut b, Side::White, (0, 3), (0, 0)).unwrap();
        let mover = id(&b, Side::White, Hex::new(0, 3));
        for _ in 0..100 {
            for _ in 0..10 {
                let from = b.unit(mover).unwrap().anchor;
                assert_eq!(
                    b.submit(
                        mover,
                        Move {
                            from,
                            to: Hex::new(-2, 3),
                            promotion: None
                        }
                    ),
                    Err(Rejection::Committed)
                );
            }
            a.advance(20);
            b.advance(20);
        }
        assert_eq!(state(&a), state(&b));
        assert_eq!(a.orders()[0].mv.to, Hex::new(0, 0));
    }
    #[test]
    fn adjacent_idle_pieces_do_not_fight_or_move_automatically() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Pawn),
            (1, 0, Side::Black, Kind::Pawn),
            (0, 2, Side::White, Kind::Rook),
            (-1, 0, Side::Black, Kind::Bishop),
        ]);
        let before = state(&g);
        g.advance(20000);
        assert_eq!(state(&g), before);
        assert!(g.moves.is_empty());
        assert!(g.orders().is_empty());
    }
    #[test]
    fn off_board_and_extreme_destinations_are_rejected_without_mutating_the_game() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Pawn),
            (2, 0, Side::Black, Kind::Pawn),
        ]);
        let before = state(&g);
        for side in [Side::White, Side::Black] {
            let from = if side == Side::White {
                Hex::new(0, 0)
            } else {
                Hex::new(2, 0)
            };
            let mover = id(&g, side, from);
            for to in [
                Hex::new(i32::MAX, i32::MAX),
                Hex::new(i32::MIN, i32::MIN),
                Hex::new(i32::MIN, 0),
                Hex::new(0, i32::MIN),
                Hex::new(0, i32::MAX),
                Hex::new(6, 0),
                Hex::new(5, 1),
            ] {
                assert_eq!(
                    g.submit(
                        mover,
                        Move {
                            from,
                            to,
                            promotion: None
                        }
                    ),
                    Err(Rejection::Illegal)
                );
                assert!(!to.on_board());
                assert_eq!(state(&g), before);
            }
        }
    }
    #[test]
    fn every_piece_has_its_own_chess_geometry() {
        let center = Hex::new(0, 0);
        for (kind, count) in [
            (Kind::King, 6),
            (Kind::Pawn, 3),
            (Kind::Bishop, 18),
            (Kind::Knight, 12),
            (Kind::Rook, 30),
            (Kind::Queen, 36),
        ] {
            let p = Position {
                pieces: [(
                    center,
                    Piece {
                        side: Side::White,
                        kind,
                    },
                )]
                .into_iter()
                .collect(),
            };
            let moves = p.legal_from(center);
            assert_eq!(moves.len(), count, "{}", kind.name());
            for m in moves {
                match kind {
                    Kind::Pawn | Kind::King => assert_eq!(center.distance(m.to), 1),
                    Kind::Bishop => assert!(center.distance(m.to) <= 2),
                    Kind::Knight => assert_eq!(center.distance(m.to), 2),
                    _ => {}
                }
            }
        }
    }
    #[test]
    fn captures_complete_at_landing_and_never_free_a_slot_early() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Rook),
            (1, 0, Side::Black, Kind::Pawn),
        ]);
        command(&mut g, Side::White, (0, 0), (1, 0)).unwrap();
        g.advance(700);
        assert_eq!(g.captured_count(Side::Black), 0);
        assert_eq!(g.active(Side::White), 1);
        g.advance(500);
        assert_eq!(g.active(Side::White), 0);
        assert!(g.position.pieces.contains_key(&Hex::new(1, 0)));
        command(&mut g, Side::White, (1, 0), (2, 0)).unwrap();
    }
    #[test]
    fn opposing_crossings_are_symmetric_and_dead_pieces_cannot_resurrect() {
        for reverse in [false, true] {
            let mut g = fixture(&[
                (-1, 0, Side::White, Kind::Rook),
                (1, 0, Side::Black, Kind::Rook),
            ]);
            let mut orders = [
                (Side::White, (-1, 0), (0, 0)),
                (Side::Black, (1, 0), (0, 0)),
            ];
            if reverse {
                orders.reverse();
            }
            for (side, from, to) in orders {
                command(&mut g, side, from, to).unwrap();
            }
            g.advance(5000);
            assert_eq!(g.losses, [1, 1]);
            assert_eq!(g.units.len(), 2);
            assert!(g.orders().is_empty());
        }
    }
    #[test]
    fn friendly_destinations_and_bishop_paths_are_reserved() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Bishop),
            (3, -1, Side::White, Kind::Rook),
        ]);
        command(&mut g, Side::White, (0, 0), (2, -1)).unwrap();
        assert_eq!(
            command(&mut g, Side::White, (3, -1), (2, -1)),
            Err(Rejection::Illegal)
        );
        assert_eq!(g.active(Side::White), 1);
    }
    #[test]
    fn newly_arrived_enemy_does_not_reopen_a_reserved_friendly_lane() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Rook),
            (-2, 3, Side::White, Kind::Rook),
            (2, 3, Side::Black, Kind::Knight),
        ]);
        command(&mut g, Side::White, (0, 0), (0, 4)).unwrap();
        command(&mut g, Side::Black, (2, 3), (0, 3)).unwrap();
        g.advance(1200);
        assert_eq!(g.position.pieces[&Hex::new(0, 3)].side, Side::Black);
        assert_eq!(
            command(&mut g, Side::White, (-2, 3), (0, 3)),
            Err(Rejection::Illegal)
        );
        g.advance(3600);
        assert_eq!(
            g.position.pieces.len(),
            g.units.iter().filter(|u| !u.marching()).count()
        );
    }
    #[test]
    fn intervening_blockers_stop_walks_but_not_knight_jumps() {
        let mut p = Position {
            pieces: [
                (
                    Hex::new(0, 0),
                    Piece {
                        side: Side::White,
                        kind: Kind::Bishop,
                    },
                ),
                (
                    Hex::new(1, 0),
                    Piece {
                        side: Side::White,
                        kind: Kind::Pawn,
                    },
                ),
                (
                    Hex::new(1, -1),
                    Piece {
                        side: Side::Black,
                        kind: Kind::Pawn,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };
        assert!(!p.legal_from(Hex::new(0, 0)).contains(&mv((0, 0), (2, -1))));
        p.pieces.get_mut(&Hex::new(0, 0)).unwrap().kind = Kind::Knight;
        assert!(p.legal_from(Hex::new(0, 0)).contains(&mv((0, 0), (2, -1))));
    }
    #[test]
    fn knight_vaults_a_stationary_enemy_without_capturing_it() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Knight),
            (0, 1, Side::Black, Kind::Pawn),
        ]);
        command(&mut g, Side::White, (0, 0), (0, 2)).unwrap();
        g.advance(2000);
        assert_eq!(g.losses, [0, 0]);
        assert_eq!(
            g.unit(id(&g, Side::White, Hex::new(0, 2))).unwrap().anchor,
            Hex::new(0, 2)
        );
    }
    #[test]
    fn vacated_knight_origin_can_be_occupied_without_losing_identity() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Knight),
            (0, -1, Side::Black, Kind::King),
        ]);
        g.units.retain(|u| u.anchor != Hex::new(0, -5));
        g.position.pieces.remove(&Hex::new(0, -5));
        let knight = id(&g, Side::White, Hex::new(0, 0));
        command(&mut g, Side::White, (0, 0), (0, 2)).unwrap();
        command(&mut g, Side::Black, (0, -1), (0, 0)).unwrap();
        g.advance(800);
        assert!(g.unit(knight).unwrap().marching());
        assert_eq!(g.position.pieces[&Hex::new(0, 0)].side, Side::Black);
        g.advance(400);
        assert!(g.unit(knight).is_some());
        assert_eq!(g.position.pieces.len(), 3);
    }
    #[test]
    fn king_escape_is_physical_not_a_health_check() {
        let mut g = fixture(&[(0, -3, Side::White, Kind::Rook)]);
        command(&mut g, Side::White, (0, -3), (0, -5)).unwrap();
        command(&mut g, Side::Black, (0, -5), (1, -5)).unwrap();
        g.advance(3000);
        assert_eq!(g.outcome, Outcome::Playing);
        assert_eq!(g.losses, [0, 0]);
    }
    #[test]
    fn simultaneous_king_crossing_draws_and_freezes_the_game() {
        let mut g = Game::new(Position {
            pieces: [
                (
                    Hex::new(0, 0),
                    Piece {
                        side: Side::White,
                        kind: Kind::King,
                    },
                ),
                (
                    Hex::new(1, 0),
                    Piece {
                        side: Side::Black,
                        kind: Kind::King,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        });
        g.advance(START_DELAY);
        command(&mut g, Side::White, (0, 0), (1, 0)).unwrap();
        command(&mut g, Side::Black, (1, 0), (0, 0)).unwrap();
        g.advance(2000);
        assert_eq!(
            g.outcome,
            Outcome::Draw("Both kings captured simultaneously")
        );
        let before = state(&g);
        let now = g.now();
        g.advance(10000);
        assert_eq!(state(&g), before);
        assert_eq!(g.now(), now);
    }
    #[test]
    fn bot_takes_a_winning_king_capture_even_if_the_square_is_defended() {
        let mut g = Game::new(Position {
            pieces: [
                (
                    Hex::new(0, 0),
                    Piece {
                        side: Side::White,
                        kind: Kind::King,
                    },
                ),
                (
                    Hex::new(1, 0),
                    Piece {
                        side: Side::Black,
                        kind: Kind::King,
                    },
                ),
                (
                    Hex::new(2, 0),
                    Piece {
                        side: Side::Black,
                        kind: Kind::Queen,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        });
        g.advance(START_DELAY);
        let (_, m) = g.ai_move(Side::White).unwrap();
        assert_eq!(m.to, Hex::new(1, 0));
    }
    #[test]
    fn promotion_is_committed_with_the_move() {
        let mut g = fixture(&[(-2, -2, Side::White, Kind::Pawn)]);
        let mover = id(&g, Side::White, Hex::new(-2, -2));
        g.submit(
            mover,
            Move {
                from: Hex::new(-2, -2),
                to: Hex::new(-2, -3),
                promotion: Some(Kind::Knight),
            },
        )
        .unwrap();
        assert_eq!(g.orders()[0].mv.promotion, Some(Kind::Knight));
        g.set_promotion(Side::White, Kind::Queen);
        assert_eq!(g.unit(mover).unwrap().promoting_to(), Some(Kind::Knight));
        g.advance(1200);
        assert_eq!(g.unit(mover).unwrap().piece.kind, Kind::Knight);
    }
    #[test]
    fn travel_times_follow_steps_and_piece_roles_exactly() {
        for (kind, from, to, duration) in [
            (Kind::King, (0, 5), (0, 4), 800),
            (Kind::Knight, (0, 0), (2, 0), 1200),
            (Kind::Pawn, (0, 0), (0, -1), 1200),
            (Kind::Bishop, (0, 0), (1, 1), 2400),
            (Kind::Rook, (0, 0), (3, 0), 3600),
            (Kind::Queen, (0, 0), (3, 0), 3600),
        ] {
            let extra = if kind == Kind::King {
                vec![]
            } else {
                vec![(from.0, from.1, Side::White, kind)]
            };
            let mut g = fixture(&extra);
            let mover = id(&g, Side::White, Hex::new(from.0, from.1));
            command(&mut g, Side::White, from, to).unwrap();
            assert_eq!(g.unit(mover).unwrap().remaining_ms(), duration);
            g.advance(duration - 20);
            assert_eq!(g.unit(mover).unwrap().remaining_ms(), 20);
            assert_eq!(g.active(Side::White), 1, "{kind:?}");
            g.advance(20);
            assert_eq!(g.unit(mover).unwrap().remaining_ms(), 0);
            assert_eq!(g.active(Side::White), 0, "{kind:?}");
            assert_eq!(g.position.pieces[&Hex::new(to.0, to.1)].kind, kind);
        }
    }
    #[test]
    fn winning_capture_leaves_the_attacker_landed_and_no_order_in_flight() {
        let mut g = fixture(&[(0, -4, Side::White, Kind::Queen)]);
        let queen = id(&g, Side::White, Hex::new(0, -4));
        command(&mut g, Side::White, (0, -4), (0, -5)).unwrap();
        g.advance(1180);
        assert_eq!(g.outcome, Outcome::Playing);
        g.advance(20);
        assert_eq!(g.outcome, Outcome::KingCaptured(Side::White));
        assert_eq!(g.unit(queen).unwrap().pos(), point(Hex::new(0, -5)));
        assert_eq!(g.active(Side::White), 0);
        assert!(g.orders().is_empty());
    }
    #[test]
    fn fixed_step_chunking_does_not_change_moves_or_captures() {
        let extra = [
            (0, 0, Side::White, Kind::Rook),
            (2, 0, Side::Black, Kind::Pawn),
        ];
        let mut a = fixture(&extra);
        let mut b = fixture(&extra);
        command(&mut a, Side::White, (0, 0), (2, 0)).unwrap();
        command(&mut b, Side::White, (0, 0), (2, 0)).unwrap();
        a.advance(3125);
        for _ in 0..625 {
            b.advance(5);
        }
        assert_eq!(a.captured_count(Side::Black), 1);
        assert_eq!(a.active(Side::White), 0);
        assert_eq!(state(&a), state(&b));
        assert_eq!(a.moves, b.moves);
    }
    #[test]
    fn bots_never_exceed_slots_and_preserve_material_and_unique_stationary_cells() {
        let mut g = Game::default();
        g.advance(START_DELAY);
        for _ in 0..1000 {
            for side in [Side::White, Side::Black] {
                if let Some((id, m)) = g.ai_move(side) {
                    g.submit(id, m).unwrap();
                }
                assert!(g.active(side) <= MOVE_LIMIT);
            }
            g.advance(100);
            assert_eq!(
                g.position.pieces.len(),
                g.units.iter().filter(|u| !u.marching()).count()
            );
            assert_eq!(g.units.len() + g.losses.iter().sum::<usize>(), 36);
            for u in &g.units {
                assert!(u.tile().on_board());
                assert!(u.x.is_finite() && u.y.is_finite());
            }
        }
        assert!(!g.moves.is_empty());
    }
    #[test]
    fn corner_king_can_evade_a_committed_attack_after_a_half_second_response() {
        for target in [(-1, -4), (1, -5)] {
            let mut g = fixture(&[(0, -4, Side::White, Kind::Queen)]);
            let king = id(&g, Side::Black, Hex::new(0, -5));
            command(&mut g, Side::White, (0, -4), (0, -5)).unwrap();
            g.advance(500);
            command(&mut g, Side::Black, (0, -5), target).unwrap();
            g.advance(800);
            assert_eq!(g.outcome, Outcome::Playing, "escape={target:?}");
            assert_eq!(g.unit(king).unwrap().anchor, Hex::new(target.0, target.1));
            assert_eq!(g.captured_count(Side::Black), 0);
        }
    }
    #[test]
    fn two_slot_clearance_maneuver_creates_a_royal_escape_without_castling_rules() {
        let mut g = fixture(&[
            (0, -4, Side::White, Kind::Queen),
            (1, -5, Side::Black, Kind::Queen),
            (-1, -4, Side::Black, Kind::Bishop),
        ]);
        let king = id(&g, Side::Black, Hex::new(0, -5));
        let queen = id(&g, Side::Black, Hex::new(1, -5));
        command(&mut g, Side::White, (0, -4), (0, -5)).unwrap();
        g.advance(500);
        assert_eq!(
            command(&mut g, Side::Black, (0, -5), (1, -5)),
            Err(Rejection::Illegal)
        );
        command(&mut g, Side::Black, (1, -5), (1, -4)).unwrap();
        command(&mut g, Side::Black, (0, -5), (1, -5)).unwrap();
        assert_eq!(g.active(Side::Black), 2);
        assert_eq!(
            command(&mut g, Side::Black, (-1, -4), (-1, -3)),
            Err(Rejection::Full)
        );
        g.advance(800);
        assert_eq!(g.outcome, Outcome::Playing);
        assert_eq!(g.unit(king).unwrap().anchor, Hex::new(1, -5));
        assert!(g.unit(queen).unwrap().marching());
        assert_eq!(g.position.pieces[&Hex::new(1, -5)].kind, Kind::King);
        assert_eq!(g.units.len(), 5);
    }
    #[test]
    fn interception_log_uses_physical_location_instead_of_old_waypoints() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Rook),
            (0, 4, Side::Black, Kind::Rook),
        ]);
        command(&mut g, Side::White, (0, 0), (0, 4)).unwrap();
        command(&mut g, Side::Black, (0, 4), (0, 0)).unwrap();
        g.advance(3000);
        let location = Hex::new(0, 2).label();
        assert_eq!(g.moves.len(), 2);
        assert!(
            g.moves
                .iter()
                .all(|line| line.contains("intercepted near") && line.ends_with(&location))
        );
        assert_eq!(g.captured_count(Side::White), 1);
        assert_eq!(g.captured_count(Side::Black), 1);
    }
    #[test]
    fn late_blocker_stops_a_slide_at_capture_and_releases_the_unused_lane() {
        let mut g = fixture(&[
            (0, 0, Side::White, Kind::Rook),
            (2, 3, Side::Black, Kind::Knight),
        ]);
        let rook = id(&g, Side::White, Hex::new(0, 0));
        command(&mut g, Side::White, (0, 0), (0, 4)).unwrap();
        command(&mut g, Side::Black, (2, 3), (0, 3)).unwrap();
        g.advance(3580);
        assert_eq!(g.captured_count(Side::Black), 0);
        assert_eq!(g.active(Side::White), 1);
        g.advance(20);
        assert_eq!(g.captured_count(Side::Black), 1);
        assert_eq!(g.unit(rook).unwrap().anchor, Hex::new(0, 3));
        assert_eq!(g.active(Side::White), 0);
        assert!(g.unit(rook).unwrap().waypoints().next().is_none());
        command(&mut g, Side::White, (0, 5), (0, 4)).unwrap();
        g.advance(800);
        assert_eq!(g.unit(rook).unwrap().anchor, Hex::new(0, 3));
    }
    #[test]
    fn seeded_mixed_play_preserves_material_reservations_and_board_projection() {
        for seed in 1..=24 {
            let mut random = seed as u64;
            let mut roll = || {
                random = random
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                random >> 32
            };
            let mut g = Game::default();
            g.advance(START_DELAY);
            for turn in 0..1500 {
                if g.outcome != Outcome::Playing {
                    break;
                }
                let sides = if roll() % 2 == 0 {
                    [Side::White, Side::Black]
                } else {
                    [Side::Black, Side::White]
                };
                for side in sides {
                    let ready: Vec<u64> = g
                        .units
                        .iter()
                        .filter(|u| u.piece.side == side && !u.marching())
                        .map(|u| u.id)
                        .collect();
                    if g.active(side) < MOVE_LIMIT && !ready.is_empty() {
                        let mover = ready[roll() as usize % ready.len()];
                        let moves = g.legal_from(mover);
                        if !moves.is_empty() {
                            g.submit(mover, moves[roll() as usize % moves.len()])
                                .unwrap();
                        }
                    }
                    assert!(g.active(side) <= MOVE_LIMIT, "seed={seed} turn={turn}");
                    let mut reserved = BTreeSet::new();
                    for u in g.units.iter().filter(|u| u.piece.side == side) {
                        for &h in &u.route {
                            assert!(
                                reserved.insert(h),
                                "shared friendly lane: seed={seed} turn={turn} tile={h:?}"
                            );
                        }
                    }
                }
                g.advance([20, 40, 100, 200][roll() as usize % 4]);
                let stationary: Vec<_> = g.units.iter().filter(|u| !u.marching()).collect();
                assert_eq!(
                    g.position.pieces.len(),
                    stationary.len(),
                    "seed={seed} turn={turn}"
                );
                let expected: BTreeMap<_, _> =
                    stationary.iter().map(|u| (u.anchor, u.piece)).collect();
                assert_eq!(g.position.pieces, expected, "seed={seed} turn={turn}");
                assert_eq!(g.units.len() + g.losses.iter().sum::<usize>(), 36);
                assert_eq!(
                    g.units.iter().map(|u| u.id).collect::<BTreeSet<_>>().len(),
                    g.units.len()
                );
                assert!(
                    g.units
                        .iter()
                        .all(|u| u.x.is_finite() && u.y.is_finite() && u.tile().on_board())
                );
            }
        }
    }
    #[test]
    fn bot_defends_against_an_incoming_king_capture_instead_of_grabbing_material() {
        let mut g = fixture(&[
            (0, -3, Side::White, Kind::Queen),
            (3, -3, Side::Black, Kind::Rook),
            (3, -2, Side::White, Kind::Pawn),
        ]);
        command(&mut g, Side::White, (0, -3), (0, -5)).unwrap();
        let (mover, mv) = g.ai_move(Side::Black).unwrap();
        assert_eq!(g.unit(mover).unwrap().piece.kind, Kind::King);
        assert!(!g.committed_threat_at(Side::Black, mv.to));
        g.submit(mover, mv).unwrap();
        g.advance(800);
        assert_eq!(g.outcome, Outcome::Playing);
        assert!(!g.king_in_danger(Side::Black));
    }
    #[test]
    fn king_warning_survives_attack_commitment_but_allows_an_escape() {
        let mut g = fixture(&[(0, 3, Side::Black, Kind::Rook)]);
        assert!(g.king_in_danger(Side::White));
        command(&mut g, Side::Black, (0, 3), (0, 5)).unwrap();
        assert!(!g.position.in_check(Side::White));
        assert!(g.king_in_danger(Side::White));
        command(&mut g, Side::White, (0, 5), (1, 4)).unwrap();
        g.advance(800);
        assert!(!g.king_in_danger(Side::White));
        assert_eq!(g.outcome, Outcome::Playing);
    }
    #[test]
    fn quiet_draw_and_resignation_freeze_without_passive_combat() {
        let mut g = fixture(&[]);
        g.advance(QUIET - 20);
        assert_eq!(g.outcome, Outcome::Playing);
        g.advance(20);
        assert!(matches!(g.outcome, Outcome::Draw(_)));
        let mut g = fixture(&[]);
        g.resign(Side::White);
        let now = g.now();
        g.advance(10000);
        assert_eq!(g.now(), now);
        assert_eq!(g.outcome, Outcome::Resigned(Side::Black));
    }
}
