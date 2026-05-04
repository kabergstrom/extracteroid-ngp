use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::world::*;

// ---------------------------------------------------------------------------
// Shoot dispatch
// ---------------------------------------------------------------------------

pub fn dispatch_shoot(w: &mut GameWorld, kind: ShootKind) {
    match kind {
        ShootKind::Default => default_shoot(w),
        ShootKind::TwinShoot => twin_shoot(w),
        ShootKind::TwinWideShoot => twin_wide_shoot(w),
        ShootKind::ChamberShootThree => chamber_shoot_three(w),
        ShootKind::ChamberShootFive => chamber_shoot_five(w),
    }
}

// ---------------------------------------------------------------------------
// Shoot functions
// ---------------------------------------------------------------------------

fn default_shoot(w: &mut GameWorld) {
    let weapon = match w.main_weapon.as_ref() {
        Some(wep) if wep.ammo > 0 => wep,
        _ => return,
    };
    let stats = weapon.def.projectile_stats.clone();
    let weapon_id = weapon.id;
    let ship_pos = w.ship.pos;
    let ship_rot = w.ship.rot;
    let ship_height = w.ship.def.height;

    w.main_weapon.as_mut().unwrap().ammo -= 1;
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    let rot_vec = Vec2::from_angle(ship_rot);
    w.create_projectile(
        ship_pos + rot_vec * (ship_height / 2. + stats.length / 2.),
        rot_vec * stats.speed,
        stats,
        weapon_id,
    );
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;
}

fn twin_shoot(w: &mut GameWorld) {
    let weapon = match w.main_weapon.as_ref() {
        Some(wep) if wep.ammo > 0 => wep,
        _ => return,
    };
    let stats = weapon.def.projectile_stats.clone();
    let weapon_id = weapon.id;

    let (pos_l, pos_r) = w.ship.wing_tips(Vec2::new(0., stats.length / 2.));
    let fwd_l = w.ship.rot;
    let fwd_r = w.ship.rot;

    // Compute wing pivot directions (read-only from w)
    let target_data = w
        .ship
        .targeting_system
        .current_target
        .and_then(|id| w.asteroid_from_id_pos_vel(id));
    let wing_turret_rot = w.ship.targeting_system.wing_turret_rot;
    let ship_rot = w.ship.rot;
    let (dir_l, dir_r) = if let Some((tar_pos, tar_vel)) = target_data {
        let delta_l = w.delta_angle_to_target(pos_l, ship_rot, TAU, tar_pos, tar_vel);
        let delta_r = w.delta_angle_to_target(pos_r, ship_rot, TAU, tar_pos, tar_vel);
        let turn_l = delta_l.clamp(-wing_turret_rot, wing_turret_rot);
        let turn_r = delta_r.clamp(-wing_turret_rot, wing_turret_rot);
        (
            Vec2::from_angle(fwd_l + turn_l),
            Vec2::from_angle(fwd_r + turn_r),
        )
    } else {
        (Vec2::from_angle(fwd_l), Vec2::from_angle(fwd_r))
    };

    // Right wing
    w.main_weapon.as_mut().unwrap().ammo -= 1;
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    w.create_projectile(pos_r, dir_r * stats.speed, stats.clone(), weapon_id);
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;

    // Left wing
    if w
        .main_weapon
        .as_ref()
        .map_or(true, |wep| wep.ammo <= 0)
    {
        return;
    }
    w.main_weapon.as_mut().unwrap().ammo -= 1;
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    w.create_projectile(pos_l, dir_l * stats.speed, stats, weapon_id);
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;
}

fn twin_wide_shoot(w: &mut GameWorld) {
    let weapon = match w.main_weapon.as_ref() {
        Some(wep) if wep.ammo > 0 => wep,
        _ => return,
    };
    let stats = weapon.def.projectile_stats.clone();
    let weapon_id = weapon.id;
    let ship_rot = w.ship.rot;

    let (pos_l, pos_r) = w.ship.wing_tips(Vec2::new(0., stats.length / 2.));

    // Compute spread angle based on target distance
    let target_data = w
        .ship
        .targeting_system
        .current_target
        .and_then(|id| w.asteroid_from_id_pos_vel(id));
    let (fwd_l, fwd_r) = if let Some((tar_pos, _)) = target_data {
        let inner_dist_sq = RADAR_DIST * RADAR_DIST;
        let dist_sq = tar_pos.distance_squared(w.ship.pos);
        let scaling =
            lerp_f32(PI / 6., PI / 3., (dist_sq / inner_dist_sq).clamp(0., 1.));
        (ship_rot - scaling, ship_rot + scaling)
    } else {
        (ship_rot - PI / 3., ship_rot + PI / 3.)
    };

    // Compute wing pivot directions
    let wing_turret_rot = w.ship.targeting_system.wing_turret_rot;
    let (dir_l, dir_r) = if let Some((tar_pos, tar_vel)) = target_data {
        let delta_l = w.delta_angle_to_target(pos_l, ship_rot, TAU, tar_pos, tar_vel);
        let delta_r = w.delta_angle_to_target(pos_r, ship_rot, TAU, tar_pos, tar_vel);
        let turn_l = delta_l.clamp(-wing_turret_rot, wing_turret_rot);
        let turn_r = delta_r.clamp(-wing_turret_rot, wing_turret_rot);
        (
            Vec2::from_angle(fwd_l + turn_l),
            Vec2::from_angle(fwd_r + turn_r),
        )
    } else {
        (Vec2::from_angle(fwd_l), Vec2::from_angle(fwd_r))
    };

    // Right wing
    w.main_weapon.as_mut().unwrap().ammo -= 1;
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    w.create_projectile(pos_r, dir_r * stats.speed, stats.clone(), weapon_id);
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;

    // Left wing
    if w
        .main_weapon
        .as_ref()
        .map_or(true, |wep| wep.ammo <= 0)
    {
        return;
    }
    w.main_weapon.as_mut().unwrap().ammo -= 1;
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    w.create_projectile(pos_l, dir_l * stats.speed, stats, weapon_id);
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;
}

fn chamber_shoot_three(w: &mut GameWorld) {
    let weapon = match w.main_weapon.as_ref() {
        Some(wep) if wep.ammo > 0 => wep,
        _ => return,
    };
    let stats = weapon.def.projectile_stats.clone();
    let weapon_id = weapon.id;
    let chamber = weapon.chamber;
    let ship_pos = w.ship.pos;
    let ship_rot = w.ship.rot;
    let ship_height = w.ship.def.height;

    w.main_weapon.as_mut().unwrap().ammo -= 1;
    if chamber == 0 {
        w.main_weapon.as_mut().unwrap().chamber = 3 - 1;
    }
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    let rot_vec = Vec2::from_angle(ship_rot);
    w.create_projectile(
        ship_pos + rot_vec * (ship_height / 2. + stats.length / 2.),
        rot_vec * stats.speed,
        stats,
        weapon_id,
    );
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;
}

fn chamber_shoot_five(w: &mut GameWorld) {
    let weapon = match w.main_weapon.as_ref() {
        Some(wep) if wep.ammo > 0 => wep,
        _ => return,
    };
    let stats = weapon.def.projectile_stats.clone();
    let weapon_id = weapon.id;
    let chamber = weapon.chamber;
    let ship_pos = w.ship.pos;
    let ship_rot = w.ship.rot;
    let ship_height = w.ship.def.height;

    w.main_weapon.as_mut().unwrap().ammo -= 1;
    if chamber == 0 {
        w.main_weapon.as_mut().unwrap().chamber = 5 - 1;
    }
    w.add_expense(Expense {
        name: stats.name.clone(),
        count: 1,
        cost: stats.cost,
    });
    let rot_vec = Vec2::from_angle(ship_rot);
    w.create_projectile(
        ship_pos + rot_vec * (ship_height / 2. + stats.length / 2.),
        rot_vec * stats.speed,
        stats,
        weapon_id,
    );
    w.main_weapon.as_mut().unwrap().last_shot = w.frame_t;
}

// ---------------------------------------------------------------------------
// Impact dispatch
// ---------------------------------------------------------------------------

pub fn projectile_impact(w: &mut GameWorld, kind: ImpactKind, bullet: &Projectile, pos: Vec2, info: CollisionInfo) {
    match kind {
        ImpactKind::None => {}
        ImpactKind::Bouncing => bouncing_impact(w, bullet, pos, info),
        ImpactKind::Flak => flak_impact(w, bullet, pos, info),
        ImpactKind::Rocket => rocket_impact(w, bullet, pos, info),
        ImpactKind::Missile => missile_impact(w, bullet, pos, info),
    }
}

// ---------------------------------------------------------------------------
// Impact functions
// ---------------------------------------------------------------------------

fn bouncing_impact(w: &mut GameWorld, bullet: &Projectile, pos: Vec2, info: CollisionInfo) {
    let normal = info.normal;
    let new_vel = bullet.current_velocity() - 2. * bullet.current_velocity().dot(normal) * normal;
    let mut clone_def = bullet.def.clone();
    clone_def.duration = bullet.lifetime_remaining(w.frame_t) / 2.;
    w.create_projectile(
        pos + new_vel.normalize() * clone_def.length,
        new_vel,
        clone_def,
        bullet.id,
    );
}

fn flak_impact(w: &mut GameWorld, bullet: &Projectile, pos: Vec2, _info: CollisionInfo) {
    let initial_angle = bullet.current_direction().y.atan2(bullet.current_direction().x);
    const PELLETS: i32 = 7;
    const SPEED_FACTOR: f32 = 1.2;
    let bullet_speed = bullet.vel.length();
    let bullet_duration = bullet.duration;
    let bullet_id = bullet.id;
    for i in 0..PELLETS {
        let vel =
            Vec2::from_angle(initial_angle + i as f32 * TAU / PELLETS as f32) * bullet_speed * SPEED_FACTOR;
        w.create_projectile(
            pos,
            vel,
            ProjectileDef {
                name: "Flak Shrapnel".to_string(),
                speed: bullet_speed * SPEED_FACTOR,
                length: 5.,
                duration: bullet_duration / SPEED_FACTOR as f64 / 3.,
                shape: EntityShape::Pellet,
                ..Default::default()
            },
            bullet_id,
        );
    }
}

fn rocket_impact(w: &mut GameWorld, bullet: &Projectile, pos: Vec2, _info: CollisionInfo) {
    let bullet_id = bullet.id;
    w.create_projectile(
        pos,
        Vec2::ZERO,
        ProjectileDef {
            name: "Explosive Payload".to_string(),
            damage: 0.3,
            speed: 0.,
            length: 72.,
            length_curve: LengthCurve::Explosion,
            duration: 0.525,
            shape: EntityShape::Disc,
            persists: true,
            ..Default::default()
        },
        bullet_id,
    );
}

fn missile_impact(w: &mut GameWorld, bullet: &Projectile, pos: Vec2, _info: CollisionInfo) {
    let bullet_id = bullet.id;
    w.create_projectile(
        pos,
        Vec2::ZERO,
        ProjectileDef {
            name: "Explosive Payload".to_string(),
            damage: 0.3,
            speed: 0.,
            length: 42.,
            length_curve: LengthCurve::Explosion,
            duration: 0.525,
            shape: EntityShape::Disc,
            persists: true,
            ..Default::default()
        },
        bullet_id,
    );
}

// ---------------------------------------------------------------------------
// Weapons
// ---------------------------------------------------------------------------

pub fn init_weapons() -> Vec<WeaponDef> {
    let mut weapons = vec![];

    let pistol_ammo = ProjectileDef {
        name: "Parabellum 90mm".to_string(),
        speed: 585.,
        cost: 0.1,
        length: 5.,
        duration: 3. * RADAR_DIST as f64 / 585.,
        ..Default::default()
    };
    let bouncing_ammo = ProjectileDef {
        name: "Bounce Round".to_string(),
        speed: 500.,
        cost: 0.25,
        length: 6.,
        duration: 8.,
        shape: EntityShape::Disc,
        impact_kind: ImpactKind::Bouncing,
        ..Default::default()
    };
    let flak_ammo = ProjectileDef {
        name: "Flak Shell".to_string(),
        speed: 500.,
        cost: 0.15,
        length: 10.,
        duration: 1.5 * RADAR_DIST as f64 / 500.,
        shape: EntityShape::Pellet,
        impact_kind: ImpactKind::Flak,
        death_impact: true,
        ..Default::default()
    };
    let rifle_ammo = ProjectileDef {
        name: "MWTO 762".to_string(),
        damage: 2.,
        speed: 1150.,
        cost: 0.35,
        length: 15.,
        shape: EntityShape::Oblong,
        ..Default::default()
    };
    let burst_rifle_ammo = ProjectileDef {
        name: "MWTO 556".to_string(),
        damage: 1.,
        speed: 1150.,
        cost: 0.25,
        length: 11.,
        shape: EntityShape::Oblong,
        ..Default::default()
    };
    let mg_ammo = ProjectileDef {
        name: "Parabellum 108mm".to_string(),
        speed: 900.,
        cost: 0.2,
        length: 6.,
        ..Default::default()
    };
    let rocket_ammo = ProjectileDef {
        name: "Seeker Rocket".to_string(),
        speed: 420.,
        cost: 0.6,
        length: 20.,
        shape: EntityShape::Oblong,
        homing_turn_rate: PI / 6.,
        impact_kind: ImpactKind::Rocket,
        ..Default::default()
    };
    let missile_ammo = ProjectileDef {
        name: "Homing Missile".to_string(),
        speed: 280.,
        cost: 0.35,
        length: 8.,
        shape: EntityShape::Oblong,
        homing_turn_rate: PI / 3.,
        impact_kind: ImpactKind::Missile,
        ..Default::default()
    };
    let interceptor = ProjectileDef {
        name: "Interceptor".to_string(),
        speed: 225.,
        cost: 0.5,
        length: 10.,
        shape: EntityShape::Pellet,
        duration: 18.,
        homing_turn_rate: PI / 4.,
        ..Default::default()
    };

    weapons.push(WeaponDef {
        name: "Peashooter".to_string(),
        desc: "A gun for hire. What it lacks in performance, it makes up for in grit.".to_string(),
        rental_fee: 10.,
        projectile_stats: pistol_ammo.clone(),
        cd: 0.575,
        magazine: 6,
        reload: 1.7,
        weight: 40.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Winggun".to_string(),
        desc: "".to_string(),
        rental_fee: 0.,
        projectile_stats: pistol_ammo.clone(),
        cd: 0.65,
        magazine: 8,
        reload: 2.05,
        weight: 45.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::TwinShoot,
    });
    weapons.push(WeaponDef {
        name: "Bouncer".to_string(),
        desc: "Projectile ricochets.".to_string(),
        rental_fee: 0.,
        projectile_stats: bouncing_ammo,
        cd: 0.7,
        magazine: 5,
        reload: 2.1,
        weight: 60.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Flak Cannon".to_string(),
        desc: "Projectile shatters into pellets.".to_string(),
        rental_fee: 0.,
        projectile_stats: flak_ammo,
        cd: 1.55,
        magazine: 9,
        reload: 3.4,
        weight: 80.,
        effective_range: RADAR_DIST * 2.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Rifle".to_string(),
        desc: "High-velocity rounds.".to_string(),
        rental_fee: 0.,
        projectile_stats: rifle_ammo,
        cd: 0.875,
        magazine: 8,
        reload: 2.55,
        weight: 55.,
        effective_range: RADAR_DIST * 4.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Burst Rifle".to_string(),
        desc: "Rapid 3-shot burst.".to_string(),
        rental_fee: 0.,
        projectile_stats: burst_rifle_ammo,
        cd: 0.0425,
        magazine: 3,
        reload: 1.375,
        weight: 75.,
        effective_range: RADAR_DIST * 4.,
        shoot_kind: ShootKind::ChamberShootThree,
    });
    weapons.push(WeaponDef {
        name: "Machine Gun".to_string(),
        desc: "More dakka.".to_string(),
        rental_fee: 0.,
        projectile_stats: mg_ammo.clone(),
        cd: 0.09125,
        magazine: 30,
        reload: 3.75,
        weight: 140.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::ChamberShootFive,
    });
    weapons.push(WeaponDef {
        name: "Heavy Machine Gun".to_string(),
        desc: "Even more dakka.".to_string(),
        rental_fee: 0.,
        projectile_stats: mg_ammo,
        cd: 0.0625,
        magazine: 50,
        reload: 5.25,
        weight: 210.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::ChamberShootFive,
    });
    weapons.push(WeaponDef {
        name: "Piercer".to_string(),
        desc: "".to_string(),
        rental_fee: 0.,
        projectile_stats: ProjectileDef {
            name: "Brosnan HEV".to_string(),
            damage: 0.5,
            speed: 720.,
            cost: 1.,
            length: 9.,
            shape: EntityShape::Oblong,
            persists: true,
            ..Default::default()
        },
        cd: 1.375,
        magazine: 12,
        reload: 3.9,
        weight: 230.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Railgun".to_string(),
        desc: "Piercing hitscan weapon. Must charge before firing.".to_string(),
        rental_fee: 0.,
        projectile_stats: ProjectileDef {
            name: "Tungsten Rod".to_string(),
            damage: 0.75,
            speed: 0.,
            cost: 1.5,
            length: 1500.,
            shape: EntityShape::Line,
            persists: true,
            duration: 0.275,
            ..Default::default()
        },
        cd: 1.325,
        magazine: 1,
        reload: 2.65,
        weight: 170.,
        effective_range: RADAR_DIST * 5.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Rocket Launcher".to_string(),
        desc: "Projectiles explode on impact.".to_string(),
        rental_fee: 0.,
        projectile_stats: rocket_ammo,
        cd: 1.275,
        magazine: 6,
        reload: 2.5,
        weight: 120.,
        effective_range: RADAR_DIST * 3.,
        shoot_kind: ShootKind::Default,
    });
    weapons.push(WeaponDef {
        name: "Missile Pod".to_string(),
        desc: "Fires clusters of missiles that explode on impact.".to_string(),
        rental_fee: 0.,
        projectile_stats: missile_ammo,
        cd: 0.275,
        magazine: 4,
        reload: 3.3,
        weight: 100.,
        effective_range: RADAR_DIST * 5.,
        shoot_kind: ShootKind::TwinWideShoot,
    });
    weapons.push(WeaponDef {
        name: "Interceptor Swarm".to_string(),
        desc: "".to_string(),
        rental_fee: 0.,
        projectile_stats: interceptor,
        cd: 0.125,
        magazine: 8,
        reload: 6.,
        weight: 200.,
        effective_range: RADAR_DIST * 5.,
        shoot_kind: ShootKind::Default,
    });

    weapons
}

// ---------------------------------------------------------------------------
// Ships
// ---------------------------------------------------------------------------

pub fn init_ships() -> Vec<ShipDef> {
    let mut ships = vec![];
    ships.push(ShipDef {
        name: "LoanStar".to_string(),
        manufacturer: "Warp Gate AB".to_string(),
        desc: "A cheap drone for Extracteroid use. Offered as a loaner, the STAR-3Xa is a workhorse that even a child could pilot.".to_string(),
        height: 32.,
        width: 24.,
        speed: 0.,
        extraction_time: 0.2,
        turn_rate: PI * 7. / 4.,
        recoil_factor: 1. / 3.,
        reload_factor: 7. / 32.,
        recoil_duration: 0.1125,
        launch_fee: 5.,
        rental_fee: 10.,
        frame_weight: 100.,
        weight_limit: 210.,
        fuel_capacity: 25.,
        kind: ShipKind::LoanStar,
        shape: ShipShape::Triangle,
    });
    ships.push(ShipDef {
        name: "BrittleStar".to_string(),
        manufacturer: "Warp Gate AB".to_string(),
        desc: "The STAR-5Xb is a modified LoanStar with low operating costs. It sports a lightweight frame with improved fuel efficiency.".to_string(),
        height: 32.,
        width: 24.,
        speed: 0.,
        extraction_time: 0.15,
        turn_rate: PI * 7. / 4.,
        recoil_factor: 1. / 3.,
        reload_factor: 7. / 32.,
        recoil_duration: 0.175,
        launch_fee: 5.,
        rental_fee: 0.,
        frame_weight: 70.,
        weight_limit: 210.,
        fuel_capacity: 30.,
        kind: ShipKind::BrittleStar,
        shape: ShipShape::Star,
    });
    ships.push(ShipDef {
        name: "Gunship".to_string(),
        manufacturer: "FAANG Megacorp".to_string(),
        desc: "A nimble drone for daring pilots, yet it pays a heavy price when reloading or firing.".to_string(),
        height: 42.,
        width: 25.,
        speed: 0.,
        extraction_time: 0.15,
        turn_rate: PI * 9. / 4.,
        recoil_factor: 1. / 18.,
        reload_factor: 1. / 5.,
        recoil_duration: 0.0975,
        launch_fee: 15.,
        rental_fee: 0.,
        frame_weight: 125.,
        weight_limit: 325.,
        fuel_capacity: 50.,
        kind: ShipKind::Gunship,
        shape: ShipShape::Diamond,
    });
    ships.push(ShipDef {
        name: "Crabber".to_string(),
        manufacturer: "Aces Co".to_string(),
        desc: "What about crab? With a squat shape and huge weight capacity, the Crabber is a favorite of greedy pilots system-wide.".to_string(),
        height: 23.,
        width: 54.,
        speed: 0.,
        extraction_time: 0.15,
        turn_rate: PI * 3. / 4.,
        recoil_factor: 0.5,
        reload_factor: 0.5,
        recoil_duration: 0.225,
        launch_fee: 10.,
        rental_fee: 0.,
        frame_weight: 180.,
        weight_limit: 400.,
        fuel_capacity: 75.,
        kind: ShipKind::Crabber,
        shape: ShipShape::Crab,
    });
    ships.push(ShipDef {
        name: "BattleStar".to_string(),
        manufacturer: "Warp Gate AB".to_string(),
        desc: "Wing weapons pivot toward target.".to_string(),
        height: 96.,
        width: 72.,
        speed: 0.,
        extraction_time: 0.15,
        turn_rate: PI * 7. / 5.,
        recoil_factor: 0.95,
        reload_factor: 1. / 32.,
        recoil_duration: 0.125,
        launch_fee: 25.,
        rental_fee: 0.,
        frame_weight: 300.,
        weight_limit: 525.,
        fuel_capacity: 100.,
        kind: ShipKind::BattleStar,
        shape: ShipShape::TriangleWithTail,
    });
    ships.push(ShipDef {
        name: "Carrier".to_string(),
        manufacturer: "Snowstorm".to_string(),
        desc: "Missiles automatically track main target.".to_string(),
        height: 64.,
        width: 64.,
        speed: 0.,
        extraction_time: 0.15,
        turn_rate: PI * 7. / 6.,
        recoil_factor: 1. / 3.,
        reload_factor: 7. / 32.,
        recoil_duration: 0.0975,
        launch_fee: 30.,
        rental_fee: 0.,
        frame_weight: 400.,
        weight_limit: 650.,
        fuel_capacity: 125.,
        kind: ShipKind::Carrier,
        shape: ShipShape::Pentagon,
    });

    ships
}

// ---------------------------------------------------------------------------
// Gear
// ---------------------------------------------------------------------------

pub fn init_gear() -> Vec<GearDef> {
    use GearType::*;
    use Rarity::*;
    let mut gear_defs = vec![];

    // --- Resources ---
    gear_defs.push(GearDef {
        name: "Scrap".to_string(),
        value: 0.25,
        weight: 0.5,
        drop_rate: 400,
        instasell: true,
        stackable: true,
        tags: vec![ASTEROID_LOOT.to_string()],
        rarity: Common,
        gear_type: Resource,
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Minerals".to_string(),
        value: 1.,
        weight: 1.,
        drop_rate: 130,
        instasell: true,
        stackable: true,
        rarity: Uncommon,
        gear_type: Resource,
        tags: vec![ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Rare Metals".to_string(),
        value: 5.,
        weight: 2.,
        drop_rate: 40,
        instasell: true,
        stackable: true,
        rarity: Rare,
        gear_type: Resource,
        tags: vec![ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Vespian Gas".to_string(),
        value: 10.,
        weight: 3.,
        drop_rate: 10,
        instasell: true,
        stackable: true,
        rarity: Epic,
        gear_type: Resource,
        tags: vec![ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Gold Minerals".to_string(),
        value: 20.,
        weight: 4.,
        drop_rate: 1,
        instasell: true,
        stackable: true,
        rarity: Legendary,
        gear_type: Resource,
        tags: vec![ASTEROID_LOOT.to_string()],
        ..Default::default()
    });

    // --- Components ---
    gear_defs.push(GearDef {
        name: "Barrel Extender".to_string(),
        desc: "+5% bullet velocity".to_string(),
        value: 0.,
        weight: 15.,
        drop_rate: 4,
        effect: GearEffect::ProjectileSpeed(1.05),
        rarity: Common,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Particle Accelerator".to_string(),
        desc: "+20% bullet velocity".to_string(),
        value: 0.,
        weight: 40.,
        drop_rate: 1,
        effect: GearEffect::ProjectileSpeed(1.2),
        rarity: Rare,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Reloading Servos".to_string(),
        desc: "+5% reload rate".to_string(),
        value: 0.,
        weight: 10.,
        drop_rate: 4,
        effect: GearEffect::ReloadRate(0.95),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Extended Magazine".to_string(),
        desc: "+100% Magazine and Reload Time".to_string(),
        value: 0.,
        weight: 80.,
        drop_rate: 4,
        effect: GearEffect::ExtendedMagazine(2., 2),
        rarity: Rare,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Basic Scope".to_string(),
        desc: "+5% vision".to_string(),
        value: 0.,
        weight: 15.,
        drop_rate: 4,
        effect: GearEffect::Zoom(0.95),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Heavy Scope".to_string(),
        desc: "+10% vision".to_string(),
        value: 0.,
        weight: 25.,
        drop_rate: 1,
        effect: GearEffect::Zoom(0.9),
        rarity: Rare,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Aim Ahead".to_string(),
        desc: "+20 Leading Offset".to_string(),
        value: 0.,
        weight: 10.,
        drop_rate: 7,
        effect: GearEffect::AimLead(20.),
        rarity: Common,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Fragile Barrier".to_string(),
        desc: "+1 Shields at start of mission".to_string(),
        value: 0.,
        weight: 25.,
        drop_rate: 4,
        effect: GearEffect::Shield(1.),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Shield Generator".to_string(),
        desc: "+1 Regenerating Shields".to_string(),
        value: 0.,
        weight: 60.,
        drop_rate: 4,
        effect: GearEffect::ShieldGenerator {
            shield: 1.,
            capacity: 1.,
            regen_rate: 0.1,
            delay_mul: 2.,
        },
        rarity: Rare,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Ship Display Mod".to_string(),
        desc: "Drone/Mission info".to_string(),
        value: 2.,
        weight: 3.,
        drop_rate: 6,
        effect: GearEffect::ShipDiagnostics,
        rarity: Common,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Weapon Display Mod".to_string(),
        desc: "Weapon/Targeting info".to_string(),
        value: 2.,
        weight: 3.,
        drop_rate: 6,
        effect: GearEffect::WeaponDiagnostics,
        rarity: Common,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Fuel Cell".to_string(),
        desc: "+15% fuel efficiency".to_string(),
        value: 5.,
        weight: 5.,
        drop_rate: 4,
        effect: GearEffect::FuelEfficiency(0.15),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Recall Device".to_string(),
        desc: "+20% extraction speed".to_string(),
        weight: 10.,
        drop_rate: 4,
        effect: GearEffect::ExtractionSpeed(1.2),
        rarity: Common,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Insurance Papers".to_string(),
        desc: "+20% extraction insurance".to_string(),
        weight: 3.,
        drop_rate: 4,
        effect: GearEffect::Insurance(0.2),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Anti-Armor Munitions".to_string(),
        desc: "+50% damage".to_string(),
        weight: 10.,
        drop_rate: 4,
        effect: GearEffect::Damage(1.5),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Initiate FCS".to_string(),
        desc: "Primitive lock-on targeting.".to_string(),
        value: 5.,
        weight: 20.,
        drop_rate: 7,
        effect: GearEffect::TrackingAssist([0.65, 0.25, 0.05], 1.05),
        rarity: Uncommon,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Acolyte FCS".to_string(),
        desc: "FCS ideal at midrange shots.".to_string(),
        value: 5.,
        weight: 30.,
        drop_rate: 7,
        effect: GearEffect::TrackingAssist([0.4, 0.7, 0.2], 1.125),
        rarity: Rare,
        tags: vec![GEAR_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        ..Default::default()
    });

    // --- Weapon gear items ---
    gear_defs.push(GearDef {
        name: "Peashooter".to_string(),
        desc: "This is a rental. How'd you get this?".to_string(),
        weight: 40.,
        rarity: Starter,
        gear_type: Weapon,
        mission_tiers: Some(vec![]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Winggun".to_string(),
        desc: "Click to equip weapon.".to_string(),
        value: 10.,
        weight: 45.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Bouncer".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 60.,
        value: 10.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Flak Cannon".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 80.,
        value: 15.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Rifle".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 55.,
        value: 15.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Machine Gun".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 140.,
        value: 20.,
        drop_rate: 4,
        rarity: Rare,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Burst Rifle".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 75.,
        value: 10.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Heavy Machine Gun".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 230.,
        value: 15.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Piercer".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 260.,
        value: 15.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Railgun".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 170.,
        value: 20.,
        drop_rate: 4,
        rarity: Rare,
        gear_type: Weapon,
        effect: GearEffect::RailgunCharge,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Rocket Launcher".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 120.,
        value: 20.,
        drop_rate: 4,
        rarity: Rare,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Missile Pod".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 100.,
        value: 10.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Interceptor Swarm".to_string(),
        desc: "Click to equip weapon.".to_string(),
        weight: 250.,
        value: 20.,
        drop_rate: 4,
        rarity: Rare,
        gear_type: Weapon,
        tags: vec![WEAPON_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });

    // --- Drone gear items ---
    gear_defs.push(GearDef {
        name: "LoanStar".to_string(),
        desc: "This is a rental. How'd you get this?".to_string(),
        weight: 100.,
        rarity: Starter,
        gear_type: Drone,
        mission_tiers: Some(vec![]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "BrittleStar".to_string(),
        desc: "Click to equip drone.".to_string(),
        value: 20.,
        weight: 70.,
        drop_rate: 7,
        rarity: Common,
        gear_type: Drone,
        tags: vec![SHIP_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Gunship".to_string(),
        desc: "Click to equip drone.".to_string(),
        value: 20.,
        weight: 125.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Drone,
        tags: vec![SHIP_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Crabber".to_string(),
        desc: "Click to equip drone.".to_string(),
        value: 20.,
        weight: 180.,
        drop_rate: 4,
        rarity: Common,
        gear_type: Drone,
        tags: vec![SHIP_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![1]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "BattleStar".to_string(),
        desc: "Click to equip drone.".to_string(),
        value: 50.,
        weight: 300.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Drone,
        tags: vec![SHIP_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });
    gear_defs.push(GearDef {
        name: "Carrier".to_string(),
        desc: "Click to equip drone.".to_string(),
        value: 50.,
        weight: 400.,
        drop_rate: 4,
        rarity: Uncommon,
        gear_type: Drone,
        tags: vec![SHIP_LOOT.to_string(), ASTEROID_LOOT.to_string()],
        mission_tiers: Some(vec![2]),
        ..Default::default()
    });

    gear_defs
}

// ---------------------------------------------------------------------------
// Missions
// ---------------------------------------------------------------------------

pub fn init_missions() -> Vec<MissionDef> {
    let mut mission_defs = vec![];

    mission_defs.push(MissionDef {
        name: "Starter Mission".to_string(),
        desc: "Standard asteroid field".to_string(),
        extraction_time: 0.05,
        mission_fee: 0.,
        initial_sides: 15,
        growth: 1.18,
        initial_mult: 1.0,
        mult_growth: 0.25,
        spawn_interval: 13.75,
        max_waves: 5,
        mission_tier: 1,
        tags: vec![STARTER_TAG.to_string()],
        wave_kind: WaveKind::Starter,
    });

    mission_defs.push(MissionDef {
        name: "Mining Mission".to_string(),
        desc: "Many waves of high-velocity asteroids".to_string(),
        extraction_time: 0.1,
        mission_fee: 50.,
        initial_sides: 16,
        growth: 1.1,
        initial_mult: 1.25,
        mult_growth: 0.25,
        wave_kind: WaveKind::Mining,
        spawn_interval: 9.5,
        max_waves: 8,
        mission_tier: 2,
        tags: vec![],
    });

    mission_defs.push(MissionDef {
        name: "Bounty Mission".to_string(),
        desc: "Heavily armored asteroids".to_string(),
        extraction_time: 0.1,
        mission_fee: 50.,
        initial_sides: 12,
        growth: 1.2,
        initial_mult: 1.5,
        mult_growth: 0.5,
        wave_kind: WaveKind::Bounty,
        spawn_interval: 8.,
        max_waves: 5,
        mission_tier: 2,
        tags: vec![],
    });

    mission_defs.push(MissionDef {
        name: "Asteroid Belt".to_string(),
        desc: "Dense field of debris".to_string(),
        extraction_time: 0.1,
        mission_fee: 50.,
        initial_sides: 30,
        growth: 1.2,
        initial_mult: 2.0,
        mult_growth: 0.5,
        wave_kind: WaveKind::Belt,
        spawn_interval: 20.,
        max_waves: 3,
        mission_tier: 2,
        tags: vec![],
    });

    mission_defs.push(MissionDef {
        name: "Barren Space".to_string(),
        desc: "Boring dead space.".to_string(),
        extraction_time: 0.1,
        mission_fee: 0.,
        initial_sides: 15,
        growth: 1.5,
        initial_mult: 1.0,
        mult_growth: 0.0,
        wave_kind: WaveKind::Barren,
        spawn_interval: 30.,
        max_waves: 3,
        mission_tier: 1,
        tags: vec![],
    });

    mission_defs.push(MissionDef {
        name: "Meteor Zone".to_string(),
        desc: "Anomalies detected.".to_string(),
        extraction_time: 0.1,
        mission_fee: 0.,
        initial_sides: 7,
        growth: 1.3,
        initial_mult: 1.0,
        mult_growth: 0.0,
        wave_kind: WaveKind::Meteor,
        spawn_interval: 15.,
        max_waves: 4,
        mission_tier: 1,
        tags: vec![],
    });

    mission_defs
}

// ---------------------------------------------------------------------------
// Wave dispatch
// ---------------------------------------------------------------------------

pub fn dispatch_wave(w: &mut GameWorld, kind: WaveKind) {
    match kind {
        WaveKind::Starter => starter_wave_func(w),
        WaveKind::Mining => mining_wave_func(w),
        WaveKind::Bounty => bounty_wave_func(w),
        WaveKind::Belt => belt_wave_func(w),
        WaveKind::Barren => barren_wave_func(w),
        WaveKind::Meteor => meteor_wave_func(w),
    }
}

// ---------------------------------------------------------------------------
// Wave functions
// ---------------------------------------------------------------------------

fn starter_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let max_waves = mission.def.max_waves;
    let total_sides = mission.total_sides();
    let center = w.center();

    if current_wave == max_waves {
        let pos_array = [(1f32, 1f32), (-1., 1.), (1., -1.), (-1., -1.)];
        for (x, y) in pos_array {
            let rot = w.rng.gen_range_f32(0., TAU);
            let dist_factor = Vec2::new(
                w.rng.gen_range_f32(0.8, 0.9),
                w.rng.gen_range_f32(0.65, 0.8),
            );
            let pos = center + Vec2::new(x, y).normalize() * (GAME_X * dist_factor);
            let over_under_center = Vec2::from_angle(PI / 6.);
            let toward_center =
                rotate_vec2((center - pos).normalize(), over_under_center);
            let vel = toward_center * ASTEROID_SPEED * 2. / 3.;
            let sides: u8 = 10;
            let mut altered_vertices = vec![];
            for side in 0..sides {
                match side {
                    3 => altered_vertices.push((side, 0.7)),
                    8 => altered_vertices.push((side, 0.7)),
                    _ => {}
                }
            }
            w.create_asteroid(pos, vel, 128., PI / 2., sides, altered_vertices, rot, 0, 5);
        }
        return;
    }

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (16, 3, 8)
    } else if total_sides >= 32 {
        (12, 2, 4)
    } else if total_sides >= 21 {
        (8, 1, 2)
    } else {
        (6, 0, 1)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 6.,
            outer_angle: PI / 4.,
            base_speed_modifier: 0.5,
            scaling_speed_modifier: 1.35,
            size_speed_reduction: 0.04,
            base_value: 1,
            base_size: 135.,
            scaling_size: 15.,
        },
    );
}

fn mining_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let max_waves = mission.def.max_waves;
    let total_sides = mission.total_sides();
    let center = w.center();

    if current_wave == max_waves {
        let pos_array = [(1f32, 1f32), (-1., 1.), (1., -1.), (-1., -1.)];
        for (x, y) in pos_array {
            let rot = w.rng.gen_range_f32(0., TAU);
            let dist_factor = Vec2::new(
                w.rng.gen_range_f32(0.8, 0.9),
                w.rng.gen_range_f32(0.65, 0.8),
            );
            let pos = center + Vec2::new(x, y).normalize() * (GAME_X * dist_factor);
            let over_under_center = Vec2::from_angle(PI / 6.);
            let toward_center =
                rotate_vec2((center - pos).normalize(), over_under_center);
            let vel = toward_center * ASTEROID_SPEED * 2. / 3.;
            let sides: u8 = 10;
            let mut altered_vertices = vec![];
            for side in 0..sides {
                if side % 2 == 1 {
                    altered_vertices.push((side, 0.5));
                }
            }
            w.create_asteroid_special(
                pos,
                vel,
                72.,
                PI / 1.5,
                sides,
                altered_vertices,
                rot,
                0,
                5,
                PI / 12.,
            );
        }
        return;
    }

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (21, 3, 16)
    } else if total_sides >= 32 {
        (17, 2, 8)
    } else if total_sides >= 21 {
        (13, 1, 4)
    } else {
        (9, 0, 2)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 8.,
            outer_angle: PI / 2.5,
            base_speed_modifier: 1.8,
            scaling_speed_modifier: 0.8 * 1.8,
            size_speed_reduction: 0.04,
            base_value: 2,
            base_size: 140.,
            scaling_size: 12.5,
        },
    );
}

fn bounty_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let total_sides = mission.total_sides();

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (20, 4, 8)
    } else if total_sides >= 32 {
        (16, 4, 4)
    } else if total_sides >= 21 {
        (12, 3, 2)
    } else {
        (8, 3, 1)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 8.,
            outer_angle: PI / 2.5,
            base_speed_modifier: 1.,
            scaling_speed_modifier: 1.25,
            size_speed_reduction: 0.025,
            base_value: 1,
            base_size: 160.,
            scaling_size: 10.,
        },
    );
}

fn belt_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let total_sides = mission.total_sides();

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (7, 1, 3)
    } else if total_sides >= 32 {
        (6, 1, 2)
    } else if total_sides >= 21 {
        (5, 1, 1)
    } else {
        (4, 1, 0)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 6.,
            outer_angle: PI,
            base_speed_modifier: 1.15,
            scaling_speed_modifier: 1.15,
            size_speed_reduction: 0.025,
            base_value: 1,
            base_size: 72.,
            scaling_size: 8.,
        },
    );
}

fn barren_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let total_sides = mission.total_sides();

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (6, 0, 25)
    } else if total_sides >= 32 {
        (5, 0, 20)
    } else if total_sides >= 21 {
        (4, 0, 15)
    } else {
        (3, 0, 10)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 6.,
            outer_angle: PI / 5.,
            base_speed_modifier: 0.85,
            scaling_speed_modifier: 1.2,
            size_speed_reduction: 0.025,
            base_value: 2,
            base_size: 140.,
            scaling_size: 15.,
        },
    );
}

fn meteor_wave_func(w: &mut GameWorld) {
    let mission = w.mission.as_ref().unwrap();
    let current_wave = mission.current_wave;
    let total_sides = mission.total_sides();

    let (sides_cap, armor_cap, glitch_chance) = if total_sides >= 43 {
        (3, 0, 65)
    } else if total_sides >= 32 {
        (3, 0, 55)
    } else if total_sides >= 21 {
        (3, 0, 45)
    } else {
        (3, 0, 35)
    };
    spawn_asteroids_from_sides(
        w,
        current_wave,
        total_sides,
        SpawnParameters {
            sides_cap,
            armor_cap,
            glitch_chance,
            inner_angle: PI / 8.,
            outer_angle: PI / 4.,
            base_speed_modifier: 0.75,
            scaling_speed_modifier: 1.2,
            size_speed_reduction: 0.025,
            base_value: 2,
            base_size: 160.,
            scaling_size: 12.,
        },
    );
}

// ---------------------------------------------------------------------------
// Spawn helpers
// ---------------------------------------------------------------------------

struct SpawnParameters {
    sides_cap: u8,
    armor_cap: i32,
    glitch_chance: i32,
    inner_angle: f32,
    outer_angle: f32,
    base_speed_modifier: f32,
    scaling_speed_modifier: f32,
    size_speed_reduction: f32,
    base_value: i32,
    base_size: f32,
    scaling_size: f32,
}

fn spawn_asteroids_from_sides(
    w: &mut GameWorld,
    current_wave: i32,
    mut total_sides: i32,
    sp: SpawnParameters,
) {
    let center = w.center();
    while total_sides > 0 {
        let (sides, side_cost, value, r) = if w.rng.gen_range_i32(0, 100) < sp.glitch_chance {
            (1u8, 10i32, 10, 10.)
        } else {
            let sides = w.rng.gen_range_i32(3, sp.sides_cap as i32 + 1) as u8;
            (
                sides,
                sides as i32,
                sp.base_value,
                sp.base_size + sp.scaling_size * sides as f32,
            )
        };
        let armor = w.rng.gen_range_i32(0, sp.armor_cap + 1);
        total_sides -= side_cost;
        total_sides -= armor;
        let rot = w.rng.gen_range_f32(0., TAU);
        let dist_factor = if current_wave == 1 {
            Vec2::new(
                w.rng.gen_range_f32(0.75, 0.8),
                w.rng.gen_range_f32(0.6, 0.65),
            )
        } else {
            Vec2::new(
                w.rng.gen_range_f32(0.8, 0.9),
                w.rng.gen_range_f32(0.65, 0.8),
            )
        };
        let random_dir = Vec2::new(
            w.rng.gen_range_f32(-1., 1.),
            w.rng.gen_range_f32(-1., 1.),
        )
        .normalize();
        let pos = center + random_dir * (GAME_X * dist_factor);
        let sign = if w.rng.gen_range_i32(0, 2) == 0 {
            -1.
        } else {
            1.
        };
        let over_under_center =
            Vec2::from_angle(sign * w.rng.gen_range_f32(sp.inner_angle, sp.outer_angle));
        let toward_center =
            rotate_vec2((center - pos).normalize(), over_under_center);
        let vel = toward_center
            * ((sp.base_speed_modifier * ASTEROID_SPEED
                + sp.scaling_speed_modifier * total_sides as f32)
                * (1.0 - sp.size_speed_reduction * (sides).saturating_sub(3) as f32));
        let altered_vertices = randomize_altered_vertices(sides, 5, &mut w.rng);
        let rot_speed = w.rng.gen_range_f32(-PI / 60., PI / 60.);
        w.create_asteroid(
            pos,
            vel,
            r,
            rot_speed,
            sides,
            altered_vertices,
            rot,
            armor,
            value + armor,
        );
    }
}

fn randomize_altered_vertices(sides: u8, chance: i32, rng: &mut Rng) -> Vec<(u8, f32)> {
    let mut altered_vertices = vec![];
    for side in 0..sides {
        if rng.gen_range_i32(0, 100) <= chance {
            altered_vertices.push((side, rng.gen_range_f32(0.25, 0.75)));
        } else if rng.gen_range_i32(0, 100) <= chance {
            altered_vertices.push((side, rng.gen_range_f32(1.2, 1.5)));
        }
    }
    altered_vertices
}
