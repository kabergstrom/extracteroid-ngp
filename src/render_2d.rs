//! Canvas-based 2D rendering: gameplay view and drone console CRT overlays.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::f32::consts::{PI, TAU};

use glam::Vec2;
use newgameplus_api::canvas::{self, Canvas, Color, PathEl, Shape, StrokeStyle};
use newgameplus_api::*;

use crate::world::*;

// ---------------------------------------------------------------------------
// Color helpers (sRGB)
// ---------------------------------------------------------------------------

fn c_green() -> Color {
    Color::srgb(42. / 255., 250. / 255., 104. / 255.)
}
fn c_yellow() -> Color {
    Color::srgb(218. / 255., 213. / 255., 49. / 255.)
}
fn c_red() -> Color {
    Color::srgb(199. / 255., 114. / 255., 72. / 255.)
}
fn c_blue() -> Color {
    Color::srgb(19. / 255., 76. / 255., 189. / 255.)
}
fn c_green_a(a: f32) -> Color {
    Color::srgba(42. / 255., 250. / 255., 104. / 255., a)
}
fn c_white_a(a: f32) -> Color {
    Color::srgba(1., 1., 1., a)
}
fn c_lightgray_a(a: f32) -> Color {
    Color::srgba(0.75, 0.75, 0.75, a)
}

// ---------------------------------------------------------------------------
// Shape builders
// ---------------------------------------------------------------------------

/// Closed polygon from vertex slice.
fn polygon_shape(verts: &[Vec2]) -> Shape {
    if verts.is_empty() {
        return Shape::Circle {
            cx: 0.,
            cy: 0.,
            r: 0.,
        };
    }
    Shape::Path(
        core::iter::once(PathEl::MoveTo(verts[0].x, verts[0].y))
            .chain(verts[1..].iter().map(|v| PathEl::LineTo(v.x, v.y)))
            .chain(core::iter::once(PathEl::Close))
            .collect(),
    )
}

/// Open polyline (no Close).
fn polyline_shape(verts: &[Vec2]) -> Shape {
    if verts.is_empty() {
        return Shape::Circle {
            cx: 0.,
            cy: 0.,
            r: 0.,
        };
    }
    Shape::Path(
        core::iter::once(PathEl::MoveTo(verts[0].x, verts[0].y))
            .chain(verts[1..].iter().map(|v| PathEl::LineTo(v.x, v.y)))
            .collect(),
    )
}

/// Rotated filled ellipse approximated as a 12-segment polygon.
fn ellipse_shape(cx: f32, cy: f32, rx: f32, ry: f32, angle_rad: f32) -> Shape {
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let segments: usize = 12;
    Shape::Path(
        (0..segments)
            .map(|i| {
                let t = i as f32 * TAU / segments as f32;
                let ex = rx * t.cos();
                let ey = ry * t.sin();
                let x = cx + ex * cos_a - ey * sin_a;
                let y = cy + ex * sin_a + ey * cos_a;
                if i == 0 {
                    PathEl::MoveTo(x, y)
                } else {
                    PathEl::LineTo(x, y)
                }
            })
            .chain(core::iter::once(PathEl::Close))
            .collect(),
    )
}

/// Arc path for progress indicators.
fn arc_shape(center: Vec2, radius: f32, progress: f32) -> Shape {
    let segments: usize = 32;
    let start_angle = -PI / 2.;
    let sweep = -progress.clamp(0., 1.) * TAU;
    Shape::Path(
        (0..=segments)
            .map(|i| {
                let t = i as f32 / segments as f32;
                let angle = start_angle + sweep * t;
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();
                if i == 0 {
                    PathEl::MoveTo(x, y)
                } else {
                    PathEl::LineTo(x, y)
                }
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn draw_polygon(canvas: &mut Canvas, verts: &[Vec2], thickness: f32, color: Color) {
    if verts.len() < 2 {
        return;
    }
    canvas.stroke_shape(
        polygon_shape(verts),
        color,
        StrokeStyle {
            width: thickness,
            ..Default::default()
        },
    );
}

fn draw_polyline(canvas: &mut Canvas, verts: &[Vec2], thickness: f32, color: Color) {
    if verts.len() < 2 {
        return;
    }
    canvas.stroke_shape(
        polyline_shape(verts),
        color,
        StrokeStyle {
            width: thickness,
            ..Default::default()
        },
    );
}

/// Circular progress arc (counter-clockwise from top).
fn draw_progress_arc(
    canvas: &mut Canvas,
    center: Vec2,
    radius: f32,
    thickness: f32,
    progress: f32,
    bg_color: Color,
    fg_color: Color,
) {
    let bg_style = StrokeStyle {
        width: thickness * 2.,
        ..Default::default()
    };
    canvas.stroke_circle(center.x, center.y, radius, bg_color, bg_style);

    if progress <= 0. {
        return;
    }
    let arc_style = StrokeStyle {
        width: thickness * 1.5,
        ..Default::default()
    };
    canvas.stroke_shape(arc_shape(center, radius + thickness * 0.5, progress), fg_color, arc_style);
}

// ---------------------------------------------------------------------------
// Gameplay view MVP
// ---------------------------------------------------------------------------

fn gameplay_mvp(zoom_factor: f32) -> [[f32; 4]; 4] {
    let half_w = GAME_X / zoom_factor;
    let half_h = GAME_Y / zoom_factor;
    // Y-down: bottom > top so positive Y goes downward (macroquad convention)
    glam::Mat4::orthographic_rh(-half_w, half_w, half_h, -half_h, -1.0, 1.0).to_cols_array_2d()
}

// ---------------------------------------------------------------------------
// Main gameplay rendering
// ---------------------------------------------------------------------------

const PARTICLE_FRAME_BUFFER_T: f64 = 0.067;

pub fn render_gameplay(
    canvas: &mut Canvas,
    w: &GameWorld,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
) {
    canvas.set_mvp(gameplay_mvp(w.zoom_factor));

    let green = c_green();
    let yellow = c_yellow();
    let red = c_red();
    let blue = c_blue();
    let frame_t = w.frame_t;

    // ---- Distance markers (concentric circles) ----
    let thin = StrokeStyle {
        width: 1.,
        ..Default::default()
    };
    for r in 1..10 {
        canvas.stroke_circle(0., 0., r as f32 * RADAR_DIST, green, thin);
    }

    // ---- Bullets ----
    for bullet in &w.bullets {
        let color = if bullet.pending_destroy {
            Color::WHITE
        } else {
            green
        };
        let len = bullet.current_length(frame_t);
        match bullet.def.shape {
            EntityShape::Pellet => {
                canvas.fill_circle(bullet.pos.x, bullet.pos.y, len / 2., color);
            }
            EntityShape::Disc => {
                let s = StrokeStyle {
                    width: 2.,
                    ..Default::default()
                };
                canvas.stroke_circle(bullet.pos.x, bullet.pos.y, len / 2., color, s);
            }
            EntityShape::Line | EntityShape::Oblong => {
                let angle = bullet.current_direction().to_angle() + PI / 2.;
                canvas.fill_shape(
                    ellipse_shape(bullet.pos.x, bullet.pos.y, 2., len / 2., angle),
                    color,
                );
            }
        }
    }

    // ---- Particles ----
    for particle in &w.particles {
        let is_fresh = frame_t < particle.spawn_t + PARTICLE_FRAME_BUFFER_T;
        let base = if is_fresh { Color::WHITE } else { green };
        let alpha = if particle.fade_out {
            let t = ((frame_t - particle.spawn_t) as f32 / particle.duration as f32)
                .clamp(0., 1.)
                .powf(0.5);
            lerp_f32(1., 0., t)
        } else {
            1.
        };
        let color = Color::srgba(base.value[0], base.value[1], base.value[2], alpha);

        match particle.shape {
            EntityShape::Pellet => {
                canvas.fill_circle(particle.pos.x, particle.pos.y, particle.length / 2., color);
            }
            EntityShape::Disc => {
                let s = StrokeStyle {
                    width: particle.thickness,
                    ..Default::default()
                };
                canvas.stroke_circle(
                    particle.pos.x,
                    particle.pos.y,
                    particle.length / 2.,
                    color,
                    s,
                );
            }
            EntityShape::Line | EntityShape::Oblong => {
                let angle = particle.rot + PI / 2.;
                canvas.fill_shape(
                    ellipse_shape(
                        particle.pos.x,
                        particle.pos.y,
                        particle.thickness.max(1.),
                        particle.length / 2.,
                        angle,
                    ),
                    color,
                );
            }
        }
    }

    // ---- Asteroids ----
    let ship_target = w.ship.targeting_system.current_target;
    let weapon_diag = w.ship.guidance_system.weapon_diagnostics;
    let can_shoot = w.can_shoot();

    for asteroid in &w.asteroids {
        let e_r = asteroid.e_radius(frame_t);
        let (color, thickness) = if asteroid.pending_destroy {
            (Color::WHITE, 3.)
        } else if weapon_diag && can_shoot && ship_target == Some(asteroid.id) {
            (yellow, 3.)
        } else {
            (green, 3.)
        };

        if asteroid.sides < 3 {
            canvas.fill_circle(asteroid.pos.x, asteroid.pos.y, e_r, color);
        } else {
            draw_polygon(canvas, &asteroid.vertices(frame_t), thickness, color);
        }

        // Armor rings
        for idx in 1..asteroid.value.ceil() as i32 {
            let ring_alpha = if idx <= asteroid.armor.ceil() as i32 {
                1.
            } else {
                0.25
            };
            let ring_color = c_green_a(ring_alpha);
            let ring_r = e_r - 12.5 * idx as f32;
            if ring_r > 0. {
                if asteroid.sides < 3 {
                    canvas.fill_circle(asteroid.pos.x, asteroid.pos.y, ring_r, ring_color);
                } else {
                    draw_polygon(
                        canvas,
                        &asteroid.vertices_with_scale(ring_r / asteroid.radius, frame_t),
                        2.,
                        ring_color,
                    );
                }
            }
        }
    }

    // ---- Loot ----
    for loot in &w.loot {
        if loot.sides < 3 {
            canvas.fill_circle(loot.pos.x, loot.pos.y, loot.radius, green);
        } else {
            let step = TAU / loot.sides as f32;
            let verts: Vec<Vec2> = (0..loot.sides)
                .map(|i| {
                    let a = i as f32 * step + loot.rot + PI / 2.;
                    Vec2::new(
                        loot.pos.x + a.sin() * loot.radius,
                        loot.pos.y - a.cos() * loot.radius,
                    )
                })
                .collect();
            canvas.fill_shape(polygon_shape(&verts), green);
        }

        // Expiration progress bar
        let total_life = loot.expire_t - loot.spawn_t;
        let progress = if total_life > 0. {
            (loot.lifetime_remaining(frame_t) / total_life) as f32
        } else {
            0.
        };
        draw_progress_arc(
            canvas,
            loot.pos,
            loot.radius + 3.,
            3.,
            progress,
            Color::BLACK,
            yellow,
        );

        // Text labels
        let name = "Temporal Loot";
        let nm = ctx.measure_text(font, name, 24.);
        canvas.draw_text(
            font,
            name,
            loot.pos.x - nm.width / 2.,
            loot.pos.y - loot.radius * 2.,
            24.,
            green,
        );

        let time_text = format!("{:.0}s", loot.lifetime_remaining(frame_t));
        let tm = ctx.measure_text(font, &time_text, 24.);
        canvas.draw_text(
            font,
            &time_text,
            loot.pos.x - tm.width / 2.,
            loot.pos.y,
            24.,
            Color::BLACK,
        );

        let claim = if w.extraction_t.is_some() {
            "Claiming..."
        } else {
            "Extract to Claim"
        };
        let cm = ctx.measure_text(font, claim, 24.);
        canvas.draw_text(
            font,
            claim,
            loot.pos.x - cm.width / 2.,
            loot.pos.y + loot.radius * 2.,
            24.,
            green,
        );
    }

    // ---- Ship ----
    if !w.ship.destroyed() {
        let ship_color =
            if weapon_diag && w.main_weapon.as_ref().is_some_and(|wp| wp.reloading()) {
                yellow
            } else {
                green
            };
        for v_set in w.ship.vertex_sets() {
            draw_polygon(canvas, &v_set, 3., ship_color);
        }

        // Extraction pulse
        if let Some(d) = w.extraction_t {
            let pulse = ((d.end_t - frame_t).powi(2) as f32 * 2. * TAU)
                .cos()
                .round()
                .max(0.);
            canvas.fill_circle(
                w.ship.pos.x,
                w.ship.pos.y,
                w.ship.bounding_radius() * 1.2,
                c_white_a(pulse),
            );
        }
    }

    // ---- Shields ----
    let shield_floor = w.ship.shield_system.shield.floor() as i32;
    let shield_cap = w.ship.shield_system.shield_capacity.floor() as i32;
    for idx in 1..=shield_floor.max(shield_cap) {
        let shield = w.ship.shield_system.shield;
        if shield < idx as f64 && shield > (idx - 1) as f64 {
            // Partially-regenerated layer: split vertices into regen / unregen
            let verts = w.ship.shield_vertices(idx);
            let frac = shield.fract();
            let sides = w.ship.shield_system.sides as usize;
            let split_idx = ((frac * sides as f64) as usize).min(verts.len());
            if split_idx > 0 && split_idx < verts.len() {
                // Connect the two segments at the boundary
                let mut regen: Vec<Vec2> = verts[..split_idx].to_vec();
                regen.push(verts[split_idx]);
                draw_polyline(canvas, &regen, 1.5, c_lightgray_a(0.65));
                let mut unregen: Vec<Vec2> = verts[split_idx..].to_vec();
                unregen.push(verts[0]);
                draw_polyline(canvas, &unregen, 1.5, c_lightgray_a(0.25));
            } else {
                let alpha = if split_idx > 0 { 0.65 } else { 0.25 };
                draw_polygon(canvas, &verts, 1.5, c_lightgray_a(alpha));
            }
        } else {
            let color = if idx <= shield_floor {
                blue
            } else if idx == shield_floor + 1
                && w.ship.shield_system.last_struck_t + PARTICLE_FRAME_BUFFER_T >= frame_t
            {
                Color::srgb(6. / 255., 138. / 255., 249. / 255.)
            } else {
                c_lightgray_a(0.25)
            };
            draw_polygon(canvas, &w.ship.shield_vertices(idx), 1.5, color);
        }
    }

    // ---- Mission start helper ("You" label + arrow) ----
    if let Some(ref mission) = w.mission {
        let total_helper_t = 2.5_f64;
        let helper_fadeout_t = 0.5_f64;
        if mission.mission_start_t + total_helper_t > frame_t {
            let phase = lerp_f32(
                0.,
                1.,
                ((frame_t + helper_fadeout_t - total_helper_t - mission.mission_start_t) as f32
                    / helper_fadeout_t as f32)
                    .clamp(0., 1.),
            );
            let pulse_alpha = (phase * TAU * 8.).cos().round().max(0.);
            let hc = c_white_a(pulse_alpha);

            let label = "You";
            let lm = ctx.measure_text(font, label, 24.);
            let x = w.ship.pos.x - lm.width / 2.;
            let y = w.ship.pos.y - lm.height * 1.5 - w.ship.bounding_radius() * 2.;
            canvas.draw_text(font, label, x, y, 24., hc);

            // Arrow pointing down
            let tri = [
                Vec2::new(w.ship.pos.x - lm.width * 0.25, y + lm.height * 0.5),
                Vec2::new(w.ship.pos.x + lm.width * 0.25, y + lm.height * 0.5),
                Vec2::new(w.ship.pos.x, y + lm.height),
            ];
            canvas.fill_shape(polygon_shape(&tri), hc);
        }
    }

    // ---- HUD text ----
    let gains = w.cargo_value(true) + w.bounty();
    let misc_losses = round_with_decimals(w.losses(), 2);
    let expenses = round_with_decimals(w.total_expense_value(), 2);
    let net = gains - misc_losses - expenses;
    let sym = if net > 0. { "+" } else { "" };
    let bal_text = format!("Mission balance: {}{:.2}", sym, net);
    let bal_color = if net < 0. { red } else { yellow };

    let x_left = -GAME_X / w.zoom_factor / 1.075;
    let y_top = -GAME_Y / w.zoom_factor / 1.175;

    canvas.draw_text(font, &bal_text, x_left, y_top, 48., bal_color);

    if let Some(ref mission) = w.mission {
        let mult_text = format!("Drop multiplier: {:.2}x", mission.loot_multiplier());
        canvas.draw_text(font, &mult_text, x_left, y_top + 60., 48., yellow);
    }

    let danger: i32 = w
        .asteroids
        .iter()
        .map(|a| a.sides as i32 + a.armor as i32)
        .sum();
    let danger_text = format!("Current Danger: {}", danger);
    let y_bot = GAME_Y / w.zoom_factor;
    canvas.draw_text(font, &danger_text, x_left, y_bot - 60., 48., red);
}

// ---------------------------------------------------------------------------
// Drone console CRT
// ---------------------------------------------------------------------------

pub fn render_drone_console(
    canvas: &mut Canvas,
    w: &GameWorld,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    width: f32,
    height: f32,
) {
    // Pixel-space ortho for the console render target
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let green = c_green();
    let font_size = 20.;
    let line_height = font_size * 1.4;
    let bar_width = width * 0.55;
    let label_width = width * 0.28;
    let x0 = 10.;
    let mut y = font_size + 10.;

    // Weight
    let enc = w.encumberance();
    let enc_suffix = if w.ship.guidance_system.ship_diagnostics {
        format!(" (+{:.1}% Enc)", enc * 100.)
    } else if w.total_weight() > w.ship.def.weight_limit {
        " (#OVER#)".into()
    } else {
        String::new()
    };
    let wt = format!(
        "Weight: {:.0}/{}u{}",
        w.total_weight(),
        w.ship.def.weight_limit,
        enc_suffix,
    );
    canvas.draw_text(font, &wt, x0, y, font_size, green);
    y += line_height;

    // Ship diagnostics
    if w.ship.guidance_system.ship_diagnostics {
        let tw = w.total_weight();
        let tt = format!("Turn Rate: {:.2}rad/s", w.ship.current_turn_rate(tw));
        canvas.draw_text(font, &tt, x0, y, font_size, green);
        y += line_height;

        let vt = format!(
            "Vision: +{:.2}%",
            (ZOOM_FACTOR - w.zoom_factor) / ZOOM_FACTOR,
        );
        canvas.draw_text(font, &vt, x0, y, font_size, green);
        y += line_height;
    }

    // Weapon diagnostics
    if w.ship.guidance_system.weapon_diagnostics {
        if let Some(ref weapon) = w.main_weapon {
            let (hit, shot) = (weapon.shots_hit, weapon.shots_fired);
            let acc = if shot > 0 {
                hit as f32 / shot as f32 * 100.
            } else {
                0.
            };
            let at = format!("Direct Hits (Acc): {}/{} ({:.1}%)", hit, shot, acc);
            canvas.draw_text(font, &at, x0, y, font_size, green);
            y += line_height;
        }
    }

    y += line_height * 0.5;

    // Fuel bar
    let fuel_pct = if w.ship.def.fuel_capacity > 0. {
        (w.ship.fuel / w.ship.def.fuel_capacity) as f32
    } else {
        0.
    };
    let fuel_val = if w.ship.guidance_system.ship_diagnostics {
        format!("{:.1}/{:.0}", w.ship.fuel, w.ship.def.fuel_capacity)
    } else {
        String::new()
    };
    y += draw_crt_bar(
        canvas, ctx, font, "Fuel", fuel_pct, &fuel_val, x0, y, label_width, bar_width, green,
        Color::WHITE, font_size,
    );

    // Ammo bar
    if let Some(ref weapon) = w.main_weapon {
        let ammo = weapon.ammo.max(0);
        let max_ammo = weapon.def.magazine;
        let ammo_pct = if max_ammo > 0 {
            ammo as f32 / max_ammo as f32
        } else {
            0.
        };
        let ammo_val = if w.ship.guidance_system.weapon_diagnostics {
            format!("{}/{}", ammo, max_ammo)
        } else {
            String::new()
        };
        y += draw_crt_bar(
            canvas, ctx, font, "Ammo", ammo_pct, &ammo_val, x0, y, label_width, bar_width, green,
            Color::WHITE, font_size,
        );

        // Reload bar
        if weapon.reloading() {
            let elapsed = (w.frame_t - weapon.last_reload) as f32;
            let reload_pct = elapsed / weapon.def.reload;
            let reload_val = if w.ship.guidance_system.weapon_diagnostics {
                format!("{:.1}s", weapon.def.reload - elapsed)
            } else {
                String::new()
            };
            y += draw_crt_bar(
                canvas,
                ctx,
                font,
                "Reload",
                reload_pct,
                &reload_val,
                x0,
                y,
                label_width,
                bar_width,
                c_yellow(),
                c_yellow(),
                font_size,
            );
        }
    }

    // Shield info
    let shield = w.ship.shield_system.shield;
    let shield_cap = w.ship.shield_system.shield_capacity;
    if shield + shield_cap > 0. && w.ship.guidance_system.ship_diagnostics {
        let st = format!("Shields: {:.1}/{:.0}", shield, shield_cap);
        canvas.draw_text(font, &st, x0, y + line_height, font_size, green);
        y += line_height;

        if shield < shield_cap && w.ship.shield_system.shield_regen_rate != 0. {
            let last = w.ship.shield_system.last_struck_t;
            let delay = w.ship.shield_system.shield_regen_delay;
            if w.frame_t > last + delay {
                let rt = format!("Regen: {:.2}/s", w.ship.shield_system.shield_regen_rate);
                canvas.draw_text(font, &rt, x0, y + line_height, font_size, green);
            } else {
                let dp = (1. - (w.frame_t - last) / delay).clamp(0., 1.) as f32;
                let dc = Color::srgb(0.5, 0.5, 1.);
                y += draw_crt_bar(
                    canvas, ctx, font, "Delay", dp, "", x0, y, label_width, bar_width, dc,
                    Color::WHITE, font_size,
                );
            }
        }
    }
    let _ = y; // suppress unused warning
}

// ---------------------------------------------------------------------------
// CRT progress bar helper
// ---------------------------------------------------------------------------

/// Labeled horizontal progress bar. Returns total height consumed.
fn draw_crt_bar(
    canvas: &mut Canvas,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    label: &str,
    progress: f32,
    value_text: &str,
    x: f32,
    y: f32,
    label_width: f32,
    bar_width: f32,
    label_color: Color,
    fill_color: Color,
    font_size: f32,
) -> f32 {
    let bar_height = font_size * 1.2;

    // Label
    canvas.draw_text(font, label, x, y + bar_height * 0.75, font_size, label_color);

    // Bar background
    let bx = x + label_width;
    canvas.fill_rect(bx, y, bar_width, bar_height, Color::srgb(0.6, 0.6, 0.6));

    // Bar fill
    let fw = bar_width * progress.clamp(0., 1.);
    if fw > 0. {
        canvas.fill_rect(bx, y, fw, bar_height, fill_color);
    }

    // Bar border
    canvas.stroke_rect(
        bx,
        y,
        bar_width,
        bar_height,
        label_color,
        StrokeStyle {
            width: 2.,
            ..Default::default()
        },
    );

    // Value text centered in bar
    if !value_text.is_empty() {
        let m = ctx.measure_text(font, value_text, font_size);
        let tx = bx + (bar_width - m.width) / 2.;
        canvas.draw_text(font, value_text, tx, y + bar_height * 0.75, font_size, Color::BLACK);
    }

    bar_height + font_size * 0.3
}

// ---------------------------------------------------------------------------
// Enterprises CRT screen
// ---------------------------------------------------------------------------

pub fn render_enterprises_tv(
    canvas: &mut Canvas,
    w: &GameWorld,
    _ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    width: f32,
    height: f32,
) {
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let green = c_green();
    let red = c_red();
    let yellow = c_yellow();
    let font_size = 18.;
    let line_h = font_size * 1.4;
    let x0 = 8.;
    let mut y = font_size + 8.;

    canvas.draw_text(font, "EXTRACTEROID Enterprises", x0, y, font_size, green);
    y += line_h * 1.2;

    let bal = w.balance();
    let bal_color = if bal >= 0. { green } else { red };
    let bal_text = format!("Balance: {:.2}c", bal);
    canvas.draw_text(font, &bal_text, x0, y, font_size, bal_color);
    y += line_h;

    let status = if w.render_main_menu() {
        if w.ship.destroyed() {
            "DESTROYED"
        } else if w.extraction_successful() {
            "EXTRACTED"
        } else {
            "DOCKED"
        }
    } else if w.extraction_t.is_some() {
        "EXTRACTING"
    } else {
        "DEPLOYED"
    };
    canvas.draw_text(font, &format!("Status: {}", status), x0, y, font_size, green);
    y += line_h;

    // Ship/weapon selection (when docked)
    if w.render_main_menu() {
        let ship_text = format!("Ship: {}", w.metaprog.cfg_ship_choice);
        canvas.draw_text(font, &ship_text, x0, y, font_size * 0.9, green);
        y += line_h;
        let weapon_text = format!("Weapon: {}", w.metaprog.cfg_weapon_choice);
        canvas.draw_text(font, &weapon_text, x0, y, font_size * 0.9, green);
        y += line_h;
    }

    if !w.render_main_menu() {
        let bounty_text = format!("Bounty: {:.0}c", w.bounty());
        canvas.draw_text(font, &bounty_text, x0, y, font_size, yellow);
        y += line_h;

        let losses_text = format!("Losses: {:.0}c", w.losses());
        canvas.draw_text(font, &losses_text, x0, y, font_size, red);
        y += line_h;

        canvas.draw_text(font, "Expenses:", x0, y, font_size, green);
        y += line_h;
        for entry in &w.expenses {
            let text = format!("{}", entry);
            canvas.draw_text(font, &text, x0 + 4., y, font_size * 0.85, green);
            y += line_h * 0.85;
        }

        if w.extraction_t.is_some() {
            y += line_h * 0.5;
            let remaining = w.extraction_end_t().unwrap_or(0.) - w.frame_t;
            let ext_text = if remaining > 0. {
                format!("Extracting in {:.1}s", remaining)
            } else {
                "Extraction complete!".into()
            };
            canvas.draw_text(font, &ext_text, x0, y, font_size, yellow);
        }
    }
    let _ = y;
}

// ---------------------------------------------------------------------------
// Scanner CRT screen
// ---------------------------------------------------------------------------

pub fn render_scanner_tv(
    canvas: &mut Canvas,
    w: &GameWorld,
    _ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    width: f32,
    height: f32,
) {
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let green = c_green();
    let yellow = c_yellow();
    let red = c_red();
    let font_size = 16.;
    let line_h = font_size * 1.35;
    let x0 = 8.;
    let mut y = font_size + 8.;

    canvas.draw_text(font, "Mission Scanner", x0, y, font_size, green);
    y += line_h * 1.2;

    // Current mission info (while deployed)
    if !w.render_main_menu() {
        if let Some(ref mission) = w.mission {
            let text = format!("Active: {}", mission.def.name);
            canvas.draw_text(font, &text, x0, y, font_size, green);
            y += line_h;

            if w.ship.guidance_system.ship_diagnostics {
                let wave_text = format!(
                    "Wave: {}/{}",
                    mission.current_wave, mission.def.max_waves
                );
                canvas.draw_text(font, &wave_text, x0, y, font_size, green);
                y += line_h;

                let next_wave_t = mission.def.spawn_interval - (w.frame_t - w.last_spawn_t);
                if mission.current_wave < mission.def.max_waves {
                    let nw = format!("Next wave: {:.1}s", next_wave_t.max(0.));
                    canvas.draw_text(font, &nw, x0, y, font_size, green);
                } else {
                    canvas.draw_text(font, "All waves spawned!", x0, y, font_size, yellow);
                }
                y += line_h;
            }
        }
    }

    // Scanning animation
    if w.mission_scanner.scanning {
        let dots = ((w.frame_t - w.mission_scanner.current_scan_t) as i32 % 3) + 1;
        let ellipsis: String = (0..dots).map(|_| '.').collect();
        let text = format!("SCANNING{}", ellipsis);
        canvas.draw_text(font, &text, x0, y, font_size, yellow);
        y += line_h;

        // Progress bar
        let total = w.mission_scanner.scan_conclusion_t - w.mission_scanner.current_scan_t;
        let elapsed = w.frame_t - w.mission_scanner.current_scan_t;
        let progress = if total > 0. { (elapsed / total) as f32 } else { 0. };
        let bar_w = width - 20.;
        let bar_h = 10.;
        canvas.fill_rect(x0, y, bar_w, bar_h, Color::srgb(0.15, 0.15, 0.15));
        canvas.fill_rect(x0, y, bar_w * progress.clamp(0., 1.), bar_h, green);
        y += bar_h + line_h * 0.5;
    }

    // Show scanned mission info (when docked or has time_to_accept)
    if w.render_main_menu() || w.mission_scanner.time_to_accept.is_some() {
        let def = &w.mission_scanner.def;
        canvas.draw_text(font, &def.name, x0, y, font_size, green);
        y += line_h;
        canvas.draw_text(font, &def.desc, x0, y, font_size * 0.9, green);
        y += line_h;
        let stats = format!("Tier {} | {} waves | {:.2}x loot", def.mission_tier, def.max_waves, def.initial_mult);
        canvas.draw_text(font, &stats, x0, y, font_size * 0.9, green);
        y += line_h;
        let ext = format!("Extract: {:.2}s | Fee: {:.2}c", def.extraction_time, def.mission_fee);
        canvas.draw_text(font, &ext, x0, y, font_size * 0.9, green);
        y += line_h;

        // Time-to-accept countdown
        if let Some(tta) = w.mission_scanner.time_to_accept {
            let remaining = (w.mission_scanner.last_scan_t + tta) - w.frame_t;
            let color = if remaining < 5. { red } else { yellow };
            let text = format!("Accept in: {:.1}s", remaining.max(0.));
            canvas.draw_text(font, &text, x0, y, font_size, color);
        }
    }
    let _ = y;
}

// ---------------------------------------------------------------------------
// Cargo CRT screen
// ---------------------------------------------------------------------------

pub fn render_cargo_tv(
    canvas: &mut Canvas,
    w: &GameWorld,
    _ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    width: f32,
    height: f32,
) {
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let green = c_green();
    let yellow = c_yellow();
    let red = c_red();
    let font_size = 15.;
    let line_h = font_size * 1.3;
    let x0 = 8.;
    let mut y = font_size + 6.;

    canvas.draw_text(font, "CARGO HOLD", x0, y, font_size + 2., green);
    y += line_h * 1.2;

    let val = w.cargo_value(true);
    let wt = w.cargo_weight();
    let val_text = format!(
        "Val: {:.2}c  W8: {:.0}u",
        val, wt
    );
    canvas.draw_text(font, &val_text, x0, y, font_size, green);
    y += line_h;

    // Build display list: rentals (if not owned) + cargo
    let cargo = w.cargo();
    let max_visible: usize = 9;
    let total = cargo.len();
    let selected = w.cargo_selected_idx.min(total.saturating_sub(1));
    let mut scroll = w.cargo_scroll_offset.max(0) as usize;

    // Auto-scroll to keep selection visible
    if selected < scroll {
        scroll = selected;
    } else if selected >= scroll + max_visible && total > 0 {
        scroll = selected - max_visible + 1;
    }

    // Scroll up indicator
    if scroll > 0 {
        canvas.draw_text(font, "  ...", x0, y, font_size, Color::srgb(0.5, 0.5, 0.5));
        y += line_h * 0.8;
    }

    for (i, item) in cargo.iter().enumerate().skip(scroll).take(max_visible) {
        let gear = w.def_storage.gear_defs.iter().find(|g| g.name == item.name);
        let prefix = if item.pending_destroy {
            "<LOST>"
        } else if item.pending_sale {
            "<SOLD>"
        } else if item.is_rental {
            "<LOAN>"
        } else if item.is_active {
            "<ACTV>"
        } else if gear.map_or(false, |g| g.equippable() && !item.is_active && item.available()) {
            "<inac>"
        } else {
            ""
        };

        let name_str = if item.name_hidden {
            gear.map_or("UNKNOWN".into(), |g| g.hidden_name())
        } else if gear.map_or(false, |g| g.stackable) && item.count > 1 {
            format!("{}x {}", item.count, item.name)
        } else {
            item.name.clone()
        };

        let is_selected = i == selected;
        let indicator = if is_selected { "> " } else { "  " };
        let color = if item.pending_destroy {
            red
        } else if is_selected {
            yellow
        } else {
            green
        };

        let text = if prefix.is_empty() {
            format!("{}{}", indicator, name_str)
        } else {
            format!("{}{} {}", indicator, prefix, name_str)
        };
        canvas.draw_text(font, &text, x0, y, font_size, color);
        y += line_h;
    }

    // Scroll down indicator
    if scroll + max_visible < total {
        canvas.draw_text(font, "  ...", x0, y, font_size, Color::srgb(0.5, 0.5, 0.5));
    }

    // Action hints for selected item
    if !cargo.is_empty() && selected < cargo.len() {
        let item = &cargo[selected];
        let hint_color = Color::srgb(0.5, 0.8, 0.5);
        let mut hint_y = height - font_size * 0.8;
        let hint_fs = font_size * 0.85;

        if item.name_hidden {
            canvas.draw_text(font, "INTERACT: Reveal", x0, hint_y, hint_fs, hint_color);
        } else if !item.pending_destroy && !item.is_rental && !item.pending_sale {
            let gear = w.def_storage.gear_defs.iter().find(|g| g.name == item.name);
            if gear.map_or(false, |g| g.equippable()) {
                if gear.map_or(false, |g| g.value > 0.) {
                    canvas.draw_text(font, "SELL: Sell item", x0, hint_y, hint_fs, hint_color);
                    hint_y -= line_h;
                }
                canvas.draw_text(font, "INTERACT: Stow", x0, hint_y, hint_fs, hint_color);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Balance CRT screen
// ---------------------------------------------------------------------------

pub fn render_balance_tv(
    canvas: &mut Canvas,
    w: &GameWorld,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    width: f32,
    height: f32,
) {
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let green = c_green();
    let red = c_red();
    let font_size = 22.;
    let line_h = font_size * 2.0;
    let x0 = 10.;
    let mut y = font_size + 15.;

    // Balance
    let bal = w.balance();
    let bal_color = if bal >= 0. { green } else { red };
    let label = "BALANCE:";
    let value = format!("{:.2}c", bal);
    canvas.draw_text(font, label, x0, y, font_size, green);
    let lm = ctx.measure_text(font, &value, font_size);
    canvas.draw_text(font, &value, width - x0 - lm.width, y, font_size, bal_color);
    y += line_h;

    // Rent due
    let label = "RENT DUE:";
    let value = format!("{:.2}c", w.metaprog.rent);
    canvas.draw_text(font, label, x0, y, font_size, green);
    let lm = ctx.measure_text(font, &value, font_size);
    canvas.draw_text(font, &value, width - x0 - lm.width, y, font_size, green);
    y += line_h;

    // Days left
    let days = w.days_until_rent();
    let days_color = if days <= 1 { red } else { green };
    let label = "DAYS LEFT:";
    let value = format!("{}", days);
    canvas.draw_text(font, label, x0, y, font_size, green);
    let lm = ctx.measure_text(font, &value, font_size);
    canvas.draw_text(font, &value, width - x0 - lm.width, y, font_size, days_color);
    let _ = y;
}

// ---------------------------------------------------------------------------
// Extraction countdown overlay (drawn on gameplay canvas)
// ---------------------------------------------------------------------------

pub fn render_extraction_hud(
    canvas: &mut Canvas,
    w: &GameWorld,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
) {
    if let Some(d) = w.extraction_t {
        let remaining = d.end_t - w.frame_t;
        if remaining > 0. {
            let text = format!("EXTRACTING {:.1}s", remaining);
            let font_size = 48.;
            let m = ctx.measure_text(font, &text, font_size);
            let x = -m.width / 2.;
            let y = GAME_Y / w.zoom_factor - 120.;
            canvas.draw_text(font, &text, x, y, font_size, c_yellow());
        }
    }
}

// ---------------------------------------------------------------------------
// Game over overlay (drawn on gameplay canvas)
// ---------------------------------------------------------------------------

pub fn render_game_over(
    canvas: &mut Canvas,
    w: &GameWorld,
    ctx: &(impl RenderCtx + ?Sized),
    font: canvas::FontHandle,
    reason: u8,
) {
    let width = 1024.0_f32;
    let height = 768.0_f32;
    canvas.set_mvp(
        glam::Mat4::orthographic_rh(0., width, height, 0., -1., 1.).to_cols_array_2d(),
    );

    let red = c_red();
    let green = c_green();

    if reason == 1 {
        // Ship destroyed — recycling program narrative
        const LOCALES: &[&str] = &[
            "Soylent Greener", "Infinity Accounting Bureau",
            "OneRepublic Farm Cooperative", "Outer Reaches",
            "MedOrgan MacroProcessor", "Human+",
            "Blacksky Voluntary Company", "Nu U",
            "HyperServitor", "Matrix Ener-Gel",
            "Camp Valiant", "St. Bernard Incinerator Plant",
            "Neo Folsom Penitentiary", "Democratic People's Bank",
        ];
        let locale = LOCALES[w.game_over_locale_idx as usize % LOCALES.len()];
        let elapsed = w.frame_t - w.game_over_t;

        let top = "Congratulations! You have been selected for the";
        let top2 = "EXTRACTEROID\u{00ae} pilot recycling program";
        let top3 = "on account of your performance.";

        let trip = if elapsed >= 5.0 { "one-way trip" } else { "trip" };
        let home = if elapsed >= 5.0 { "forever home" } else { "home" };
        let body = format!(
            "Please enjoy your complementary {} to {},",
            trip, locale,
        );
        let body2 = format!("your new {}. Don't forget to smile.", home);

        let fs = 24.0;
        let mut y = height * 0.25;

        let m = ctx.measure_text(font, top, fs);
        canvas.draw_text(font, top, width / 2.0 - m.width / 2.0, y, fs, red);
        y += fs * 1.5;
        let fs2 = fs + 4.0;
        let m = ctx.measure_text(font, top2, fs2);
        canvas.draw_text(font, top2, width / 2.0 - m.width / 2.0, y, fs2, red);
        y += fs2 * 1.5;
        let m = ctx.measure_text(font, top3, fs);
        canvas.draw_text(font, top3, width / 2.0 - m.width / 2.0, y, fs, red);
        y += fs * 2.5;

        let m = ctx.measure_text(font, &body, fs);
        canvas.draw_text(font, &body, width / 2.0 - m.width / 2.0, y, fs, green);
        y += fs * 1.5;
        let m = ctx.measure_text(font, &body2, fs);
        canvas.draw_text(font, &body2, width / 2.0 - m.width / 2.0, y, fs, green);
        y += fs * 3.0;

        let sub = "Press DEPLOY to continue";
        let m = ctx.measure_text(font, sub, 28.0);
        canvas.draw_text(font, sub, width / 2.0 - m.width / 2.0, y, 28.0, green);
    } else if reason == 2 {
        // Rent default
        let text = "RENT DEFAULT";
        let fs = 48.0;
        let m = ctx.measure_text(font, text, fs);
        canvas.draw_text(font, text, width / 2.0 - m.width / 2.0, height * 0.35, fs, red);

        let sub = "Unable to pay rent. All assets have been reset.";
        let fs2 = 24.0;
        let m = ctx.measure_text(font, sub, fs2);
        canvas.draw_text(font, sub, width / 2.0 - m.width / 2.0, height * 0.35 + 60.0, fs2, green);

        let sub2 = "Press DEPLOY to continue";
        let m = ctx.measure_text(font, sub2, 28.0);
        canvas.draw_text(font, sub2, width / 2.0 - m.width / 2.0, height * 0.35 + 130.0, 28.0, green);
    }
}
