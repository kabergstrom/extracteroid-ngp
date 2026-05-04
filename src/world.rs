use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::{PI, TAU};
use core::fmt;

use glam::Vec2;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const GAME_X: f32 = 1600.;
pub const GAME_Y: f32 = 900.;

pub const ASTEROID_SPEED: f32 = 145.;
pub const ASTEROID_SPEED_SHATTER: f32 = 180.;
const ASTEROID_SPAWN_IMMUNITY_DURATION: f64 = 0.125;
const ASTEROID_HIT_IMMUNITY_DURATION: f64 = 1. / 30.;
const ASTEROID_DBL_HIT_IMMUNITY_DURATION: f64 = 1. / 15.;

const BULLET_LIFETIME: f64 = 10.;

const FUEL_WEIGHT: f32 = 0.1;
pub const FUEL_BURN_RATE: f32 = 100. / 60.;
pub const FUEL_COST: f64 = 0.2;

pub const DEBT_INTEREST: f64 = 0.1;

pub const RADAR_DIST: f32 = 300.;

pub const ZOOM_FACTOR: f32 = 1.6;

pub const ASTEROID_LOOT: &str = "AsteroidLoot";
pub const GEAR_LOOT: &str = "GearLoot";
pub const SHIP_LOOT: &str = "ShipLoot";
pub const WEAPON_LOOT: &str = "WeaponLoot";
pub const STARTER_TAG: &str = "Starter";
pub const UNKNOWN_STR: &str = "UNKNOWN";

// ---------------------------------------------------------------------------
// XorShift32 PRNG
// ---------------------------------------------------------------------------

pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16777216.0
    }
    pub fn gen_range_i32(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u32;
        min + (self.next_u32() % range) as i32
    }
    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    pub fn gen_range_u32(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        min + self.next_u32() % (max - min)
    }
}

// ---------------------------------------------------------------------------
// ElementID
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Copy, Hash)]
pub struct ElementID(pub u64);

// ---------------------------------------------------------------------------
// Ship types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ShipKind {
    #[default]
    LoanStar,
    BrittleStar,
    Gunship,
    Crabber,
    BattleStar,
    Carrier,
}

/// Per-ship polygon shape, stored as a separate enum so vertex generation
/// is data-driven rather than using function pointers.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ShipShape {
    /// Triangle: nose, bottom-left, bottom-right
    #[default]
    Triangle,
    /// Pentagon: nose, bl, bl*0.6, br*0.6, br
    Star,
    /// Diamond: nose, bl, back-center, br
    Diamond,
    /// Crab: nose, ln, ll, bcl, bcr, rl, rn (wide shape)
    Crab,
    /// TriangleWithTail: main triangle + rear triangle
    TriangleWithTail,
    /// Pentagon2: nose, fl, bl, br, fr
    Pentagon,
}

#[derive(Clone)]
pub struct ShipDef {
    pub name: String,
    pub manufacturer: String,
    pub desc: String,
    pub height: f32,
    pub width: f32,
    pub speed: f32,
    pub extraction_time: f64,
    pub launch_fee: f64,
    pub rental_fee: f64,
    pub frame_weight: f32,
    pub weight_limit: f32,
    pub fuel_capacity: f64,
    pub turn_rate: f32,
    pub recoil_factor: f32,
    pub reload_factor: f32,
    pub recoil_duration: f64,
    pub kind: ShipKind,
    pub shape: ShipShape,
}
impl fmt::Display for ShipDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Drone: {}\nManufacturer: {}\nDimensions: {}x{}\nLaunch fee: {:.2}c\n{}Extraction time: {:.2}s\nWeight (Drone/Cap): {}u/{}u\nFuel capacity: {:.2}kl\nTurn rate: {:.2}rad/s\n(Recoil: {:.2}%, Reload: {:.2}%)\nDescription: {}",
            self.name,
            self.manufacturer,
            self.width,
            self.height,
            self.launch_fee,
            if self.rental_fee > 0. {
                format!("Rental fee: {:.2}c\n", self.rental_fee)
            } else {
                String::new()
            },
            self.extraction_time,
            self.frame_weight,
            self.weight_limit,
            self.fuel_capacity,
            self.turn_rate,
            self.recoil_factor * 100.,
            self.reload_factor * 100.,
            self.desc,
        )
    }
}

#[derive(Clone, Default)]
pub struct TargetingSystem {
    pub aim_lead: f32,
    pub tracking_assist_short: f32,
    pub tracking_assist_mid: f32,
    pub tracking_assist_far: f32,
    pub tracking_rate: f64,
    pub current_target: Option<ElementID>,
    pub tracking_t: Option<f64>,
    pub main_turret_rot: f32,
    pub wing_turret_rot: f32,
}
impl TargetingSystem {
    pub fn new() -> Self {
        Self {
            tracking_rate: 1.,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default)]
pub struct GuidanceSystem {
    pub weapon_diagnostics: bool,
    pub ship_diagnostics: bool,
    pub fuel_efficiency_bonus: f64,
    pub salvage_minimum_factor: f64,
}

#[derive(Clone)]
pub struct ShieldSystem {
    pub shield: f64,
    pub shield_capacity: f64,
    pub shield_regen_rate: f64,
    pub shield_regen_delay: f64,
    pub shield_damage: f32,
    pub sides: u8,
    pub shield_margin: f32,
    pub last_struck_t: f64,
}
impl Default for ShieldSystem {
    fn default() -> Self {
        Self {
            shield: 0.,
            shield_capacity: 0.,
            shield_regen_rate: 0.,
            shield_regen_delay: 1.,
            shield_damage: 1.,
            sides: 8,
            shield_margin: 1.5,
            last_struck_t: 0.,
        }
    }
}

#[derive(Clone)]
pub struct Ship {
    pub id: ElementID,
    pub def: ShipDef,
    pub pos: Vec2,
    pub rot: f32,
    pub vel: Vec2,
    pub fuel: f64,
    pub destroyed_t: Option<f64>,
    pub targeting_system: TargetingSystem,
    pub guidance_system: GuidanceSystem,
    pub shield_system: ShieldSystem,
}

impl Ship {
    pub fn vertex_sets(&self) -> Vec<Vec<Vec2>> {
        ship_vertices(self).0
    }
    pub fn wing_tips(&self, offset: Vec2) -> (Vec2, Vec2) {
        let (_, [pos_l, pos_r]) = ship_vertices(self);
        let rot_vec = Vec2::from_angle(self.rot);
        let offset_vec = Vec2::new((self.rot + PI / 2.).cos(), (self.rot + PI / 2.).sin());
        let offset_x = offset_vec * offset.x;
        let offset_y = rot_vec * offset.y;
        (pos_l - offset_x - offset_y, pos_r + offset_x - offset_y)
    }
    pub fn current_turn_rate(&self, total_weight: f32) -> f32 {
        if total_weight < self.def.weight_limit {
            self.def.turn_rate
        } else if total_weight > self.def.weight_limit * 2. {
            self.def.turn_rate
                * 0.5
                * (self.def.weight_limit / (total_weight - self.def.weight_limit)).powf(2.)
        } else {
            self.def.turn_rate * self.def.weight_limit / total_weight
        }
    }
    pub fn destroyed(&self) -> bool {
        self.destroyed_t.is_some()
    }
    pub fn fuel_used(&self) -> f64 {
        if self.destroyed() {
            self.def.fuel_capacity
        } else {
            self.def.fuel_capacity - self.fuel
        }
    }
    pub fn bounding_radius(&self) -> f32 {
        let all_verts: Vec<Vec2> = self.vertex_sets().into_iter().flatten().collect();
        if all_verts.is_empty() {
            return self.def.height.max(self.def.width) / 2.;
        }
        // Simple bounding: max distance from pos
        let mut max_r = 0.0f32;
        for v in &all_verts {
            let d = (*v - self.pos).length();
            if d > max_r {
                max_r = d;
            }
        }
        max_r
    }
    pub fn outer_shield_radius(&self) -> f32 {
        self.bounding_radius()
            * (1. + self.shield_system.shield_margin * self.shield_system.shield.floor() as f32)
    }
    pub fn shield_vertices(&self, shield_num: i32) -> Vec<Vec2> {
        let pos = self.pos;
        let rot = -PI / 2.;
        let radius =
            self.bounding_radius() * (1. + self.shield_system.shield_margin * shield_num as f32);
        if self.shield_system.sides < 3 {
            let offset = Vec2::new(rot.cos(), rot.sin()) * radius;
            return vec![pos - offset, pos + offset];
        }
        let mut vertices = Vec::with_capacity(self.shield_system.sides as usize);
        let angle_step = TAU / self.shield_system.sides as f32;
        for i in 0..self.shield_system.sides {
            let angle = i as f32 * angle_step + rot + PI / 2.;
            let vertex = Vec2::new(angle.sin(), -angle.cos()) * radius + pos;
            vertices.push(vertex);
        }
        vertices
    }
    pub fn wing_pivot(
        &self,
        pos_l: Vec2,
        pos_r: Vec2,
        fwd_l: f32,
        fwd_r: f32,
        w: &GameWorld,
    ) -> (Vec2, Vec2) {
        if let Some((tar_pos, tar_vel)) = self
            .targeting_system
            .current_target
            .and_then(|id| w.asteroid_from_id_pos_vel(id))
        {
            let delta_angle_l = w.delta_angle_to_target(pos_l, self.rot, TAU, tar_pos, tar_vel);
            let delta_angle_r = w.delta_angle_to_target(pos_r, self.rot, TAU, tar_pos, tar_vel);
            let turn_amount_l = delta_angle_l.clamp(
                -self.targeting_system.wing_turret_rot,
                self.targeting_system.wing_turret_rot,
            );
            let turn_amount_r = delta_angle_r.clamp(
                -self.targeting_system.wing_turret_rot,
                self.targeting_system.wing_turret_rot,
            );
            (
                Vec2::from_angle(fwd_l + turn_amount_l),
                Vec2::from_angle(fwd_r + turn_amount_r),
            )
        } else {
            (Vec2::from_angle(fwd_l), Vec2::from_angle(fwd_r))
        }
    }
}

// ---------------------------------------------------------------------------
// Entity shapes
// ---------------------------------------------------------------------------

#[derive(Clone, Default, Copy, PartialEq)]
pub enum EntityShape {
    #[default]
    Pellet,
    Oblong,
    Line,
    Disc,
}
impl EntityShape {
    pub fn area(&self, length: f32) -> f32 {
        match self {
            EntityShape::Pellet | EntityShape::Disc => PI * (length / 2.).powf(2.),
            EntityShape::Oblong | EntityShape::Line => length * 2.,
        }
    }
}

// ---------------------------------------------------------------------------
// Projectile
// ---------------------------------------------------------------------------

/// How projectile length varies over its lifetime.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum LengthCurve {
    /// Constant length = def.length.
    #[default]
    Constant,
    /// Explosion: grows in age, shrinks over lifetime. Used by rocket/missile.
    Explosion,
}

/// What happens when a projectile hits something.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ImpactKind {
    #[default]
    None,
    Bouncing,
    Flak,
    Rocket,
    Missile,
}

#[derive(Clone)]
pub struct ProjectileDef {
    pub name: String,
    pub damage: f32,
    pub speed: f32,
    pub cost: f64,
    pub length: f32,
    pub length_curve: LengthCurve,
    pub shape: EntityShape,
    pub persists: bool,
    pub duration: f64,
    pub homing_turn_rate: f32,
    pub impact_kind: ImpactKind,
    pub death_impact: bool,
}
impl Default for ProjectileDef {
    fn default() -> Self {
        Self {
            name: UNKNOWN_STR.to_string(),
            damage: 1.,
            speed: 300.,
            cost: 0.,
            length: 1.,
            length_curve: LengthCurve::Constant,
            shape: Default::default(),
            persists: false,
            duration: BULLET_LIFETIME,
            homing_turn_rate: 0.,
            impact_kind: ImpactKind::None,
            death_impact: false,
        }
    }
}

#[derive(Clone)]
pub struct Projectile {
    pub id: ElementID,
    pub pos: Vec2,
    pub vel: Vec2,
    pub dir: Vec2,
    pub spawn_t: f64,
    pub duration: f64,
    pub pending_destroy: bool,
    pub def: ProjectileDef,
    pub target: Option<ElementID>,
    pub parent: Option<ElementID>,
}
impl Projectile {
    pub fn age(&self, frame_t: f64) -> f64 {
        frame_t - self.spawn_t
    }
    pub fn lifetime_remaining(&self, frame_t: f64) -> f64 {
        self.duration - self.age(frame_t)
    }
    pub fn current_length(&self, frame_t: f64) -> f32 {
        match self.def.length_curve {
            LengthCurve::Constant => self.def.length,
            LengthCurve::Explosion => {
                let age_factor = (self.age(frame_t) / 0.1).clamp(0., 1.).powf(2.) as f32;
                let life_factor = (self.lifetime_remaining(frame_t) / self.duration) as f32;
                age_factor * life_factor.powf(2.) * self.def.length
            }
        }
    }
    pub fn current_velocity(&self) -> Vec2 {
        if self.vel == Vec2::ZERO {
            self.dir
        } else {
            self.vel
        }
    }
    pub fn current_direction(&self) -> Vec2 {
        if self.vel.length_squared() > 0. {
            self.vel.normalize()
        } else {
            self.dir
        }
    }
}

// ---------------------------------------------------------------------------
// Particle
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub accel: Vec2,
    pub rot: f32,
    pub rot_vel: f32,
    pub spawn_t: f64,
    pub duration: f64,
    pub fade_out: bool,
    pub length: f32,
    pub growth: f32,
    pub thickness: f32,
    pub shape: EntityShape,
}
impl Default for Particle {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            accel: Vec2::ZERO,
            rot: 0.,
            rot_vel: 0.,
            spawn_t: 0.,
            duration: 0.,
            fade_out: true,
            length: 0.,
            growth: 0.,
            thickness: 0.,
            shape: Default::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Weapon
// ---------------------------------------------------------------------------

/// How the weapon fires projectiles.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ShootKind {
    #[default]
    Default,
    TwinShoot,
    TwinWideShoot,
    ChamberShootThree,
    ChamberShootFive,
}

#[derive(Clone)]
pub struct WeaponDef {
    pub name: String,
    pub desc: String,
    pub projectile_stats: ProjectileDef,
    pub cd: f32,
    pub magazine: i32,
    pub reload: f32,
    pub weight: f32,
    pub effective_range: f32,
    pub rental_fee: f64,
    pub shoot_kind: ShootKind,
}
impl fmt::Display for WeaponDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Weapon: {}\n{}Fire rate: {:.2}/s\nMagazine: {}\nReload: {}s\nWeight: {}u\nAmmo cost: {:.2}c/ammo\nDescription: {}",
            self.name,
            if self.rental_fee > 0. {
                format!("Rental fee: {:.2}c\n", self.rental_fee)
            } else {
                String::new()
            },
            1. / (if self.magazine == 1 {
                self.reload
            } else {
                self.cd
            }),
            self.magazine,
            self.reload,
            self.weight,
            self.projectile_stats.cost,
            self.desc,
        )
    }
}

#[derive(Clone)]
pub struct Weapon {
    pub id: ElementID,
    pub def: WeaponDef,
    pub ammo: i32,
    pub chamber: i32,
    pub last_shot: f64,
    pub last_reload: f64,
    pub shots_fired: i32,
    pub shots_hit: i32,
}
impl Weapon {
    pub fn reloading(&self) -> bool {
        self.ammo == -1
    }
    pub fn ready_to_fire(&self, frame_t: f64) -> bool {
        frame_t - self.last_shot > self.def.cd as f64 && self.ammo > 0
    }
}

// ---------------------------------------------------------------------------
// Loot
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Loot {
    pub id: ElementID,
    pub pos: Vec2,
    pub rot: f32,
    pub radius: f32,
    pub sides: u8,
    pub spawn_t: f64,
    pub expire_t: f64,
    pub frozen_t: Option<f64>,
    pub contained_items: Vec<Item>,
}
impl Loot {
    pub fn lifetime_remaining(&self, frame_t: f64) -> f64 {
        self.frozen_t.unwrap_or_else(|| self.expire_t - frame_t)
    }
}

// ---------------------------------------------------------------------------
// Asteroid
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Asteroid {
    pub id: ElementID,
    pub pos: Vec2,
    pub vel: Vec2,
    pub rot: f32,
    pub rot_speed: f32,
    pub radius: f32,
    pub sides: u8,
    pub altered_vertices: Vec<(u8, f32)>,
    pub spawn_t: f64,
    pub pending_destroy: bool,
    pub armor: f32,
    pub value: f64,
    pub last_struck_t: f64,
    pub last_struck_id: Option<ElementID>,
    pub homing_turn_rate: f32,
}
impl Asteroid {
    pub fn vertices(&self, frame_t: f64) -> Vec<Vec2> {
        self.vertices_internal(self.pos, self.rot, 1., frame_t)
    }
    pub fn vertices_with_scale(&self, scale: f32, frame_t: f64) -> Vec<Vec2> {
        self.vertices_internal(self.pos, self.rot, scale, frame_t)
    }
    pub fn vertices_at_pos_rot(&self, pos: Vec2, rot: f32, frame_t: f64) -> Vec<Vec2> {
        self.vertices_internal(pos, rot, 1., frame_t)
    }
    fn vertices_internal(&self, pos: Vec2, rot: f32, scale: f32, frame_t: f64) -> Vec<Vec2> {
        if self.sides < 3 {
            let offset = Vec2::new(rot.cos(), rot.sin()) * self.e_radius(frame_t);
            return vec![pos - offset, pos + offset];
        }
        let mut vertices = Vec::with_capacity(self.sides as usize);
        let angle_step = TAU / self.sides as f32;
        for i in 0..self.sides {
            let angle = i as f32 * angle_step + rot + PI / 2.;
            let factor = self
                .altered_vertices
                .iter()
                .find(|(idx, _)| *idx == i)
                .map_or(1.0, |(_, f)| *f);
            let vertex =
                Vec2::new(angle.sin(), -angle.cos()) * factor * self.e_radius(frame_t) * scale
                    + pos;
            vertices.push(vertex);
        }
        vertices
    }
    pub fn e_radius(&self, frame_t: f64) -> f32 {
        let t = ((frame_t - self.spawn_t) / 0.1).powf(0.5).clamp(0., 1.) as f32;
        self.radius * t
    }
    pub fn loot_drops(&self, w: &mut GameWorld) -> Vec<Item> {
        let tier = w.mission.as_ref().map_or(1, |m| m.def.mission_tier);
        let capped_tier = (tier.min(2) as usize).max(1) - 1;
        let loot_mult = w.mission.as_ref().map_or(1.0, |m| m.loot_multiplier());
        let base_value = self.value * loot_mult as f64;
        let frac = w.rng.next_f32();
        let value = base_value.floor() as i32 + (frac < base_value.fract() as f32) as i32;

        let base_res_chance = [1110u32, 900];
        let base_comp_chance = [50u32, 98];
        let mut comp_chance = base_comp_chance[capped_tier];
        let mut drone_chance = 15u32;
        let mut weapon_chance = 25u32;
        let mut allow_temporal_drops = true;
        let mut items = Vec::new();

        for i in 0..value {
            if i > 0 {
                comp_chance += 20;
                if allow_temporal_drops {
                    drone_chance += 5;
                    weapon_chance += 10;
                }
            }
            comp_chance =
                (comp_chance as i32 + w.pity_timers.drop_counter_component * 5).max(0) as u32;
            if allow_temporal_drops {
                drone_chance =
                    (drone_chance as i32 + w.pity_timers.drop_counter_drone * 2).max(0) as u32;
                weapon_chance =
                    (weapon_chance as i32 + w.pity_timers.drop_counter_weapon * 3).max(0) as u32;
            }
            let gear_type_roll: Vec<(GearType, u32, [u32; 5])> = vec![
                (
                    GearType::Resource,
                    base_res_chance[capped_tier],
                    match tier {
                        1 => [144, 48, 12, 4, 0],
                        2 => [80, 40, 20, 10, 5],
                        _ => [100, 30, 10, 3, 1],
                    },
                ),
                (
                    GearType::Component,
                    comp_chance,
                    match tier {
                        1 => [95, 35, 15, 5, 0],
                        2 => [80, 40, 20, 10, 5],
                        _ => [100, 30, 10, 3, 1],
                    },
                ),
                (GearType::Drone, drone_chance, [16, 8, 4, 2, 1]),
                (GearType::Weapon, weapon_chance, [16, 8, 4, 2, 1]),
            ];
            let weights: Vec<u32> = gear_type_roll.iter().map(|(_, w, _)| *w).collect();
            let Some(idx) = weighted_random_vec(&weights, &mut w.rng) else {
                return items;
            };
            let (gear_type_rolled, _, rarity_rates) = &gear_type_roll[idx];

            use Rarity::*;
            let has_rarity_or_0 = |rarity: &Rarity| -> u32 {
                if w.def_storage
                    .gear_defs
                    .iter()
                    .any(|g| {
                        g.tags.iter().any(|t| t == ASTEROID_LOOT)
                            && g.rarity == *rarity
                            && g.mission_tiers.as_ref().is_none_or(|mt| mt.contains(&tier))
                    })
                {
                    1
                } else {
                    0
                }
            };
            let rarity_weights: Vec<u32> = vec![
                rarity_rates[0] * has_rarity_or_0(&Common),
                rarity_rates[1] * has_rarity_or_0(&Uncommon),
                rarity_rates[2] * has_rarity_or_0(&Rare),
                rarity_rates[3] * has_rarity_or_0(&Epic),
                rarity_rates[4] * has_rarity_or_0(&Legendary),
            ];
            let rarities = [Common, Uncommon, Rare, Epic, Legendary];
            let Some(r_idx) = weighted_random_vec(&rarity_weights, &mut w.rng) else {
                return items;
            };
            let rarity_rolled = &rarities[r_idx];

            let pool: Vec<usize> = w
                .def_storage
                .gear_defs
                .iter()
                .enumerate()
                .filter(|(_, g)| {
                    g.tags.iter().any(|t| t == ASTEROID_LOOT)
                        && g.gear_type == *gear_type_rolled
                        && g.rarity == *rarity_rolled
                        && g.mission_tiers.as_ref().is_none_or(|mt| mt.contains(&tier))
                })
                .map(|(i, _)| i)
                .collect();
            let pool_weights: Vec<u32> = pool
                .iter()
                .map(|&i| w.def_storage.gear_defs[i].drop_rate)
                .collect();
            if let Some(p_idx) = weighted_random_vec(&pool_weights, &mut w.rng) {
                let gear_idx = pool[p_idx];
                let gear_def = &w.def_storage.gear_defs[gear_idx];
                let name = gear_def.name.clone();
                let name_hidden = gear_def.equippable();
                items.push(Item {
                    name,
                    count: 1,
                    name_hidden,
                    ..Default::default()
                });
                w.pity_timers
                    .increment_counters_except(gear_def.gear_type.clone());
                w.pity_timers
                    .reset_counter_of(gear_def.gear_type.clone());
                match gear_def.gear_type {
                    GearType::Resource => (),
                    GearType::Component => comp_chance = 0,
                    GearType::Drone | GearType::Weapon => {
                        drone_chance = 0;
                        weapon_chance = 0;
                        allow_temporal_drops = false;
                    }
                }
            }
        }
        items
    }
}

fn weighted_random_vec(weights: &[u32], rng: &mut Rng) -> Option<usize> {
    if weights.is_empty() {
        return None;
    }
    let total: u32 = weights.iter().sum();
    if total != 0 {
        let rng_val = rng.gen_range_u32(0, total);
        let mut counter = 0u32;
        for (idx, &w) in weights.iter().enumerate() {
            if counter + w > rng_val {
                return Some(idx);
            }
            counter += w;
        }
    }
    Some(rng.gen_range_u32(0, weights.len() as u32) as usize)
}

// ---------------------------------------------------------------------------
// Collision
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
pub struct CollisionInfo {
    pub normal: Vec2,
    pub depth: f32,
}

pub fn check_convex_polygon_collision(poly1: &[Vec2], poly2: &[Vec2]) -> Option<CollisionInfo> {
    let mut axes = Vec::new();
    for i in 0..poly1.len() {
        let p1 = poly1[i];
        let p2 = poly1[(i + 1) % poly1.len()];
        let edge = p2 - p1;
        if edge.length_squared() > 0.0 {
            axes.push(edge.perp().normalize());
        }
    }
    for i in 0..poly2.len() {
        let p1 = poly2[i];
        let p2 = poly2[(i + 1) % poly2.len()];
        let edge = p2 - p1;
        if edge.length_squared() > 0.0 {
            axes.push(edge.perp().normalize());
        }
    }
    let mut min_overlap = f32::MAX;
    let mut smallest_axis = Vec2::ZERO;
    for axis in &axes {
        let (mut min1, mut max1) = (f32::MAX, f32::MIN);
        for &vertex in poly1 {
            let projection = vertex.dot(*axis);
            min1 = min1.min(projection);
            max1 = max1.max(projection);
        }
        let (mut min2, mut max2) = (f32::MAX, f32::MIN);
        for &vertex in poly2 {
            let projection = vertex.dot(*axis);
            min2 = min2.min(projection);
            max2 = max2.max(projection);
        }
        if max1 < min2 || max2 < min1 {
            return None;
        }
        let overlap = (max1 - min2).min(max2 - min1);
        if overlap < min_overlap {
            min_overlap = overlap;
            smallest_axis = *axis;
        }
    }
    if poly1.is_empty() || poly2.is_empty() {
        return None;
    }
    let center1: Vec2 = poly1.iter().copied().sum::<Vec2>() / poly1.len() as f32;
    let center2: Vec2 = poly2.iter().copied().sum::<Vec2>() / poly2.len() as f32;
    let direction = center2 - center1;
    if smallest_axis.dot(direction) > 0.0 {
        smallest_axis = -smallest_axis;
    }
    Some(CollisionInfo {
        normal: smallest_axis,
        depth: min_overlap,
    })
}

/// For convex regular polygons (asteroids), SAT directly. No triangulation needed.
pub fn check_polygon_collision(poly1: &[Vec2], poly2: &[Vec2]) -> Option<CollisionInfo> {
    check_convex_polygon_collision(poly1, poly2)
}

// ---------------------------------------------------------------------------
// Item / Gear / Mission / Expense
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct Item {
    pub name: String,
    pub count: i32,
    pub pending_sale: bool,
    pub pending_destroy: bool,
    pub name_hidden: bool,
    pub is_active: bool,
    pub is_rental: bool,
}
impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}x: {}",
            self.count,
            if self.name_hidden {
                UNKNOWN_STR
            } else {
                self.name.as_str()
            }
        )
    }
}
impl Item {
    pub fn available(&self) -> bool {
        !self.name_hidden && !self.pending_destroy && !self.pending_sale
    }
}

#[derive(Clone, Default, PartialEq)]
pub enum Rarity {
    Starter,
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum GearType {
    Resource,
    #[default]
    Component,
    Drone,
    Weapon,
}

/// Gear effects applied on equip and reversed on unequip.
#[derive(Clone, Default)]
pub enum GearEffect {
    /// No effect (resources, weapons without special equip behavior).
    #[default]
    None,
    /// Multiply projectile speed (or length if speed <= 1) by factor.
    ProjectileSpeed(f32),
    /// Multiply weapon reload time by factor.
    ReloadRate(f32),
    /// Multiply reload by f1 and magazine by f2 (i32 multiplier).
    ExtendedMagazine(f32, i32),
    /// Multiply zoom_factor by factor.
    Zoom(f32),
    /// Add to aim_lead.
    AimLead(f32),
    /// Add to shield count.
    Shield(f32),
    /// Shield generator: +shield, +capacity, +regen_rate, *regen_delay.
    ShieldGenerator {
        shield: f32,
        capacity: f32,
        regen_rate: f64,
        delay_mul: f64,
    },
    /// Set ship_diagnostics flag.
    ShipDiagnostics,
    /// Set weapon_diagnostics flag.
    WeaponDiagnostics,
    /// Add to fuel_efficiency_bonus.
    FuelEfficiency(f32),
    /// Divide extraction_time by factor (speeds up extraction).
    ExtractionSpeed(f64),
    /// Compound salvage minimum factor.
    Insurance(f32),
    /// Multiply projectile damage by factor.
    Damage(f32),
    /// Tracking assist: [short, mid, far] factors + tracking_rate divisor.
    TrackingAssist([f32; 3], f32),
    /// Set weapon last_reload to current frame_t (railgun charge).
    RailgunCharge,
}

#[derive(Clone)]
pub struct GearDef {
    pub name: String,
    pub desc: String,
    pub effect: GearEffect,
    pub value: f64,
    pub weight: f32,
    pub drop_rate: u32,
    pub instasell: bool,
    pub tags: Vec<String>,
    pub stackable: bool,
    pub indestructible: bool,
    pub rarity: Rarity,
    pub gear_type: GearType,
    pub mission_tiers: Option<Vec<u8>>,
}
impl Default for GearDef {
    fn default() -> Self {
        Self {
            name: Default::default(),
            desc: Default::default(),
            effect: GearEffect::None,
            value: 0.,
            weight: 0.,
            drop_rate: 0,
            instasell: false,
            tags: Default::default(),
            stackable: false,
            indestructible: false,
            rarity: Default::default(),
            gear_type: Default::default(),
            mission_tiers: None,
        }
    }
}
impl GearDef {
    pub fn equippable(&self) -> bool {
        match self.gear_type {
            GearType::Resource => false,
            GearType::Component | GearType::Drone | GearType::Weapon => true,
        }
    }
    pub fn hidden_name(&self) -> String {
        format!("{} (?.??c {:.2}u)", UNKNOWN_STR, self.weight)
    }
}

#[derive(Clone, Default)]
pub struct Expense {
    pub name: String,
    pub count: i32,
    pub cost: f64,
}
impl fmt::Display for Expense {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total_cost = self.count as f64 * self.cost;
        write!(
            f,
            "\u{2022}{}{:.2}c ({}{} {:.2}c)",
            if total_cost > 0. {
                "-"
            } else if total_cost == 0. {
                "\u{00b1}"
            } else {
                "+"
            },
            total_cost.abs(),
            if self.count > 1 {
                format!("{}x ", self.count)
            } else {
                String::new()
            },
            self.name,
            self.cost
        )
    }
}

/// Which wave spawning logic to use.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum WaveKind {
    #[default]
    Starter,
    Mining,
    Bounty,
    Belt,
    Barren,
    Meteor,
}

#[derive(Clone)]
pub struct MissionDef {
    pub name: String,
    pub desc: String,
    pub extraction_time: f64,
    pub mission_fee: f64,
    pub initial_sides: i32,
    pub growth: f32,
    pub wave_kind: WaveKind,
    pub spawn_interval: f64,
    pub max_waves: i32,
    pub initial_mult: f32,
    pub mult_growth: f32,
    pub mission_tier: u8,
    pub tags: Vec<String>,
}
impl fmt::Display for MissionDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\n{}\nMission Fee: {}\nExtraction Delay: {:.2}s\nInitial Danger: {} (+{:.1}%/wave)\nSpawn Interval: {}s\nTotal Waves: {}\nLoot multiplier: {:.2}x (+{:.2}x/wave)\nMission Tier: {}",
            self.name,
            self.desc,
            if self.mission_fee > 0. {
                format!("{:.2}c", self.mission_fee)
            } else {
                "Free!".to_string()
            },
            self.extraction_time,
            self.initial_sides,
            (self.growth - 1.) * 100.,
            self.spawn_interval,
            self.max_waves,
            self.initial_mult,
            self.mult_growth,
            self.mission_tier
        )
    }
}

#[derive(Clone)]
pub struct Mission {
    pub id: ElementID,
    pub def: MissionDef,
    pub current_wave: i32,
    pub mission_start_t: f64,
}
impl Mission {
    pub fn total_sides(&self) -> i32 {
        let mut sides = self.def.initial_sides as f32;
        for _ in 1..self.current_wave {
            sides *= self.def.growth;
        }
        sides as i32
    }
    pub fn loot_multiplier(&self) -> f32 {
        self.def.initial_mult + self.def.mult_growth * (self.current_wave - 1).max(0) as f32
    }
}

#[derive(Clone)]
pub struct MissionScanner {
    pub def: MissionDef,
    pub time_to_accept: Option<f64>,
    pub last_scan_t: f64,
    pub scanning: bool,
    pub scan_conclusion_t: f64,
    pub current_scan_t: f64,
}
impl MissionScanner {
    pub fn new(def: MissionDef, frame_t: f64) -> Self {
        Self {
            def,
            time_to_accept: None,
            last_scan_t: frame_t,
            scanning: false,
            scan_conclusion_t: frame_t,
            current_scan_t: frame_t,
        }
    }
    pub fn limited_time(def: MissionDef, time_to_accept: f64, frame_t: f64) -> Self {
        Self {
            def,
            time_to_accept: Some(time_to_accept),
            last_scan_t: frame_t,
            scanning: false,
            scan_conclusion_t: frame_t,
            current_scan_t: frame_t,
        }
    }
}

// ---------------------------------------------------------------------------
// DefStorage
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct DefStorage {
    pub ship_defs: Vec<ShipDef>,
    pub weapon_defs: Vec<WeaponDef>,
    pub gear_defs: Vec<GearDef>,
    pub mission_defs: Vec<MissionDef>,
}
impl DefStorage {
    pub fn all_ship_names(&self) -> Vec<String> {
        self.ship_defs.iter().map(|d| d.name.clone()).collect()
    }
    pub fn all_weapon_names(&self) -> Vec<String> {
        self.weapon_defs.iter().map(|d| d.name.clone()).collect()
    }
}

// ---------------------------------------------------------------------------
// Pity timers
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PityTimers {
    pub drop_counter_component: i32,
    pub drop_counter_drone: i32,
    pub drop_counter_weapon: i32,
}
impl Default for PityTimers {
    fn default() -> Self {
        Self {
            drop_counter_component: -5,
            drop_counter_drone: -10,
            drop_counter_weapon: -10,
        }
    }
}
impl PityTimers {
    pub fn increment_counters_except(&mut self, gear_type: GearType) {
        if gear_type != GearType::Component {
            self.drop_counter_component += 1;
        }
        if gear_type != GearType::Drone {
            self.drop_counter_drone += 1;
        }
        if gear_type != GearType::Weapon {
            self.drop_counter_weapon += 1;
        }
    }
    pub fn reset_counter_of(&mut self, gear_type: GearType) {
        match gear_type {
            GearType::Resource => {}
            GearType::Component => self.drop_counter_component = Self::default().drop_counter_component,
            GearType::Drone => self.drop_counter_drone = Self::default().drop_counter_drone,
            GearType::Weapon => self.drop_counter_weapon = Self::default().drop_counter_weapon,
        }
    }
    pub fn reset_counters(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// Duration (extraction timing)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
pub struct Duration {
    pub start_t: f64,
    pub end_t: f64,
}

// ---------------------------------------------------------------------------
// DarkMeta (persistence state - in-memory only for now)
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct DarkMeta {
    pub balance: f64,
    pub gains: f64,
    pub losses: f64,
    pub rent: f64,
    pub day_rent_due: i32,
    pub current_day: i32,
    pub cargo_hold: Vec<Item>,
    pub stash: Vec<Item>,
    pub cfg_weapon_choice: String,
    pub cfg_ship_choice: String,
}

// ---------------------------------------------------------------------------
// GameWorld
// ---------------------------------------------------------------------------

pub struct GameWorld {
    pub rng: Rng,
    pub def_storage: DefStorage,
    pub current_id: u64,
    pub metaprog: DarkMeta,
    pub frame_t: f64,
    pub prev_frame_t: f64,
    pub zoom_factor: f32,
    pub screen_center: Vec2,
    pub ship: Ship,
    pub main_weapon: Option<Weapon>,
    pub asteroids: Vec<Asteroid>,
    pub bullets: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub loot: Vec<Loot>,
    pub last_spawn_t: f64,
    pub mission_scanner: MissionScanner,
    pub mission: Option<Mission>,
    pub expenses: Vec<Expense>,
    pub extraction_t: Option<Duration>,
    pub new_game: bool,
    pub pity_timers: PityTimers,
    pub deploy_triggered: bool,
    // Cargo CRT screen state
    pub cargo_scroll_offset: i32,
    pub cargo_selected_idx: usize,
    // Game over / static noise
    pub force_static_t: Option<f64>,
    pub game_over_reason: u8, // 0=none, 1=destroyed, 2=rent_default
    pub game_over_t: f64,
    pub game_over_locale_idx: u8,
}

impl GameWorld {
    pub fn new(seed: u32, def_storage: DefStorage, ship_def: ShipDef, mission_scanner: MissionScanner) -> Self {
        let id = 1u64;
        let fuel = ship_def.fuel_capacity;
        Self {
            rng: Rng::new(seed),
            current_id: id,
            metaprog: DarkMeta::default(),
            frame_t: 0.,
            prev_frame_t: 0.,
            zoom_factor: ZOOM_FACTOR,
            screen_center: Vec2::ZERO,
            ship: Ship {
                id: ElementID(id),
                def: ship_def,
                pos: Vec2::ZERO,
                rot: -PI / 2.,
                vel: Vec2::ZERO,
                fuel,
                destroyed_t: None,
                targeting_system: TargetingSystem::new(),
                guidance_system: GuidanceSystem::default(),
                shield_system: ShieldSystem::default(),
            },
            main_weapon: None,
            asteroids: Vec::new(),
            bullets: Vec::new(),
            particles: Vec::new(),
            loot: Vec::new(),
            last_spawn_t: 0.,
            mission_scanner,
            mission: None,
            expenses: Vec::new(),
            extraction_t: None,
            new_game: true,
            pity_timers: PityTimers::default(),
            deploy_triggered: false,
            cargo_scroll_offset: 0,
            cargo_selected_idx: 0,
            force_static_t: None,
            game_over_reason: 0,
            game_over_t: 0.,
            game_over_locale_idx: 0,
            def_storage,
        }
    }

    pub fn gen_id(&mut self) -> ElementID {
        self.current_id += 1;
        ElementID(self.current_id)
    }

    pub fn center(&self) -> Vec2 {
        Vec2::ZERO
    }

    pub fn dt(&self) -> f32 {
        (self.frame_t - self.prev_frame_t) as f32
    }

    pub fn update_frame_t(&mut self, t: f64) {
        self.prev_frame_t = self.frame_t;
        self.frame_t = t;
    }

    // --- Def lookups ---

    pub fn weapon_def_by_name(&self, name: &str) -> WeaponDef {
        self.def_storage
            .weapon_defs
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("weapon named {} did not exist", name))
            .clone()
    }
    pub fn ship_def_by_name(&self, name: &str) -> ShipDef {
        self.def_storage
            .ship_defs
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("ship def named {} did not exist", name))
            .clone()
    }
    pub fn gear_def_by_name(&self, name: &str) -> GearDef {
        self.def_storage
            .gear_defs
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("gear def named {} did not exist", name))
            .clone()
    }
    pub fn mission_def_by_name(&self, name: &str) -> MissionDef {
        self.def_storage
            .mission_defs
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("Mission named {} did not exist", name))
            .clone()
    }

    // --- Extraction ---

    pub fn extraction_end_t(&self) -> Option<f64> {
        self.extraction_t.map(|d| d.end_t)
    }
    pub fn extraction_factor(&self) -> f64 {
        let ship_t = self.ship.destroyed_t.unwrap_or(self.frame_t);
        if let Some(d) = self.extraction_t {
            (ship_t - d.start_t) / (d.end_t - d.start_t)
        } else {
            0.
        }
    }
    pub fn extraction_successful(&self) -> bool {
        self.extraction_end_t().is_some()
            && self.extraction_end_t() < Some(self.frame_t)
            && !self.ship.destroyed()
    }
    pub fn start_extracting(&mut self, delay: f64) {
        self.extraction_t = Some(Duration {
            start_t: self.frame_t,
            end_t: self.frame_t + delay,
        });
        for loot in self.loot.iter_mut() {
            loot.frozen_t = Some(loot.expire_t - self.frame_t);
        }
    }
    pub fn total_extraction_time(&self) -> f64 {
        let ship_t = self.ship.def.extraction_time;
        let mission_t = self.mission.as_ref().map_or(0., |m| m.def.extraction_time);
        (ship_t + mission_t) * (1. + self.encumberance() as f64)
    }
    pub fn encumberance(&self) -> f32 {
        let total = self.total_weight();
        if total <= self.ship.def.weight_limit {
            0.
        } else {
            (total - self.ship.def.weight_limit) / self.ship.def.weight_limit
        }
    }

    // --- Rendering state ---

    pub fn render_main_menu(&self) -> bool {
        self.new_game || self.ship.destroyed() || self.extraction_successful()
    }

    // --- Day / rent ---

    pub fn increment_day(&mut self) {
        self.metaprog.current_day += 1;
    }
    pub fn days_until_rent(&self) -> i32 {
        self.metaprog.day_rent_due - self.metaprog.current_day
    }

    // --- Economy ---

    pub fn balance(&self) -> f64 {
        self.metaprog.balance
    }
    pub fn bounty(&self) -> f64 {
        self.metaprog.gains
    }
    pub fn losses(&self) -> f64 {
        self.metaprog.losses
    }
    pub fn cfg_weapon_choice(&self) -> String {
        self.metaprog.cfg_weapon_choice.clone()
    }
    pub fn cfg_ship_choice(&self) -> String {
        self.metaprog.cfg_ship_choice.clone()
    }

    pub fn add_expense(&mut self, new_entry: Expense) {
        if let Some(entry) = self
            .expenses
            .iter_mut()
            .find(|exp| exp.name == new_entry.name && exp.cost == new_entry.cost)
        {
            entry.count += new_entry.count;
        } else {
            self.expenses.push(new_entry);
        }
    }
    pub fn total_expense_value(&self) -> f64 {
        self.expenses
            .iter()
            .map(|exp| exp.cost * exp.count as f64)
            .sum()
    }
    pub fn itemized_expenses(&self) -> String {
        let mut out = String::new();
        for entry in &self.expenses {
            out.push_str(&format!("\n{}", entry));
        }
        out
    }
    pub fn update_fuel_expense(&mut self) {
        let name = "Fuel Cost";
        let cost = round_with_decimals(self.ship.fuel_used() * FUEL_COST, 2);
        if let Some(entry) = self.expenses.iter_mut().find(|exp| exp.name == name) {
            entry.cost = cost;
        } else {
            self.expenses.push(Expense {
                name: name.to_string(),
                count: 1,
                cost,
            });
        }
    }

    // --- Cargo / Stash ---

    pub fn cargo(&self) -> &[Item] {
        &self.metaprog.cargo_hold
    }
    pub fn add_cargo(&mut self, items: Vec<Item>) {
        for item in items {
            let stackable = self.gear_def_by_name(&item.name).stackable;
            if stackable {
                if let Some(existing) = self.metaprog.cargo_hold.iter_mut().find(|i| i.name == item.name) {
                    existing.count += item.count;
                    continue;
                }
            }
            self.metaprog.cargo_hold.push(item);
        }
    }
    pub fn add_stash(&mut self, items: Vec<Item>) {
        for item in items {
            let stackable = self.gear_def_by_name(&item.name).stackable;
            if stackable {
                if let Some(existing) = self.metaprog.stash.iter_mut().find(|i| i.name == item.name) {
                    existing.count += item.count;
                    continue;
                }
            }
            self.metaprog.stash.push(item);
        }
    }
    pub fn take_cargo(&mut self, idx: usize, amt: i32) -> Item {
        if self.metaprog.cargo_hold[idx].count <= amt {
            return self.metaprog.cargo_hold.remove(idx);
        }
        let item_out = Item {
            name: self.metaprog.cargo_hold[idx].name.clone(),
            count: amt,
            ..Default::default()
        };
        self.metaprog.cargo_hold[idx].count -= amt;
        item_out
    }
    pub fn reveal_cargo(&mut self, idx: usize) {
        self.metaprog.cargo_hold[idx].name_hidden = false;
    }
    pub fn try_trade_cargo(&mut self, idx: usize) -> f64 {
        let can_sell = !self.metaprog.cargo_hold[idx].pending_sale;
        let gear_value = self.gear_def_by_name(&self.metaprog.cargo_hold[idx].name).value
            * self.metaprog.cargo_hold[idx].count as f64;
        if can_sell {
            self.metaprog.cargo_hold[idx].pending_sale = true;
            gear_value
        } else {
            self.metaprog.cargo_hold[idx].pending_sale = false;
            -gear_value
        }
    }
    pub fn cleanup_cargo(&mut self) {
        self.metaprog.cargo_hold.retain(|i| !i.pending_destroy && !i.pending_sale);
    }
    pub fn cleanup_stash(&mut self) {
        self.metaprog.stash.retain(|i| !i.pending_destroy && !i.pending_sale);
    }
    pub fn cargo_value(&self, instasell_only: bool) -> f64 {
        self.metaprog
            .cargo_hold
            .iter()
            .map(|i| {
                let gear = self.gear_def_by_name(&i.name);
                if i.pending_destroy || i.pending_sale {
                    0.
                } else if !instasell_only || gear.instasell {
                    i.count as f64 * gear.value
                } else {
                    0.
                }
            })
            .sum()
    }
    pub fn cargo_weight(&self) -> f32 {
        self.metaprog
            .cargo_hold
            .iter()
            .map(|i| {
                if i.pending_destroy || i.pending_sale || i.is_active {
                    0.
                } else {
                    self.gear_def_by_name(&i.name).weight * i.count as f32
                }
            })
            .sum()
    }
    pub fn total_weight(&self) -> f32 {
        self.main_weapon.as_ref().map_or(0., |w| w.def.weight)
            + self.bounty() as f32
            + self.cargo_weight()
            + self.ship.def.frame_weight
            + self.ship.fuel as f32 * FUEL_WEIGHT
    }
    pub fn cargo_pending_destroy(&mut self) {
        for item in self.metaprog.cargo_hold.iter_mut() {
            let indestructible = self.def_storage.gear_defs.iter().any(|g| g.name == item.name && g.indestructible);
            if !indestructible {
                item.pending_destroy = true;
                item.pending_sale = false;
            }
        }
    }
    pub fn cargo_pending_sale(&mut self) {
        for item in self.metaprog.cargo_hold.iter_mut() {
            let instasell = self.def_storage.gear_defs.iter().any(|g| g.name == item.name && g.instasell);
            if !item.pending_destroy && instasell {
                item.pending_sale = true;
            }
        }
    }
    pub fn pending_sale_value(&self) -> f64 {
        self.metaprog
            .cargo_hold
            .iter()
            .map(|i| {
                if i.pending_sale {
                    i.count as f64 * self.gear_def_by_name(&i.name).value
                } else {
                    0.
                }
            })
            .sum()
    }

    // --- Equipped items ---

    pub fn equipped_weapon_slots(&self) -> Vec<(String, usize)> {
        let weapon_names = self.def_storage.all_weapon_names();
        self.metaprog
            .cargo_hold
            .iter()
            .enumerate()
            .filter(|(_, item)| weapon_names.contains(&item.name) && item.available())
            .map(|(idx, item)| (item.name.clone(), idx))
            .collect()
    }
    pub fn maybe_equipped_weapon_name(&self) -> Option<String> {
        self.equipped_weapon_slots().first().map(|(name, _)| name.clone())
    }
    pub fn equipped_ship_slots(&self) -> Vec<(String, usize)> {
        let ship_names = self.def_storage.all_ship_names();
        self.metaprog
            .cargo_hold
            .iter()
            .enumerate()
            .filter(|(_, item)| ship_names.contains(&item.name) && item.available())
            .map(|(idx, item)| (item.name.clone(), idx))
            .collect()
    }
    pub fn maybe_equipped_ship_name(&self) -> Option<String> {
        self.equipped_ship_slots().first().map(|(name, _)| name.clone())
    }
    pub fn maybe_equipped_weapon_idx(&self) -> Option<usize> {
        self.equipped_weapon_slots().first().map(|(_, idx)| *idx)
    }
    pub fn maybe_equipped_ship_idx(&self) -> Option<usize> {
        self.equipped_ship_slots().first().map(|(_, idx)| *idx)
    }
    pub fn try_set_active_ship_and_weapon(&mut self) {
        for item in self.metaprog.cargo_hold.iter_mut() {
            item.is_active = false;
        }
        if let Some(name) = self.maybe_equipped_ship_name() {
            if let Some(item) = self.metaprog.cargo_hold.iter_mut().find(|i| i.name == name && i.available()) {
                item.is_active = true;
            }
        }
        if let Some(name) = self.maybe_equipped_weapon_name() {
            if let Some(item) = self.metaprog.cargo_hold.iter_mut().find(|i| i.name == name && i.available()) {
                item.is_active = true;
            }
        }
    }

    // --- World reset ---

    pub fn clear_run_state(&mut self) {
        self.extraction_t = None;
        self.asteroids.clear();
        self.bullets.clear();
        self.particles.clear();
        self.loot.clear();
        self.zoom_factor = ZOOM_FACTOR;
        self.screen_center = Vec2::ZERO;
        self.metaprog.gains = 0.;
        self.metaprog.losses = 0.;
        self.expenses.clear();
        self.pity_timers.reset_counters();
    }

    // --- Entity creation ---

    pub fn make_ship(&mut self, def: ShipDef) -> Ship {
        let id = self.gen_id();
        let fuel = def.fuel_capacity;
        Ship {
            id,
            def,
            pos: Vec2::ZERO,
            rot: -PI / 2.,
            vel: Vec2::ZERO,
            fuel,
            destroyed_t: None,
            targeting_system: TargetingSystem::new(),
            guidance_system: GuidanceSystem::default(),
            shield_system: ShieldSystem::default(),
        }
    }
    pub fn make_weapon(&mut self, def: WeaponDef) -> Weapon {
        let id = self.gen_id();
        let ammo = def.magazine;
        Weapon {
            id,
            def,
            ammo,
            chamber: 0,
            last_shot: 0.,
            last_reload: 0.,
            shots_fired: 0,
            shots_hit: 0,
        }
    }
    pub fn make_mission(&mut self, def: MissionDef) -> Mission {
        let id = self.gen_id();
        Mission {
            id,
            def,
            current_wave: 0,
            mission_start_t: self.frame_t,
        }
    }
    pub fn create_projectile(
        &mut self,
        pos: Vec2,
        vel: Vec2,
        projectile_def: ProjectileDef,
        parent: ElementID,
    ) {
        let target = self.ship.targeting_system.current_target;
        let dir = if vel.length_squared() > 0. {
            vel.normalize()
        } else if let Some(id) = target {
            if let Some((tar_pos, _)) = self.asteroid_from_id_pos_vel(id) {
                (tar_pos - pos).normalize()
            } else {
                Vec2::from_angle(self.ship.rot)
            }
        } else {
            Vec2::from_angle(self.ship.rot)
        };
        let id = self.gen_id();
        self.bullets.push(Projectile {
            id,
            pos,
            vel,
            dir,
            spawn_t: self.frame_t,
            duration: projectile_def.duration,
            pending_destroy: false,
            def: projectile_def,
            target,
            parent: Some(parent),
        });
    }
    pub fn create_asteroid(
        &mut self,
        pos: Vec2,
        vel: Vec2,
        radius: f32,
        rot_speed: f32,
        sides: u8,
        altered_vertices: Vec<(u8, f32)>,
        rot: f32,
        armor: i32,
        value: i32,
    ) {
        self.create_asteroid_special(pos, vel, radius, rot_speed, sides, altered_vertices, rot, armor, value, 0.);
    }
    pub fn create_asteroid_special(
        &mut self,
        pos: Vec2,
        vel: Vec2,
        radius: f32,
        rot_speed: f32,
        sides: u8,
        altered_vertices: Vec<(u8, f32)>,
        rot: f32,
        armor: i32,
        value: i32,
        homing_turn_rate: f32,
    ) {
        let id = self.gen_id();
        self.asteroids.push(Asteroid {
            id,
            pos,
            vel,
            rot,
            rot_speed,
            radius,
            sides,
            altered_vertices,
            spawn_t: self.frame_t,
            pending_destroy: false,
            armor: armor as f32,
            value: value as f64,
            last_struck_t: 0.,
            last_struck_id: None,
            homing_turn_rate,
        });
    }

    // --- Targeting ---

    pub fn asteroid_from_id_pos_vel(&self, id: ElementID) -> Option<(Vec2, Vec2)> {
        self.asteroids
            .iter()
            .find(|a| a.id == id)
            .map(|a| (a.pos, a.vel))
    }

    pub fn delta_angle_to_pos(&self, from: Vec2, from_rot: f32, target_pos: Vec2) -> f32 {
        let to_target = target_pos - from;
        let target_angle = to_target.y.atan2(to_target.x);
        let mut delta = target_angle - from_rot;
        while delta > PI {
            delta -= TAU;
        }
        while delta < -PI {
            delta += TAU;
        }
        delta
    }

    pub fn delta_angle_to_target(
        &self,
        from: Vec2,
        from_rot: f32,
        turn_rate: f32,
        target_pos: Vec2,
        target_vel: Vec2,
    ) -> f32 {
        let lead_pos = self.lead_target_pos(from, from_rot, turn_rate, target_pos, target_vel);
        self.delta_angle_to_pos(from, from_rot, lead_pos)
    }

    pub fn lead_target_pos(
        &self,
        from: Vec2,
        _from_rot: f32,
        _turn_rate: f32,
        target_pos: Vec2,
        target_vel: Vec2,
    ) -> Vec2 {
        let weapon_speed = self
            .main_weapon
            .as_ref()
            .map_or(500., |w| w.def.projectile_stats.speed);
        if weapon_speed <= 0. {
            return target_pos;
        }
        let dist = (target_pos - from).length();
        let time_to_target = dist / weapon_speed;
        // Track assist based on ship targeting system
        let assist = self.ship.targeting_system.tracking_assist_mid;
        target_pos + target_vel * time_to_target * assist
    }

    // --- Ship / weapon state helpers ---

    pub fn can_shoot(&self) -> bool {
        self.extraction_t.is_none()
    }
    pub fn can_rotate(&self) -> bool {
        self.extraction_t.is_none() && self.ship.fuel > 0.
    }

    // --- Update tick ---

    pub fn update_ship(&mut self) {
        let acc = -self.ship.vel / 100.;
        self.ship.vel += acc;
        if self.ship.vel.length() > 5. {
            self.ship.vel = self.ship.vel.normalize() * 5.;
        }
        let dt = self.dt();
        self.ship.pos = wrap_around(self.ship.pos + self.ship.vel * dt);

        let elapsed = self.frame_t
            - self.ship.shield_system.last_struck_t
            - self.ship.shield_system.shield_regen_delay;
        if self.ship.shield_system.shield < self.ship.shield_system.shield_capacity
            && self.ship.shield_system.shield_regen_rate != 0.
            && elapsed > 0.
        {
            self.ship.shield_system.shield +=
                self.ship.shield_system.shield_regen_rate * elapsed.min(dt as f64);
        }
        // Ship-specific per-frame behavior
        ship_update(self);
    }

    pub fn update_weapon(&mut self) {
        let Some(weapon) = &self.main_weapon else {
            return;
        };
        let frame_t = self.frame_t;
        if weapon.ammo == 0 {
            // Start reload
            let w = self.main_weapon.as_mut().unwrap();
            w.last_reload = frame_t;
            w.ammo = -1;
        } else if weapon.ammo == -1 {
            if frame_t - weapon.last_reload > weapon.def.reload as f64 {
                let w = self.main_weapon.as_mut().unwrap();
                w.ammo = w.def.magazine;
                w.last_shot = 0.;
                w.chamber = 0;
            }
        } else if frame_t - weapon.last_shot > weapon.def.reload as f64
            && self.ship.guidance_system.weapon_diagnostics
        {
            let w = self.main_weapon.as_mut().unwrap();
            w.ammo = w.def.magazine;
            w.last_shot = 0.;
            w.chamber = 0;
        }
    }

    pub fn update_bullets(&mut self) {
        let dt = self.dt();
        let frame_t = self.frame_t;
        for bullet in self.bullets.iter_mut() {
            if bullet.pending_destroy {
                continue;
            }
            // Homing
            if bullet.def.homing_turn_rate > 0. {
                if let Some(target_id) = bullet.target {
                    if let Some(ast) = self.asteroids.iter().find(|a| a.id == target_id) {
                        let to_target = ast.pos - bullet.pos;
                        let target_angle = to_target.y.atan2(to_target.x);
                        let current_angle = bullet.vel.y.atan2(bullet.vel.x);
                        let mut delta = target_angle - current_angle;
                        while delta > PI {
                            delta -= TAU;
                        }
                        while delta < -PI {
                            delta += TAU;
                        }
                        let max_turn = bullet.def.homing_turn_rate * dt;
                        let turn = delta.clamp(-max_turn, max_turn);
                        let new_angle = current_angle + turn;
                        let speed = bullet.vel.length();
                        bullet.vel = Vec2::new(new_angle.cos(), new_angle.sin()) * speed;
                    }
                }
            }
            bullet.pos += bullet.vel * dt;
            // Expire
            if bullet.age(frame_t) >= bullet.duration {
                if bullet.def.death_impact {
                    bullet.pending_destroy = true;
                } else {
                    bullet.pending_destroy = true;
                }
            }
        }
        self.bullets.retain(|b| !b.pending_destroy);
    }

    pub fn update_asteroids(&mut self) {
        let dt = self.dt();
        let _frame_t = self.frame_t;
        for ast in self.asteroids.iter_mut() {
            // Homing
            if ast.homing_turn_rate > 0. {
                let to_ship = self.ship.pos - ast.pos;
                let target_angle = to_ship.y.atan2(to_ship.x);
                let current_angle = ast.vel.y.atan2(ast.vel.x);
                let mut delta = target_angle - current_angle;
                while delta > PI {
                    delta -= TAU;
                }
                while delta < -PI {
                    delta += TAU;
                }
                let max_turn = ast.homing_turn_rate * dt;
                let turn = delta.clamp(-max_turn, max_turn);
                let new_angle = current_angle + turn;
                let speed = ast.vel.length();
                ast.vel = Vec2::new(new_angle.cos(), new_angle.sin()) * speed;
            }
            ast.pos += ast.vel * dt;
            ast.rot += ast.rot_speed * dt;
            ast.pos = wrap_around(ast.pos);
        }
    }

    pub fn update_particles(&mut self) {
        let dt = self.dt();
        let frame_t = self.frame_t;
        for p in self.particles.iter_mut() {
            p.vel += p.accel * dt;
            p.pos += p.vel * dt;
            p.rot += p.rot_vel * dt;
            p.length += p.growth * dt;
        }
        self.particles.retain(|p| frame_t - p.spawn_t < p.duration);
    }

    pub fn update_loot(&mut self) {
        self.loot.retain(|l| {
            if l.frozen_t.is_some() {
                true
            } else {
                l.expire_t > self.frame_t
            }
        });
    }

    pub fn spawn_impact_particles(&mut self, pos: &Vec2, bullet: &Projectile) {
        let count = 3 + (bullet.def.length / 3.) as i32;
        for _ in 0..count {
            let angle = self.rng.gen_range_f32(0., TAU);
            let speed = self.rng.gen_range_f32(10., 60.);
            self.particles.push(Particle {
                pos: *pos,
                vel: Vec2::new(angle.cos(), angle.sin()) * speed,
                spawn_t: self.frame_t,
                duration: self.rng.gen_range_f32(0.1, 0.4) as f64,
                length: self.rng.gen_range_f32(1., 3.),
                shape: EntityShape::Pellet,
                fade_out: true,
                ..Default::default()
            });
        }
    }

    pub fn spawn_turn_particles(&mut self, turn_amount: f32, ship: &Ship, intensity: f32) {
        let (pos_l, pos_r) = ship.wing_tips(Vec2::ZERO);
        let exhaust_dir = if turn_amount > 0. { pos_l } else { pos_r };
        let speed = 20. * intensity;
        let away = (exhaust_dir - ship.pos).normalize_or_zero();
        for _ in 0..2 {
            let angle = self.rng.gen_range_f32(-0.3, 0.3);
            let vel = Vec2::new(
                away.x * angle.cos() - away.y * angle.sin(),
                away.x * angle.sin() + away.y * angle.cos(),
            ) * speed;
            self.particles.push(Particle {
                pos: exhaust_dir,
                vel,
                spawn_t: self.frame_t,
                duration: self.rng.gen_range_f32(0.1, 0.25) as f64,
                length: self.rng.gen_range_f32(1., 2.),
                shape: EntityShape::Pellet,
                fade_out: true,
                ..Default::default()
            });
        }
    }

    // --- Collision detection ---

    pub fn check_bullet_asteroid_collisions(&mut self) {
        let frame_t = self.frame_t;
        let mut impacts: Vec<(usize, usize, Vec2, CollisionInfo)> = Vec::new();

        for (bi, bullet) in self.bullets.iter().enumerate() {
            if bullet.pending_destroy {
                continue;
            }
            if frame_t - bullet.spawn_t < ASTEROID_SPAWN_IMMUNITY_DURATION {
                continue;
            }
            let bullet_verts = bullet_vertices(bullet, frame_t);
            for (ai, asteroid) in self.asteroids.iter().enumerate() {
                if asteroid.pending_destroy {
                    continue;
                }
                if frame_t - asteroid.spawn_t < ASTEROID_SPAWN_IMMUNITY_DURATION {
                    continue;
                }
                // Quick distance check
                let dist = (bullet.pos - asteroid.pos).length();
                if dist > asteroid.e_radius(frame_t) + bullet.current_length(frame_t) * 2. {
                    continue;
                }
                let ast_verts = asteroid.vertices(frame_t);
                if let Some(info) = check_convex_polygon_collision(&bullet_verts, &ast_verts) {
                    impacts.push((bi, ai, bullet.pos, info));
                }
            }
        }

        // Process impacts
        for (bi, ai, pos, info) in impacts {
            // Damage asteroid
            let damage = self.bullets[bi].def.damage;
            self.asteroids[ai].armor -= damage;
            self.asteroids[ai].last_struck_t = frame_t;
            self.asteroids[ai].last_struck_id = Some(self.bullets[bi].id);

            // Track accuracy
            if let Some(ref mut weapon) = self.main_weapon {
                weapon.shots_hit += 1;
            }

            // Handle impact effects
            self.spawn_impact_particles(&pos, &self.bullets[bi].clone());

            if !self.bullets[bi].def.persists {
                self.bullets[bi].pending_destroy = true;
            }

            // Call impact handler
            let impact_kind = self.bullets[bi].def.impact_kind;
            let bullet_clone = self.bullets[bi].clone();
            crate::init::projectile_impact(self, impact_kind, &bullet_clone, pos, info);

            // Destroy asteroid if armor depleted
            if self.asteroids.len() > ai && self.asteroids[ai].armor <= 0. {
                self.shatter_asteroid(ai);
            }
        }
    }

    pub fn shatter_asteroid(&mut self, ai: usize) {
        let asteroid = self.asteroids[ai].clone();
        asteroid_shatter(self, &asteroid);
        if ai < self.asteroids.len() {
            self.asteroids[ai].pending_destroy = true;
        }
        self.asteroids.retain(|a| !a.pending_destroy);
    }

    pub fn check_asteroid_ship_collisions(&mut self) {
        let frame_t = self.frame_t;
        if self.ship.destroyed() {
            return;
        }
        let ship_verts: Vec<Vec2> = self.ship.vertex_sets().into_iter().flatten().collect();
        if ship_verts.is_empty() {
            return;
        }

        // Check shield first
        if self.ship.shield_system.shield >= 1. {
            let shield_verts = self.ship.shield_vertices(self.ship.shield_system.shield.floor() as i32);
            for asteroid in &self.asteroids {
                if asteroid.pending_destroy || frame_t - asteroid.spawn_t < ASTEROID_SPAWN_IMMUNITY_DURATION {
                    continue;
                }
                let ast_verts = asteroid.vertices(frame_t);
                if check_convex_polygon_collision(&shield_verts, &ast_verts).is_some() {
                    self.ship.shield_system.shield -= self.ship.shield_system.shield_damage as f64;
                    self.ship.shield_system.last_struck_t = frame_t;
                    return;
                }
            }
        }

        // Check hull
        for asteroid in &self.asteroids {
            if asteroid.pending_destroy || frame_t - asteroid.spawn_t < ASTEROID_SPAWN_IMMUNITY_DURATION {
                continue;
            }
            let ast_verts = asteroid.vertices(frame_t);
            if check_convex_polygon_collision(&ship_verts, &ast_verts).is_some() {
                self.ship.destroyed_t = Some(frame_t);
                return;
            }
        }
    }

    // --- Wave spawning ---

    pub fn spawn_wave(&mut self) {
        let Some(ref mission) = self.mission.clone() else {
            return;
        };
        if mission.current_wave >= mission.def.max_waves {
            return;
        }
        if self.frame_t - self.last_spawn_t < mission.def.spawn_interval {
            return;
        }
        self.last_spawn_t = self.frame_t;
        if let Some(ref mut m) = self.mission {
            m.current_wave += 1;
        }
        let wave_kind = mission.def.wave_kind;
        crate::init::dispatch_wave(self, wave_kind);
    }

    // --- Mission scanner ---

    pub fn init_scan(&mut self) {
        self.mission_scanner.current_scan_t = self.frame_t;
        self.mission_scanner.scanning = true;
        self.mission_scanner.scan_conclusion_t =
            self.frame_t + self.rng.gen_range_f32(20. / 3., 20.) as f64;
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

fn bullet_vertices(bullet: &Projectile, frame_t: f64) -> Vec<Vec2> {
    let len = bullet.current_length(frame_t);
    match bullet.def.shape {
        EntityShape::Pellet | EntityShape::Disc => {
            // Approximate circle as octagon
            let r = len / 2.;
            let mut verts = Vec::with_capacity(8);
            for i in 0..8 {
                let angle = i as f32 * TAU / 8.;
                verts.push(bullet.pos + Vec2::new(angle.cos(), angle.sin()) * r);
            }
            verts
        }
        EntityShape::Oblong | EntityShape::Line => {
            let dir = bullet.current_direction();
            let perp = dir.perp() * 1.0;
            let half = dir * (len / 2.);
            vec![
                bullet.pos + half + perp,
                bullet.pos + half - perp,
                bullet.pos - half - perp,
                bullet.pos - half + perp,
            ]
        }
    }
}

fn asteroid_shatter(w: &mut GameWorld, asteroid: &Asteroid) {
    // Drop loot
    let items = asteroid.loot_drops(w);
    if !items.is_empty() {
        let id = w.gen_id();
        w.loot.push(Loot {
            id,
            pos: asteroid.pos,
            rot: w.rng.gen_range_f32(0., TAU),
            radius: 8.,
            sides: 6,
            spawn_t: w.frame_t,
            expire_t: w.frame_t + 30.,
            frozen_t: None,
            contained_items: items,
        });
    }

    // Spawn shatter particles
    let count = asteroid.sides as i32 + 2;
    for _ in 0..count {
        let angle = w.rng.gen_range_f32(0., TAU);
        let speed = w.rng.gen_range_f32(20., ASTEROID_SPEED_SHATTER);
        w.particles.push(Particle {
            pos: asteroid.pos,
            vel: Vec2::new(angle.cos(), angle.sin()) * speed,
            spawn_t: w.frame_t,
            duration: w.rng.gen_range_f32(0.2, 0.6) as f64,
            length: w.rng.gen_range_f32(2., asteroid.radius / 2.),
            shape: EntityShape::Pellet,
            fade_out: true,
            ..Default::default()
        });
    }

    // Split into smaller asteroids if large enough
    if asteroid.sides >= 5 && asteroid.radius > 15. {
        let new_sides = (asteroid.sides - 1).max(3);
        let new_radius = asteroid.radius * 0.6;
        let count = 2;
        for i in 0..count {
            let angle = asteroid.rot + (i as f32 * PI);
            let offset = Vec2::new(angle.cos(), angle.sin()) * asteroid.radius * 0.5;
            let vel_angle = w.rng.gen_range_f32(0., TAU);
            let speed = w.rng.gen_range_f32(ASTEROID_SPEED * 0.5, ASTEROID_SPEED);
            let rot_speed = w.rng.gen_range_f32(-2., 2.);
            let rot = w.rng.gen_range_f32(0., TAU);
            w.create_asteroid(
                asteroid.pos + offset,
                Vec2::new(vel_angle.cos(), vel_angle.sin()) * speed,
                new_radius,
                rot_speed,
                new_sides,
                vec![],
                rot,
                1,
                0,
            );
        }
    }
}

/// Complex-multiply two Vec2s (equivalent to macroquad's Vec2::rotate).
pub fn rotate_vec2(v: Vec2, rot: Vec2) -> Vec2 {
    Vec2::new(v.x * rot.x - v.y * rot.y, v.x * rot.y + v.y * rot.x)
}

pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn wrap_around(v: Vec2) -> Vec2 {
    let mut vr = v;
    if vr.x > GAME_X {
        vr.x = -GAME_X;
    }
    if vr.x < -GAME_X {
        vr.x = GAME_X;
    }
    if vr.y > GAME_Y {
        vr.y = -GAME_Y;
    }
    if vr.y < -GAME_Y {
        vr.y = GAME_Y;
    }
    vr
}

pub fn within_bounds(v: Vec2, radius: f32, tolerance: f32) -> bool {
    !(v.x - radius > GAME_X * tolerance
        || v.x + radius < -GAME_X * tolerance
        || v.y - radius > GAME_Y * tolerance
        || v.y + radius < -GAME_Y * tolerance)
}

pub fn round_with_decimals(v: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (v * factor).round() / factor
}

// ---------------------------------------------------------------------------
// Ship dispatch (replaces function pointers)
// ---------------------------------------------------------------------------

/// Per-ship polygon vertices and wing-tip positions.
pub fn ship_vertices(ship: &Ship) -> (Vec<Vec<Vec2>>, [Vec2; 2]) {
    let h = ship.def.height;
    let w = ship.def.width;
    let r = Vec2::from_angle(ship.rot);
    match ship.def.shape {
        ShipShape::Triangle => {
            let nose = Vec2::new(h / 2., 0.);
            let bl = Vec2::new(-h / 2., -w / 2.);
            let br = Vec2::new(-h / 2., w / 2.);
            let verts = vec![vec![
                ship.pos + rotate_vec2(nose, r),
                ship.pos + rotate_vec2(bl, r),
                ship.pos + rotate_vec2(br, r),
            ]];
            let tips = [ship.pos + rotate_vec2(bl, r), ship.pos + rotate_vec2(br, r)];
            (verts, tips)
        }
        ShipShape::Star => {
            let nose = Vec2::new(h / 2., 0.);
            let bl = Vec2::new(-h / 2., -w / 2.);
            let br = Vec2::new(-h / 2., w / 2.);
            let verts = vec![vec![
                ship.pos + rotate_vec2(nose, r),
                ship.pos + rotate_vec2(bl, r),
                ship.pos + rotate_vec2(bl * 0.6, r),
                ship.pos + rotate_vec2(br * 0.6, r),
                ship.pos + rotate_vec2(br, r),
            ]];
            let tips = [ship.pos + rotate_vec2(bl, r), ship.pos + rotate_vec2(br, r)];
            (verts, tips)
        }
        ShipShape::Diamond => {
            let nose = Vec2::new(h / 2., 0.);
            let bl = Vec2::new(-h / 2., -w / 2.);
            let bc = Vec2::new(-h / 4., 0.);
            let br = Vec2::new(-h / 2., w / 2.);
            let verts = vec![vec![
                ship.pos + rotate_vec2(nose, r),
                ship.pos + rotate_vec2(bl, r),
                ship.pos + rotate_vec2(bc, r),
                ship.pos + rotate_vec2(br, r),
            ]];
            let tips = [ship.pos + rotate_vec2(bl, r), ship.pos + rotate_vec2(br, r)];
            (verts, tips)
        }
        ShipShape::Crab => {
            let nose = Vec2::new(0., 0.);
            let ln = Vec2::new(h / 2., -w / 3.);
            let ll = Vec2::new(h / 2., -w / 2.5);
            let bcl = Vec2::new(-h / 2., -w / 2.);
            let bcr = Vec2::new(-h / 2., w / 2.);
            let rl = Vec2::new(h / 2., w / 2.5);
            let rn = Vec2::new(h / 2., w / 3.);
            let verts = vec![vec![
                ship.pos + rotate_vec2(nose, r),
                ship.pos + rotate_vec2(ln, r),
                ship.pos + rotate_vec2(ll, r),
                ship.pos + rotate_vec2(bcl, r),
                ship.pos + rotate_vec2(bcr, r),
                ship.pos + rotate_vec2(rl, r),
                ship.pos + rotate_vec2(rn, r),
            ]];
            let tips = [ship.pos + rotate_vec2(ll, r), ship.pos + rotate_vec2(rl, r)];
            (verts, tips)
        }
        ShipShape::TriangleWithTail => {
            let nose = Vec2::new(h / 2., 0.);
            let bl = Vec2::new(-h / 2., -w / 2.);
            let br = Vec2::new(-h / 2., w / 2.);
            let rn = Vec2::new(-h / 1.75, 0.);
            let rbl = Vec2::new(h / 3., -w / 3.);
            let rbr = Vec2::new(h / 3., w / 3.);
            let verts = vec![
                vec![
                    ship.pos + rotate_vec2(nose, r),
                    ship.pos + rotate_vec2(bl, r),
                    ship.pos + rotate_vec2(br, r),
                ],
                vec![
                    ship.pos + rotate_vec2(rn, r),
                    ship.pos + rotate_vec2(rbl, r),
                    ship.pos + rotate_vec2(rbr, r),
                ],
            ];
            let tips = [ship.pos + rotate_vec2(bl, r), ship.pos + rotate_vec2(br, r)];
            (verts, tips)
        }
        ShipShape::Pentagon => {
            let nose = Vec2::new(h / 2., 0.);
            let fl = Vec2::new(h / 3., -w / 2.5);
            let bl = Vec2::new(-h / 2., -w / 2.);
            let br = Vec2::new(-h / 2., w / 2.);
            let fr = Vec2::new(h / 3., w / 2.5);
            let verts = vec![vec![
                ship.pos + rotate_vec2(nose, r),
                ship.pos + rotate_vec2(fl, r),
                ship.pos + rotate_vec2(bl, r),
                ship.pos + rotate_vec2(br, r),
                ship.pos + rotate_vec2(fr, r),
            ]];
            let tips = [ship.pos + rotate_vec2(bl, r), ship.pos + rotate_vec2(br, r)];
            (verts, tips)
        }
    }
}

/// Per-frame ship behavior (only Carrier does anything).
fn ship_update(w: &mut GameWorld) {
    if w.ship.def.kind == ShipKind::Carrier {
        let target = w.ship.targeting_system.current_target;
        for proj in w.bullets.iter_mut() {
            proj.target = target;
        }
    }
}

/// One-time ship launch setup.
pub fn ship_launch(w: &mut GameWorld) {
    match w.ship.def.kind {
        ShipKind::BrittleStar => {
            w.ship.guidance_system.fuel_efficiency_bonus += 0.25;
        }
        ShipKind::BattleStar => {
            w.ship.targeting_system.wing_turret_rot = PI / 3.;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Gear dispatch (replaces function pointers)
// ---------------------------------------------------------------------------

pub fn gear_equip(effect: &GearEffect, w: &mut GameWorld) {
    match effect {
        GearEffect::None => {}
        GearEffect::ProjectileSpeed(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                let stats = &mut weapon.def.projectile_stats;
                if stats.speed <= 1. {
                    stats.length *= *factor;
                } else {
                    stats.speed *= *factor;
                }
            }
        }
        GearEffect::ReloadRate(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.reload *= *factor;
            }
        }
        GearEffect::ExtendedMagazine(reload_mul, mag_mul) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.reload *= *reload_mul;
                weapon.def.magazine *= *mag_mul;
            }
        }
        GearEffect::Zoom(factor) => {
            w.zoom_factor *= *factor;
        }
        GearEffect::AimLead(amount) => {
            w.ship.targeting_system.aim_lead += *amount;
        }
        GearEffect::Shield(amount) => {
            w.ship.shield_system.shield += *amount as f64;
        }
        GearEffect::ShieldGenerator { shield, capacity, regen_rate, delay_mul } => {
            w.ship.shield_system.shield += *shield as f64;
            w.ship.shield_system.shield_capacity += *capacity as f64;
            w.ship.shield_system.shield_regen_rate += *regen_rate;
            w.ship.shield_system.shield_regen_delay *= *delay_mul;
        }
        GearEffect::ShipDiagnostics => {
            w.ship.guidance_system.ship_diagnostics = true;
        }
        GearEffect::WeaponDiagnostics => {
            w.ship.guidance_system.weapon_diagnostics = true;
        }
        GearEffect::FuelEfficiency(amount) => {
            w.ship.guidance_system.fuel_efficiency_bonus += *amount as f64;
        }
        GearEffect::ExtractionSpeed(divisor) => {
            w.ship.def.extraction_time /= *divisor;
        }
        GearEffect::Insurance(factor) => {
            let s = &mut w.ship.guidance_system.salvage_minimum_factor;
            let f = *factor as f64;
            *s = 1. - ((1. - *s) * (1. - f));
        }
        GearEffect::Damage(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.projectile_stats.damage *= *factor;
            }
        }
        GearEffect::TrackingAssist(factors, rate_div) => {
            let ts = &mut w.ship.targeting_system;
            ts.tracking_assist_short = 1. - ((1. - ts.tracking_assist_short) * (1. - factors[0]));
            ts.tracking_assist_mid = 1. - ((1. - ts.tracking_assist_mid) * (1. - factors[1]));
            ts.tracking_assist_far = 1. - ((1. - ts.tracking_assist_far) * (1. - factors[2]));
            ts.tracking_rate /= *rate_div as f64;
        }
        GearEffect::RailgunCharge => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.last_reload = w.frame_t;
            }
        }
    }
}

pub fn gear_unequip(effect: &GearEffect, w: &mut GameWorld) {
    match effect {
        GearEffect::None => {}
        GearEffect::ProjectileSpeed(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                let stats = &mut weapon.def.projectile_stats;
                if stats.speed <= 1. {
                    stats.length /= *factor;
                } else {
                    stats.speed /= *factor;
                }
            }
        }
        GearEffect::ReloadRate(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.reload /= *factor;
            }
        }
        GearEffect::ExtendedMagazine(reload_mul, mag_mul) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.reload /= *reload_mul;
                weapon.def.magazine /= *mag_mul;
            }
        }
        GearEffect::Zoom(factor) => {
            w.zoom_factor /= *factor;
        }
        GearEffect::AimLead(amount) => {
            w.ship.targeting_system.aim_lead -= *amount;
        }
        GearEffect::Shield(amount) => {
            w.ship.shield_system.shield -= *amount as f64;
        }
        GearEffect::ShieldGenerator { shield, capacity, regen_rate, delay_mul } => {
            w.ship.shield_system.shield -= *shield as f64;
            w.ship.shield_system.shield_capacity -= *capacity as f64;
            w.ship.shield_system.shield_regen_rate -= *regen_rate;
            w.ship.shield_system.shield_regen_delay /= *delay_mul;
        }
        GearEffect::ShipDiagnostics => {
            w.ship.guidance_system.ship_diagnostics = false;
        }
        GearEffect::WeaponDiagnostics => {
            w.ship.guidance_system.weapon_diagnostics = false;
        }
        GearEffect::FuelEfficiency(amount) => {
            w.ship.guidance_system.fuel_efficiency_bonus -= *amount as f64;
        }
        GearEffect::ExtractionSpeed(divisor) => {
            w.ship.def.extraction_time *= *divisor;
        }
        GearEffect::Insurance(factor) => {
            let s = &mut w.ship.guidance_system.salvage_minimum_factor;
            let f = *factor as f64;
            *s = 1. - ((1. - *s) / (1. - f));
        }
        GearEffect::Damage(factor) => {
            if let Some(ref mut weapon) = w.main_weapon {
                weapon.def.projectile_stats.damage /= *factor;
            }
        }
        GearEffect::TrackingAssist(factors, rate_div) => {
            let ts = &mut w.ship.targeting_system;
            ts.tracking_assist_short = 1. - ((1. - ts.tracking_assist_short) / (1. - factors[0]));
            ts.tracking_assist_mid = 1. - ((1. - ts.tracking_assist_mid) / (1. - factors[1]));
            ts.tracking_assist_far = 1. - ((1. - ts.tracking_assist_far) / (1. - factors[2]));
            ts.tracking_rate *= *rate_div as f64;
        }
        GearEffect::RailgunCharge => {
            // No undo needed for railgun charge
        }
    }
}
