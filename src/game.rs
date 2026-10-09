//! Rendering-independent rules for the documented Hex Chess house variant.
use std::collections::{BTreeMap, VecDeque};

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
    pub fn distance(self, other: Self) -> i32 {
        let (q, r) = (self.q - other.q, self.r - other.r);
        q.abs().max(r.abs()).max((q + r).abs())
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
            Self::King => "Commander: protect him. Walks and fights nearby.",
            Self::Pawn => "Frontline: advances forward; fights nearby enemies.",
            Self::Knight => "Fast cavalry: vaults traffic; fights at close range.",
            Self::Bishop => "Support: heals one nearby ally; weak melee attack.",
            Self::Rook => "Artillery: slow march, long-range clear-line fire.",
            Self::Queen => "Ranged fighter: mobile fire behind the frontline.",
        }
    }
    pub fn stats(self) -> Stats {
        match self {
            Self::King => Stats::new(180., 0.9, 16., 1.12),
            Self::Pawn => Stats::new(80., 0.75, 18., 1.12),
            Self::Knight => Stats::new(110., 1.5, 26., 1.12),
            Self::Bishop => Stats::new(75., 0.9, 10., 1.12),
            Self::Rook => Stats::new(120., 0.65, 22., 3.2),
            Self::Queen => Stats::new(140., 0.85, 24., 2.6),
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
    KingDefeated(Side),
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

#[derive(Clone, Copy)]
pub struct Stats {
    pub health: f64,
    pub speed: f64,
    pub damage: f64,
    pub range: f64,
}
impl Stats {
    const fn new(health: f64, speed: f64, damage: f64, range: f64) -> Self {
        Self {
            health,
            speed,
            damage,
            range,
        }
    }
}
pub const START_DELAY: u64 = 3000;
const STEP: u64 = 20;
const QUIET: u64 = 180_000;
const ACCEL: f64 = 6.;
const SPACING: f64 = 0.62;
const ROOT3: f64 = 1.7320508075688772;
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
    pub health: f64,
    pub target: Option<u64>,
    pub healing: Option<u64>,
    vx: f64,
    vy: f64,
    route: VecDeque<Hex>,
    goal: Option<Hex>,
    advance: bool,
    repath: u64,
    promotion: Kind,
}
impl Unit {
    pub fn axial(&self) -> (f32, f32) {
        let q = self.x * 2. / ROOT3;
        (q as f32, (self.y - q / 2.) as f32)
    }
    pub fn marching(&self) -> bool {
        self.vx.hypot(self.vy) > 0.01
    }
    fn pos(&self) -> (f64, f64) {
        (self.x, self.y)
    }
    fn vaults(&self, other: &Self) -> bool {
        self.piece.kind == Kind::Knight
            && self
                .route
                .front()
                .is_some_and(|h| self.anchor.distance(*h) == 2 && *h != other.anchor)
    }
}
#[derive(Clone, Copy)]
pub struct Order {
    pub mv: Move,
    pub piece: Piece,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Finished,
    Countdown,
    Illegal,
}
impl Rejection {
    pub fn message(self) -> &'static str {
        match self {
            Self::Finished => "The battle has ended.",
            Self::Countdown => "Wait for the opening countdown.",
            Self::Illegal => "No accessible route to that tile. Try a different destination.",
        }
    }
}
/// A fixed-step battlefield: orders change intentions, never position, health or momentum.
/// Movement, traffic, damage, healing and death are simulated for ALL pieces together.
pub struct Game {
    pub position: Position,
    pub moves: Vec<String>,
    pub last_move: Option<Move>,
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
                    health: piece.kind.stats().health,
                    target: None,
                    healing: None,
                    vx: 0.,
                    vy: 0.,
                    route: VecDeque::new(),
                    goal: None,
                    advance: false,
                    repath: 0,
                    promotion: Kind::Queen,
                }
            })
            .collect();
        let mut g = Self {
            position,
            units,
            moves: vec![],
            last_move: None,
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
    pub fn unit_at(&self, h: Hex) -> Option<&Unit> {
        self.units.iter().find(|u| u.anchor == h)
    }
    pub fn hex_of(&self, id: u64) -> Option<Hex> {
        self.units.iter().find(|u| u.id == id).map(|u| u.anchor)
    }
    pub fn captured_count(&self, side: Side) -> usize {
        self.losses[index(side)]
    }
    pub fn orders(&self) -> Vec<Order> {
        self.units
            .iter()
            .filter_map(|u| {
                u.goal.map(|to| Order {
                    mv: Move {
                        from: u.anchor,
                        to,
                        promotion: None,
                    },
                    piece: u.piece,
                })
            })
            .collect()
    }
    fn routes(&self, from: Hex) -> BTreeMap<Hex, Hex> {
        let Some(unit) = self.unit_at(from) else {
            return BTreeMap::new();
        };
        let mut parents = BTreeMap::from([(from, from)]);
        let mut queue = VecDeque::from([from]);
        while let Some(h) = queue.pop_front() {
            let mut dirs = EDGES.to_vec();
            if unit.piece.kind == Kind::Pawn {
                let sign = if unit.piece.side == Side::White {
                    1
                } else {
                    -1
                };
                dirs = vec![
                    Hex::new(0, -1).scale(sign),
                    Hex::new(1, -1).scale(sign),
                    Hex::new(-1, 0).scale(sign),
                ];
            } else if unit.piece.kind == Kind::Knight {
                for i in 0..6 {
                    dirs.push(EDGES[i].scale(2));
                    dirs.push(EDGES[i].add(EDGES[(i + 1) % 6]));
                }
            }
            for dir in dirs {
                let next = h.add(dir);
                if !next.on_board() || parents.contains_key(&next) {
                    continue;
                }
                if let Some(p) = self.position.pieces.get(&next) {
                    if p.side == unit.piece.side {
                        continue;
                    }
                    parents.insert(next, h); // An enemy is approachable, never a through-route.
                } else {
                    parents.insert(next, h);
                    queue.push_back(next);
                }
            }
        }
        parents
    }
    fn path(parents: &BTreeMap<Hex, Hex>, from: Hex, to: Hex) -> Option<VecDeque<Hex>> {
        if from == to || !parents.contains_key(&to) {
            return None;
        }
        let mut h = to;
        let mut result = VecDeque::new();
        while h != from {
            result.push_front(h);
            h = *parents.get(&h)?;
        }
        Some(result)
    }
    pub fn legal_from(&self, from: Hex) -> Vec<Move> {
        if self.outcome != Outcome::Playing {
            return vec![];
        }
        self.routes(from)
            .keys()
            .filter(|&&h| h != from)
            .map(|&to| Move {
                from,
                to,
                promotion: None,
            })
            .collect()
    }
    pub fn submit(&mut self, mv: Move) -> Result<(), Rejection> {
        if self.outcome != Outcome::Playing {
            return Err(Rejection::Finished);
        }
        if self.now < START_DELAY {
            return Err(Rejection::Countdown);
        }
        if mv.promotion.is_some_and(|k| !Kind::PROMOTIONS.contains(&k)) {
            return Err(Rejection::Illegal);
        }
        let parents = self.routes(mv.from);
        let route = Self::path(&parents, mv.from, mv.to).ok_or(Rejection::Illegal)?;
        let u = self
            .units
            .iter_mut()
            .find(|u| u.anchor == mv.from)
            .ok_or(Rejection::Illegal)?;
        // Idempotence: repeated input cannot restart movement, firing or a route segment.
        if u.goal == Some(mv.to) && !u.advance {
            return Ok(());
        }
        u.route = route;
        u.goal = Some(mv.to);
        u.advance = false;
        if let Some(kind) = mv.promotion {
            u.promotion = kind;
        }
        Ok(())
    }
    pub fn stop(&mut self, h: Hex) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if let Some(u) = self.units.iter_mut().find(|u| u.anchor == h) {
            u.route.clear();
            u.goal = None;
            u.advance = false;
        }
    }
    pub fn set_promotion(&mut self, side: Side, kind: Kind) {
        if Kind::PROMOTIONS.contains(&kind) {
            for u in &mut self.units {
                if u.piece.side == side {
                    u.promotion = kind;
                }
            }
        }
    }
    pub fn attack_army(&mut self, side: Side) {
        if self.outcome != Outcome::Playing || self.now < START_DELAY {
            return;
        }
        for u in &mut self.units {
            if u.piece.side == side && !u.advance {
                u.advance = true;
                u.repath = 0; // Keeps existing momentum and damage state.
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
        // Autonomous advance persists even while blocked. Replanning is simulation work,
        // not an action cooldown: every piece still moves/fights on every tick.
        for i in 0..self.units.len() {
            if !self.units[i].advance || self.now < self.units[i].repath {
                continue;
            }
            let from = self.units[i].anchor;
            let side = self.units[i].piece.side;
            let parents = self.routes(from);
            let target = self
                .units
                .iter()
                .filter(|u| u.piece.side != side && parents.contains_key(&u.anchor))
                .min_by(|a, b| {
                    distance(self.units[i].pos(), a.pos())
                        .total_cmp(&distance(self.units[i].pos(), b.pos()))
                })
                .map(|u| u.anchor);
            if let Some(to) = target
                && let Some(path) = Self::path(&parents, from, to)
            {
                self.units[i].route = path;
                self.units[i].goal = Some(to);
            }
            self.units[i].repath = self.now + 500;
        }
        let snapshot = self.units.clone();
        let dt = STEP as f64 / 1000.;
        let mut proposed = Vec::with_capacity(snapshot.len());
        for u in &snapshot {
            let holding = u.advance && (u.target.is_some() || u.healing.is_some());
            let (dx, dy) = u.route.front().filter(|_| !holding).map_or((0., 0.), |&h| {
                let p = point(h);
                (p.0 - u.x, p.1 - u.y)
            });
            let length = dx.hypot(dy);
            let speed = u.piece.kind.stats().speed.min((2. * ACCEL * length).sqrt());
            let desired = if length > 0.001 {
                (dx / length * speed, dy / length * speed)
            } else {
                (0., 0.)
            };
            let delta = (desired.0 - u.vx, desired.1 - u.vy);
            let amount = delta.0.hypot(delta.1);
            let ratio = if amount > 0. {
                (ACCEL * dt / amount).min(1.)
            } else {
                0.
            };
            let velocity = (u.vx + delta.0 * ratio, u.vy + delta.1 * ratio);
            let p = (u.x + velocity.0 * dt, u.y + velocity.1 * dt);
            let q = p.0 * 2. / ROOT3;
            let r = p.1 - q / 2.;
            if q.abs().max(r.abs()).max((q + r).abs()) > RADIUS as f64 + 0.01 {
                proposed.push((u.pos(), (0., 0.)));
            } else {
                proposed.push((p, velocity));
            }
        }
        // Evaluate traffic against one snapshot; no submission-order or iteration winner.
        let mut blocked = vec![false; snapshot.len()];
        for i in 0..snapshot.len() {
            for j in i + 1..snapshot.len() {
                let (a, b) = (&snapshot[i], &snapshot[j]);
                let vault = a.piece.side == b.piece.side
                    && (a.vaults(b) || b.vaults(a))
                    && a.route.front() != b.route.front();
                if !vault
                    && distance(proposed[i].0, proposed[j].0) < SPACING
                    && distance(proposed[i].0, proposed[j].0)
                        < distance(a.pos(), b.pos()) - 0.000001
                {
                    // A pursuer must not freeze a unit that is trying to withdraw.
                    // Stop the approaching bodies, not the ones moving away.
                    let before = distance(a.pos(), b.pos());
                    let closing_a = distance(proposed[i].0, b.pos()) < before - 0.000001;
                    let closing_b = distance(proposed[j].0, a.pos()) < before - 0.000001;
                    blocked[i] |= closing_a || !closing_b;
                    blocked[j] |= closing_b || !closing_a;
                }
            }
        }
        for (i, u) in self.units.iter_mut().enumerate() {
            if blocked[i] {
                u.vx = 0.;
                u.vy = 0.;
                continue;
            }
            (u.x, u.y) = proposed[i].0;
            (u.vx, u.vy) = proposed[i].1;
            if let Some(&next) = u.route.front()
                && distance(u.pos(), point(next)) < 0.022
            {
                if snapshot
                    .iter()
                    .any(|other| other.id != u.id && other.anchor == next)
                {
                    u.vx = 0.;
                    u.vy = 0.;
                    continue;
                }
                let old = u.anchor;
                u.anchor = next;
                (u.x, u.y) = point(next);
                u.route.pop_front();
                self.last_move = Some(Move {
                    from: old,
                    to: next,
                    promotion: None,
                });
                if u.piece.kind == Kind::Pawn {
                    self.last_progress = self.now;
                    if !next.add(u.piece.side.forward()).on_board() {
                        // Preserve proportional health: promotion isn't a healing exploit.
                        let fraction = u.health / u.piece.kind.stats().health;
                        u.piece.kind = u.promotion;
                        u.health = fraction * u.piece.kind.stats().health;
                        self.moves.push(format!(
                            "{} promoted at {}",
                            u.piece.side.name(),
                            next.label()
                        ));
                    }
                }
                if u.route.is_empty() {
                    u.goal = None;
                }
            }
        }
        let snapshot = self.units.clone();
        let mut change = vec![0.; snapshot.len()];
        for (i, u) in snapshot.iter().enumerate() {
            let stats = u.piece.kind.stats();
            let focused = u
                .goal
                .and_then(|h| {
                    snapshot
                        .iter()
                        .find(|v| v.anchor == h && v.piece.side != u.piece.side)
                })
                .map(|v| v.id);
            let target = snapshot
                .iter()
                .enumerate()
                .filter(|(_, v)| {
                    v.piece.side != u.piece.side
                        && distance(u.pos(), v.pos()) <= stats.range
                        && Self::visible(&snapshot, u, v)
                })
                .min_by(|(_, a), (_, b)| {
                    let priority = |v: &Unit| if Some(v.id) == focused { 0 } else { 1 };
                    priority(a).cmp(&priority(b)).then_with(|| {
                        distance(u.pos(), a.pos()).total_cmp(&distance(u.pos(), b.pos()))
                    })
                });
            self.units[i].target = target.map(|(_, v)| v.id);
            if let Some((j, _)) = target {
                change[j] -= stats.damage * dt;
            }
            self.units[i].healing = None;
            if u.piece.kind == Kind::Bishop {
                let ally = snapshot
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| {
                        v.id != u.id
                            && v.piece.side == u.piece.side
                            && v.health < v.piece.kind.stats().health
                            && distance(u.pos(), v.pos()) <= 2.1
                    })
                    .min_by(|(_, a), (_, b)| {
                        (a.health / a.piece.kind.stats().health)
                            .total_cmp(&(b.health / b.piece.kind.stats().health))
                    });
                if let Some((j, v)) = ally {
                    change[j] += 4. * dt;
                    self.units[i].healing = Some(v.id);
                }
            }
        }
        for (i, u) in self.units.iter_mut().enumerate() {
            u.health = (u.health + change[i]).clamp(0., u.piece.kind.stats().health);
        }
        let mut deaths = vec![];
        self.units.retain(|u| {
            if u.health <= 0. {
                deaths.push((u.piece, u.anchor));
                false
            } else {
                true
            }
        });
        for (p, h) in deaths {
            self.losses[index(p.side)] += 1;
            self.last_progress = self.now;
            self.moves.push(format!(
                "{} {} defeated at {}",
                p.side.name(),
                p.kind.name(),
                h.label()
            ));
        }
        self.position.pieces = self.units.iter().map(|u| (u.anchor, u.piece)).collect();
    }
    fn visible(units: &[Unit], a: &Unit, b: &Unit) -> bool {
        if !matches!(a.piece.kind, Kind::Rook | Kind::Queen) {
            return true;
        }
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx * dx + dy * dy;
        !units.iter().any(|v| {
            if v.id == a.id || v.id == b.id {
                return false;
            }
            let t = ((v.x - a.x) * dx + (v.y - a.y) * dy) / len.max(0.000001);
            t > 0.05 && t < 0.95 && distance(v.pos(), (a.x + t * dx, a.y + t * dy)) < 0.3
        })
    }
    fn refresh_outcome(&mut self) {
        let has = |s| {
            self.units
                .iter()
                .any(|u| u.piece.side == s && u.piece.kind == Kind::King)
        };
        self.outcome = match (has(Side::White), has(Side::Black)) {
            (false, false) => Outcome::Draw("Both kings defeated simultaneously"),
            (false, true) => Outcome::KingDefeated(Side::Black),
            (true, false) => Outcome::KingDefeated(Side::White),
            _ if self.now >= self.last_progress + QUIET => {
                Outcome::Draw("3 minutes without a casualty or pawn advance")
            }
            _ => Outcome::Playing,
        };
    }
    pub fn resign(&mut self, side: Side) {
        if self.outcome == Outcome::Playing {
            self.outcome = Outcome::Resigned(side.other());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn battle(extra: &[(i32, i32, Side, Kind)]) -> Game {
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
    fn state(g: &Game) -> Vec<(u64, Hex, f64, f64, f64)> {
        g.units()
            .iter()
            .map(|u| (u.id, u.anchor, u.x, u.y, u.health))
            .collect()
    }
    #[test]
    fn every_army_piece_can_receive_orders_without_waiting() {
        let mut g = Game::default();
        assert_eq!(g.submit(mv((0, 4), (0, 0))), Err(Rejection::Countdown));
        g.advance(START_DELAY);
        for side in [Side::White, Side::Black] {
            g.attack_army(side);
        }
        g.advance(20);
        assert!(g.orders().len() >= 18);
        let moving = g.units().iter().filter(|u| u.vx.hypot(u.vy) > 0.).count();
        assert!(moving >= 18);
    }
    #[test]
    fn movement_is_continuous_and_long_orders_persist() {
        let mut g = battle(&[(0, 3, Side::White, Kind::Pawn)]);
        g.submit(mv((0, 3), (0, 0))).unwrap();
        g.advance(400);
        let u = g.unit_at(Hex::new(0, 3)).unwrap();
        assert!(u.y < 3. && u.y > 2.);
        assert!(u.marching());
        g.advance(5000);
        assert!(g.unit_at(Hex::new(0, 0)).is_some());
    }
    #[test]
    fn repeated_orders_never_increase_speed_or_damage() {
        let extra = [
            (0, 0, Side::White, Kind::Knight),
            (1, 0, Side::Black, Kind::Pawn),
        ];
        let mut a = battle(&extra);
        let mut b = battle(&extra);
        let id = a.unit_at(Hex::new(0, 0)).unwrap().id;
        a.submit(mv((0, 0), (1, 0))).unwrap();
        b.submit(mv((0, 0), (1, 0))).unwrap();
        for _ in 0..100 {
            let from = b.hex_of(id).unwrap();
            if from != Hex::new(1, 0) {
                for _ in 0..10 {
                    b.submit(Move {
                        from,
                        to: Hex::new(1, 0),
                        promotion: None,
                    })
                    .unwrap();
                }
            }
            a.advance(20);
            b.advance(20);
        }
        assert_eq!(state(&a), state(&b));
        assert!(a.unit_at(Hex::new(1, 0)).unwrap().health < 80.);
    }
    #[test]
    fn redirect_and_stop_preserve_momentum_and_do_not_heal() {
        let mut g = battle(&[(0, 0, Side::White, Kind::Knight)]);
        g.submit(mv((0, 0), (0, -3))).unwrap();
        g.advance(200);
        let i = g
            .units
            .iter()
            .position(|u| u.anchor == Hex::new(0, 0))
            .unwrap();
        let before = (
            g.units[i].vx,
            g.units[i].vy,
            g.units[i].health,
            g.units[i].pos(),
        );
        g.submit(mv((0, 0), (0, 3))).unwrap();
        assert_eq!(
            before,
            (
                g.units[i].vx,
                g.units[i].vy,
                g.units[i].health,
                g.units[i].pos()
            )
        );
        g.stop(Hex::new(0, 0));
        assert_eq!(
            before,
            (
                g.units[i].vx,
                g.units[i].vy,
                g.units[i].health,
                g.units[i].pos()
            )
        );
        g.advance(500);
        assert!(g.units[i].vx.hypot(g.units[i].vy) < 0.001);
    }
    #[test]
    fn idle_pieces_fight_without_any_orders_and_damage_is_time_based() {
        let mut g = battle(&[
            (0, 0, Side::White, Kind::Pawn),
            (1, 0, Side::Black, Kind::Pawn),
        ]);
        g.advance(1000);
        for h in [Hex::new(0, 0), Hex::new(1, 0)] {
            assert!((g.unit_at(h).unwrap().health - 62.).abs() < 0.001);
        }
        assert!(g.orders().is_empty());
        g.advance(4000);
        assert_eq!(g.captured_count(Side::White), 1);
        assert_eq!(g.captured_count(Side::Black), 1);
    }
    #[test]
    fn ranged_fire_is_blocked_by_bodies() {
        let extra = [
            (0, 2, Side::White, Kind::Rook),
            (0, 1, Side::White, Kind::Pawn),
            (0, 0, Side::Black, Kind::Pawn),
        ];
        let mut g = battle(&extra);
        g.advance(1000);
        assert!((g.unit_at(Hex::new(0, 0)).unwrap().health - 62.).abs() < 0.001);
        assert!(g.unit_at(Hex::new(0, 2)).unwrap().target.is_none());
        let mut clear = battle(&[
            (0, 2, Side::White, Kind::Rook),
            (0, 0, Side::Black, Kind::Pawn),
        ]);
        clear.advance(1000);
        assert!((clear.unit_at(Hex::new(0, 0)).unwrap().health - 58.).abs() < 0.001);
    }
    #[test]
    fn bishops_heal_over_time_without_clicks() {
        let mut g = battle(&[
            (0, 0, Side::White, Kind::Pawn),
            (-1, 0, Side::White, Kind::Bishop),
        ]);
        g.units
            .iter_mut()
            .find(|u| u.anchor == Hex::new(0, 0))
            .unwrap()
            .health = 40.;
        g.advance(1000);
        assert!((g.unit_at(Hex::new(0, 0)).unwrap().health - 44.).abs() < 0.001);
        assert!(g.unit_at(Hex::new(-1, 0)).unwrap().healing.is_some());
    }
    #[test]
    fn paths_route_around_friends_and_pawns_cannot_retreat() {
        let g = battle(&[
            (0, 2, Side::White, Kind::Rook),
            (0, 1, Side::White, Kind::Pawn),
        ]);
        let route = Game::path(&g.routes(Hex::new(0, 2)), Hex::new(0, 2), Hex::new(0, -2)).unwrap();
        assert!(!route.contains(&Hex::new(0, 1)));
        assert!(
            g.legal_from(Hex::new(0, 1))
                .iter()
                .all(|m| 2 * m.to.r + m.to.q < 2)
        );
    }
    #[test]
    fn simultaneous_damage_can_defeat_both_kings_and_freezes_outcome() {
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
        g.advance(START_DELAY + 12000);
        assert_eq!(
            g.outcome,
            Outcome::Draw("Both kings defeated simultaneously")
        );
        let s = state(&g);
        let now = g.now();
        g.advance(10000);
        assert_eq!(s, state(&g));
        assert_eq!(now, g.now());
    }
    #[test]
    fn fixed_step_results_do_not_depend_on_frame_chunking() {
        let mut a = Game::default();
        let mut b = Game::default();
        a.advance(START_DELAY);
        b.advance(START_DELAY);
        for side in [Side::White, Side::Black] {
            a.attack_army(side);
            b.attack_army(side);
        }
        a.advance(12345);
        for _ in 0..823 {
            b.advance(15);
        }
        assert_eq!(state(&a), state(&b));
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.moves, b.moves);
    }
    #[test]
    fn full_battle_keeps_identity_anchor_and_health_consistent() {
        let mut g = Game::default();
        g.advance(START_DELAY);
        for side in [Side::White, Side::Black] {
            g.attack_army(side);
        }
        for _ in 0..600 {
            g.advance(100);
            assert_eq!(g.position.pieces.len(), g.units.len());
            for u in g.units() {
                assert!(u.anchor.on_board());
                assert!(u.health > 0. && u.health <= u.piece.kind.stats().health);
                assert!(u.x.is_finite() && u.y.is_finite());
            }
            assert_eq!(g.units.len() + g.losses.iter().sum::<usize>(), 36);
        }
        assert!(
            !g.moves.is_empty(),
            "autonomous armies must actually engage"
        );
    }
    #[test]
    fn quiet_draw_and_invalid_orders_are_non_destructive() {
        let mut g = battle(&[(0, 3, Side::White, Kind::Pawn)]);
        let before = state(&g);
        assert_eq!(g.submit(mv((0, 3), (0, 4))), Err(Rejection::Illegal));
        assert_eq!(g.submit(mv((0, 3), (20, 20))), Err(Rejection::Illegal));
        assert_eq!(state(&g), before);
        g.advance(QUIET - 20);
        assert_eq!(g.outcome, Outcome::Playing);
        g.advance(20);
        assert_eq!(
            g.outcome,
            Outcome::Draw("3 minutes without a casualty or pawn advance")
        );
    }
    #[test]
    fn contact_does_not_pin_a_retreating_unit() {
        let mut g = battle(&[
            (0, 0, Side::White, Kind::Knight),
            (1, 0, Side::Black, Kind::Queen),
        ]);
        g.submit(mv((0, 0), (1, 0))).unwrap();
        g.submit(mv((1, 0), (3, 0))).unwrap();
        let n = g
            .units
            .iter_mut()
            .find(|u| u.piece.kind == Kind::Knight)
            .unwrap();
        n.vx = 1.5;
        let q = g
            .units
            .iter_mut()
            .find(|u| u.piece.kind == Kind::Queen)
            .unwrap();
        q.x = SPACING;
        q.y = 0.;
        q.vx = 0.85;
        g.advance(STEP);
        assert!(
            g.units
                .iter()
                .find(|u| u.piece.kind == Kind::Queen)
                .unwrap()
                .x
                > SPACING
        );
    }
    #[test]
    fn advance_engages_and_holds_but_manual_orders_can_withdraw() {
        let mut g = battle(&[
            (0, 0, Side::White, Kind::Queen),
            (0, -2, Side::Black, Kind::Pawn),
        ]);
        let id = g.unit_at(Hex::new(0, 0)).unwrap().id;
        g.attack_army(Side::White);
        g.advance(500);
        let u = g.units.iter().find(|u| u.id == id).unwrap();
        assert!(!u.marching());
        assert!(u.target.is_some());
        let anchor = u.anchor;
        g.submit(Move {
            from: anchor,
            to: Hex::new(0, 2),
            promotion: None,
        })
        .unwrap();
        g.advance(400);
        assert!(g.units.iter().find(|u| u.id == id).unwrap().marching());
    }
    #[test]
    fn promotion_preserves_health_fraction_and_resignation_freezes() {
        let mut g = battle(&[(-2, -2, Side::White, Kind::Pawn)]);
        g.units
            .iter_mut()
            .find(|u| u.anchor == Hex::new(-2, -2))
            .unwrap()
            .health = 40.;
        g.submit(Move {
            from: Hex::new(-2, -2),
            to: Hex::new(-2, -3),
            promotion: Some(Kind::Knight),
        })
        .unwrap();
        g.advance(2000);
        let u = g.unit_at(Hex::new(-2, -3)).unwrap();
        assert_eq!(u.piece.kind, Kind::Knight);
        assert_eq!(u.health, 55.);
        g.resign(Side::Black);
        let before = state(&g);
        g.advance(10000);
        assert_eq!(state(&g), before);
    }
}
