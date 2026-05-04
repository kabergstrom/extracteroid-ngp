use alloc::format;
use alloc::string::ToString;
use core::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::init;
use crate::world::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const STARTING_CASH: f64 = 50.;
const STARTING_RENT: f64 = 500.;
const STARTING_RENT_DURATION: i32 = 7;
const TURN_TOLERANCE_FACTOR: f32 = 0.5;
const LOCK_TOLERANCE: f32 = PI / 12.;

/// Credit-rating-adjusted interest rate: F (0.15) / D (0.10) / C (0.05).
fn interest_level(balance: f64) -> f64 {
    if balance < -200. {
        DEBT_INTEREST * 1.5
    } else if balance < -100. {
        DEBT_INTEREST * 1.0
    } else {
        DEBT_INTEREST * 0.5
    }
}

// ---------------------------------------------------------------------------
// Public: create a fresh GameWorld
// ---------------------------------------------------------------------------

pub fn new_game(seed: u32) -> GameWorld {
    let def_storage = DefStorage {
        ship_defs: init::init_ships(),
        weapon_defs: init::init_weapons(),
        gear_defs: init::init_gear(),
        mission_defs: init::init_missions(),
    };
    let init_ship_def = def_storage
        .ship_defs
        .first()
        .expect("no ship defs")
        .clone();
    let starter_mission = def_storage
        .mission_defs
        .iter()
        .find(|m| m.tags.iter().any(|t| t == STARTER_TAG))
        .expect("no starter mission")
        .clone();
    let scanner = MissionScanner::new(starter_mission, 0.);

    let mut w = GameWorld::new(seed, def_storage, init_ship_def.clone(), scanner);
    w.metaprog.balance = STARTING_CASH;
    w.metaprog.current_day = 1;
    w.metaprog.day_rent_due = 1 + STARTING_RENT_DURATION;
    w.metaprog.rent = STARTING_RENT;
    w.metaprog.cfg_ship_choice = init_ship_def.name.clone();
    w.metaprog.cfg_weapon_choice = w
        .def_storage
        .weapon_defs
        .first()
        .expect("no weapon defs")
        .name
        .clone();
    // Give player starter gear (rental ship + rental weapon)
    w.metaprog.cargo_hold.push(Item {
        name: w.metaprog.cfg_ship_choice.clone(),
        count: 1,
        is_rental: true,
        ..Default::default()
    });
    w.metaprog.cargo_hold.push(Item {
        name: w.metaprog.cfg_weapon_choice.clone(),
        count: 1,
        is_rental: true,
        ..Default::default()
    });
    w
}

// ---------------------------------------------------------------------------
// Public: main game tick (called every frame during gameplay)
// ---------------------------------------------------------------------------

pub fn game_tick(w: &mut GameWorld) {
    if w.render_main_menu() {
        return;
    }

    w.update_ship();
    w.update_weapon();
    w.update_particles();
    w.update_asteroids();
    aim_and_fire(w);
    w.update_bullets();
    w.update_loot();
    w.check_bullet_asteroid_collisions();
    w.check_asteroid_ship_collisions();
    w.spawn_wave();
    w.update_fuel_expense();
}

// ---------------------------------------------------------------------------
// Public: begin extraction
// ---------------------------------------------------------------------------

pub fn try_extract(w: &mut GameWorld) {
    if w.extraction_t.is_none() && !w.ship.destroyed() && !w.render_main_menu() {
        let delay = w.total_extraction_time();
        w.start_extracting(delay);
    }
}

// ---------------------------------------------------------------------------
// Public: deploy into a mission
// ---------------------------------------------------------------------------

pub fn deploy_mission(w: &mut GameWorld) {
    // --- Rent check ---
    if w.days_until_rent() <= 0 {
        if w.metaprog.balance < w.metaprog.rent {
            // Game over — reset
            w.metaprog = DarkMeta::default();
            w.metaprog.balance = STARTING_CASH;
            w.metaprog.current_day = 1;
            w.metaprog.day_rent_due = 1 + STARTING_RENT_DURATION;
            w.metaprog.rent = STARTING_RENT;
            let init_ship = w.def_storage.ship_defs.first().unwrap().name.clone();
            let init_weapon = w.def_storage.weapon_defs.first().unwrap().name.clone();
            w.metaprog.cfg_ship_choice = init_ship.clone();
            w.metaprog.cfg_weapon_choice = init_weapon.clone();
            w.metaprog.cargo_hold.push(Item {
                name: init_ship,
                count: 1,
                is_rental: true,
                ..Default::default()
            });
            w.metaprog.cargo_hold.push(Item {
                name: init_weapon,
                count: 1,
                is_rental: true,
                ..Default::default()
            });
            w.game_over_reason = 2;
            w.force_static_t = Some(w.frame_t + 1.75);
            w.game_over_t = w.frame_t;
            w.game_over_locale_idx = w.rng.gen_range_u32(0, 14) as u8;
            return;
        }
        w.metaprog.balance -= w.metaprog.rent;
        w.metaprog.day_rent_due = w.metaprog.current_day + STARTING_RENT_DURATION;
    }

    // --- Reset state from previous run ---
    if w.ship.destroyed() {
        w.metaprog.gains = 0.;
    }
    w.cleanup_cargo();
    w.cleanup_stash();
    w.new_game = false;

    let ship_choice = w.metaprog.cfg_ship_choice.clone();
    let weapon_choice = w.metaprog.cfg_weapon_choice.clone();

    let ship_def = w.ship_def_by_name(&ship_choice);
    let launch_cost = ship_def.launch_fee;
    let weapon_def = w.weapon_def_by_name(&weapon_choice);
    let rental_fee = ship_def.rental_fee + weapon_def.rental_fee;

    // --- Clear run state ---
    w.clear_run_state();
    w.game_over_reason = 0;
    w.force_static_t = None;
    w.game_over_t = 0.;

    // --- Add expenses ---
    if w.metaprog.balance < 0. {
        let interest = interest_level(w.metaprog.balance);
        w.add_expense(Expense {
            name: format!("Debt Payment {:.0}%", interest * 100.),
            count: 1,
            cost: round_with_decimals(-w.metaprog.balance * interest, 2),
        });
    }
    w.add_expense(Expense {
        name: "Launch Fee".to_string(),
        count: 1,
        cost: launch_cost,
    });
    if rental_fee > 0. {
        w.add_expense(Expense {
            name: "Rental Fee".to_string(),
            count: 1,
            cost: rental_fee,
        });
    }

    // --- Create entities ---
    w.ship = w.make_ship(ship_def);
    w.main_weapon = Some(w.make_weapon(weapon_def));

    let mission_def = w.mission_scanner.def.clone();
    w.mission = Some(w.make_mission(mission_def));

    // Mission fee
    if let Some(ref m) = w.mission {
        if m.def.mission_fee > 0. {
            let fee = m.def.mission_fee;
            w.add_expense(Expense {
                name: "Mission Fee".to_string(),
                count: 1,
                cost: fee,
            });
        }
    }

    // --- Spawn first wave ---
    w.last_spawn_t = 0.;
    w.spawn_wave();

    // --- Activate ship & weapon, run launch ---
    w.try_set_active_ship_and_weapon();
    ship_launch(w);

    // --- Equip cargo ---
    equip_active_cargo(w);

    w.deploy_triggered = false;
}

/// Activate gear effects for all available (non-hidden, non-destroyed) cargo.
fn equip_active_cargo(w: &mut GameWorld) {
    // Collect effects first to avoid borrow issues
    let to_equip: alloc::vec::Vec<(GearEffect, i32)> = w
        .metaprog
        .cargo_hold
        .iter()
        .filter(|item| !item.name_hidden && item.available())
        .filter_map(|item| {
            let gear = w
                .def_storage
                .gear_defs
                .iter()
                .find(|g| g.name == item.name)?;
            if gear.equippable() {
                Some((gear.effect.clone(), item.count))
            } else {
                None
            }
        })
        .collect();

    for (effect, count) in to_equip {
        for _ in 0..count {
            gear_equip(&effect, w);
        }
    }
}

// ---------------------------------------------------------------------------
// Public: handle end of run (called when ship destroyed or extraction succeeds)
// ---------------------------------------------------------------------------

pub fn handle_mission_end(w: &mut GameWorld) -> bool {
    if !w.ship.destroyed() && !w.extraction_successful() {
        return false;
    }

    let liquidation_value = w.cargo_value(true);
    w.update_fuel_expense();

    let payout = if w.ship.destroyed() {
        let loss_factor = (1. - w.extraction_factor())
            * (1. - w.ship.guidance_system.salvage_minimum_factor);
        liquidation_value * (1. - loss_factor)
    } else {
        w.metaprog.gains + liquidation_value
    };

    w.metaprog.balance += round_with_decimals(payout, 2)
        - round_with_decimals(w.metaprog.losses, 2)
        - round_with_decimals(w.total_expense_value(), 2);

    if w.ship.destroyed() {
        w.cargo_pending_destroy();
        // Reset to default ship/weapon
        let init_ship = w.def_storage.ship_defs.first().unwrap().name.clone();
        let init_weapon = w.def_storage.weapon_defs.first().unwrap().name.clone();
        w.metaprog.cfg_ship_choice = init_ship;
        w.metaprog.cfg_weapon_choice = init_weapon;
    } else {
        // Successful extraction: collect all floating loot
        let all_items: alloc::vec::Vec<Item> = w
            .loot
            .iter()
            .flat_map(|l| l.contained_items.clone())
            .collect();
        w.add_cargo(all_items);
    }

    w.cargo_pending_sale();
    w.increment_day();

    // Brief static on mission end (0.225s, matching original)
    w.force_static_t = Some(w.frame_t + 0.225);

    // Scanner: start scanning for next mission
    w.mission_scanner.scanning = false;

    true
}

// ---------------------------------------------------------------------------
// Public: scanner tick — discover new missions while docked
// ---------------------------------------------------------------------------

pub fn scanner_tick(w: &mut GameWorld) {
    // Conclude scanning: pick a random eligible mission
    if w.mission_scanner.scanning && w.frame_t >= w.mission_scanner.scan_conclusion_t {
        let max_tier = (w.metaprog.current_day as u8 / 3 + 1).max(1);
        let current_scanner_name = w.mission_scanner.def.name.clone();
        let current_mission_name = w.mission.as_ref().map(|m| m.def.name.clone());
        let eligible: alloc::vec::Vec<usize> = w
            .def_storage
            .mission_defs
            .iter()
            .enumerate()
            .filter(|(_, def)| {
                !def.tags.iter().any(|t| t == STARTER_TAG)
                    && def.name != current_scanner_name
                    && current_mission_name.as_ref().map_or(true, |n| def.name != *n)
                    && def.mission_tier <= max_tier
            })
            .map(|(i, _)| i)
            .collect();
        if !eligible.is_empty() {
            let pick = w.rng.gen_range_u32(0, eligible.len() as u32) as usize;
            let def = w.def_storage.mission_defs[eligible[pick]].clone();
            let accept_time = w.rng.gen_range_f32(10., 20.) as f64;
            w.mission_scanner = MissionScanner::limited_time(def, accept_time, w.frame_t);
        }
    }
    // Time-to-accept expired: revert to starter mission
    if let Some(tta) = w.mission_scanner.time_to_accept {
        if w.frame_t >= w.mission_scanner.last_scan_t + tta
            && !w.mission_scanner.scanning
        {
            let starter = w
                .def_storage
                .mission_defs
                .iter()
                .find(|m| m.tags.iter().any(|t| t == STARTER_TAG))
                .expect("no starter mission")
                .clone();
            w.mission_scanner = MissionScanner::new(starter, w.frame_t);
        }
    }
}

// ---------------------------------------------------------------------------
// Public: cargo interaction handlers (used by button clicks)
// ---------------------------------------------------------------------------

pub fn handle_cargo_interact(w: &mut GameWorld) {
    if w.cargo().is_empty() {
        return;
    }
    let idx = w.cargo_selected_idx.min(w.cargo().len().saturating_sub(1));
    if idx >= w.cargo().len() {
        return;
    }
    let item = w.cargo()[idx].clone();
    if item.name_hidden {
        w.reveal_cargo(idx);
        w.try_set_active_ship_and_weapon();
    } else if !item.pending_destroy && !item.is_rental {
        let taken = w.take_cargo(idx, 1);
        w.add_stash(alloc::vec![taken]);
        w.try_set_active_ship_and_weapon();
    }
}

pub fn handle_cargo_sell(w: &mut GameWorld) {
    if w.cargo().is_empty() {
        return;
    }
    let idx = w.cargo_selected_idx.min(w.cargo().len().saturating_sub(1));
    if idx >= w.cargo().len() {
        return;
    }
    let item = &w.metaprog.cargo_hold[idx];
    if item.name_hidden || item.is_rental || item.pending_sale {
        return;
    }
    let gear = w.gear_def_by_name(&item.name);
    if gear.equippable() && gear.value > 0. {
        let value = w.try_trade_cargo(idx);
        w.metaprog.balance += value;
        w.try_set_active_ship_and_weapon();
    }
}

pub fn handle_vend_ammo(w: &mut GameWorld) {
    let Some(ref weapon) = w.main_weapon else { return };
    let current = weapon.ammo.max(0);
    let max = weapon.def.magazine;
    let needed = max - current;
    if needed <= 0 { return }
    let cost = needed as f64 * weapon.def.projectile_stats.cost;
    if w.metaprog.balance >= cost {
        w.metaprog.balance -= cost;
        let wp = w.main_weapon.as_mut().unwrap();
        wp.ammo = wp.def.magazine;
        wp.chamber = 0;
        wp.last_shot = 0.;
        wp.last_reload = 0.;
    }
}

pub fn handle_vend_fuel(w: &mut GameWorld) {
    let current = w.ship.fuel;
    let max = w.ship.def.fuel_capacity;
    let needed = max - current;
    if needed <= 0. { return }
    let cost = needed * FUEL_COST;
    if w.metaprog.balance >= cost {
        w.metaprog.balance -= cost;
        w.ship.fuel = max;
    }
}

// ---------------------------------------------------------------------------
// Auto-targeting and firing
// ---------------------------------------------------------------------------

fn aim_and_fire(w: &mut GameWorld) {
    if w.ship.destroyed() || w.main_weapon.is_none() {
        return;
    }

    // --- Read-only phase: gather state ---
    let ship_pos = w.ship.pos;
    let ship_rot = w.ship.rot;
    let can_rotate = w.can_rotate();
    let can_shoot = w.can_shoot();
    let zoom_factor = w.zoom_factor;
    let frame_t = w.frame_t;
    let dt = w.dt();

    let weapon = w.main_weapon.as_ref().unwrap();
    let effective_range_sq = weapon.def.effective_range * weapon.def.effective_range;
    let weapon_ready = weapon.ready_to_fire(frame_t);
    let weapon_chamber = weapon.chamber;
    let recoil_duration = w.ship.def.recoil_duration;
    let last_shot = weapon.last_shot;
    let recoil_factor = w.ship.def.recoil_factor;
    let reload_factor = w.ship.def.reload_factor;
    let ammo = weapon.ammo;
    let total_weight = w.total_weight();
    let turn_rate = w.ship.current_turn_rate(total_weight);

    // --- Find closest visible asteroid ---
    let mut closest: Option<(ElementID, Vec2, Vec2, f32)> = None;
    let mut shortest_dist_sq = f32::MAX;

    for ast in &w.asteroids {
        let e_r = ast.e_radius(frame_t);
        if !within_bounds(ast.pos, e_r, 1. / zoom_factor) {
            continue;
        }
        let dist_sq = ast.pos.distance_squared(ship_pos) - (e_r * 2.).powi(2);
        let angle_tolerance = if can_rotate { TAU } else { LOCK_TOLERANCE };
        let delta = w.delta_angle_to_pos(ship_pos, ship_rot, ast.pos);
        if dist_sq < shortest_dist_sq && delta.abs() <= angle_tolerance {
            shortest_dist_sq = dist_sq;
            closest = Some((ast.id, ast.pos, ast.vel, e_r));
        }
    }

    if let Some((target_id, tar_pos, tar_vel, tar_radius)) = closest {
        // Acquire target
        w.ship.targeting_system.current_target = Some(target_id);

        // Compute lead position and delta angle
        let lead_pos = w.lead_target_pos(ship_pos, ship_rot, turn_rate, tar_pos, tar_vel);
        let delta_angle =
            w.delta_angle_to_target(ship_pos, ship_rot, turn_rate, tar_pos, tar_vel);

        // Calculate rotation with recoil/reload factor
        let recoiling = frame_t - last_shot < recoil_duration as f64;
        let pre_adjust_max_turn = turn_rate * dt;
        let rotation_factor = if !can_rotate {
            0.
        } else if ammo <= 0 {
            reload_factor
        } else if recoiling {
            recoil_factor
        } else {
            1.
        };
        let max_turn = pre_adjust_max_turn * rotation_factor;
        let turn_amount = delta_angle.clamp(-max_turn, max_turn);

        // Apply rotation
        w.ship.rot += turn_amount;

        // Burn fuel
        if pre_adjust_max_turn > 0. {
            let fuel_burn = (turn_amount.abs() / pre_adjust_max_turn * dt * FUEL_BURN_RATE)
                as f64
                / (1. + w.ship.guidance_system.fuel_efficiency_bonus);
            w.ship.fuel = (w.ship.fuel - fuel_burn).max(0.);
        }

        // Turn particles
        if turn_amount.abs() > 0.001 {
            let ship_clone = w.ship.clone();
            let intensity = if max_turn.abs() > 0. {
                turn_amount.abs() / max_turn.abs()
            } else {
                0.
            };
            w.spawn_turn_particles(turn_amount, &ship_clone, intensity);
        }

        // --- Fire decision ---
        let aligned = max_turn.abs() > 0.
            && turn_amount.abs() <= max_turn.abs() * TURN_TOLERANCE_FACTOR;
        let in_range = shortest_dist_sq < effective_range_sq;
        let target_visible = within_bounds(lead_pos, tar_radius, 1. / zoom_factor);

        if aligned
            && (can_rotate || delta_angle.abs() <= LOCK_TOLERANCE)
            && in_range
            && target_visible
            && can_shoot
            && weapon_ready
        {
            let shoot_kind = w.main_weapon.as_ref().unwrap().def.shoot_kind;
            init::dispatch_shoot(w, shoot_kind);
        } else if weapon_chamber > 0 && can_shoot && weapon_ready {
            let shoot_kind = w.main_weapon.as_ref().unwrap().def.shoot_kind;
            init::dispatch_shoot(w, shoot_kind);
        }
    } else {
        // No target
        w.ship.targeting_system.current_target = None;
        // Still finish burst if in chamber
        if weapon_chamber > 0 && can_shoot && weapon_ready {
            let shoot_kind = w.main_weapon.as_ref().unwrap().def.shoot_kind;
            init::dispatch_shoot(w, shoot_kind);
        }
    }
}
