#![no_std]
extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

#[derive(newgameplus_api::NgpModule)]
struct Module;
use newgameplus_api::*;
// Re-export so rafx-generated shader modules can use `crate::ShaderResourceBindingKey`.
pub use newgameplus_api::ShaderResourceBindingKey;

mod shaders;
mod world;
mod init;
mod game;
mod render_2d;
mod room;
use shaders::{crt, mesh3d, static_noise};

static FONT_BYTES: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

// ---------------------------------------------------------------------------
// Render target sizes
// ---------------------------------------------------------------------------

const GP_W: u32 = 1024;
const GP_H: u32 = 768;
const CON_W: u32 = 320;
const CON_H: u32 = 240;
const TV_W: u32 = 320;
const TV_H: u32 = 240;
const BAL_W: u32 = 320;
const BAL_H: u32 = 180;

// ---------------------------------------------------------------------------
// GameState
// ---------------------------------------------------------------------------

pub struct GameState {
    crt_variants: ShaderVariants,
    mesh3d_variants: ShaderVariants,
    static_noise_variants: ShaderVariants,
    font: canvas::FontHandle,
    quad_mesh: MeshAlloc,
    world: world::GameWorld,
    camera: room::FpsCamera,
    grabbed: bool,

    // Physics buttons
    buttons: Vec<physics::PhysicsBody>,

    // 6 CRT screens: raw + CRT post-process pairs
    gameplay_rt: Option<RenderTarget>,
    gameplay_crt_rt: Option<RenderTarget>,
    console_rt: Option<RenderTarget>,
    console_crt_rt: Option<RenderTarget>,
    enterprises_rt: Option<RenderTarget>,
    enterprises_crt_rt: Option<RenderTarget>,
    scanner_rt: Option<RenderTarget>,
    scanner_crt_rt: Option<RenderTarget>,
    cargo_rt: Option<RenderTarget>,
    cargo_crt_rt: Option<RenderTarget>,
    balance_rt: Option<RenderTarget>,
    balance_crt_rt: Option<RenderTarget>,

    // 3D scene
    scene_target: Option<RenderTarget>,
    scene_depth: Option<RenderTarget>,

    // Per-quad colored 1x1 textures for room walls
    wall_textures: Vec<Texture>,
}

fn init_state(ctx: &(impl RenderCtx + ?Sized)) -> GameState {
    let crt_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/crt.cookedshaderpackage"),
    );
    let mesh3d_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/mesh3d.cookedshaderpackage"),
    )
    .depth_test(true);
    let static_noise_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/static_noise.cookedshaderpackage"),
    );

    let font = ctx.register_font(FONT_BYTES, 0);

    // Unit quad mesh: [-0.5, 0.5] in XY, Z=0
    let verts = [
        vertex_formats::VertexPU::new([-0.5, -0.5, 0.0], [0.0, 1.0]),
        vertex_formats::VertexPU::new([0.5, -0.5, 0.0], [1.0, 1.0]),
        vertex_formats::VertexPU::new([0.5, 0.5, 0.0], [1.0, 0.0]),
        vertex_formats::VertexPU::new([-0.5, 0.5, 0.0], [0.0, 0.0]),
    ];
    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
    let channels = VertexChannels::POSITION.union(VertexChannels::UV0);
    let quad_mesh = upload_mesh(ctx, &verts, &indices, channels);

    let seed = ctx.rand_u32();
    let world = game::new_game(seed);

    GameState {
        crt_variants,
        mesh3d_variants,
        static_noise_variants,
        font,
        quad_mesh,
        world,
        camera: room::FpsCamera::default(),
        grabbed: false,
        buttons: Vec::new(),
        gameplay_rt: None,
        gameplay_crt_rt: None,
        console_rt: None,
        console_crt_rt: None,
        enterprises_rt: None,
        enterprises_crt_rt: None,
        scanner_rt: None,
        scanner_crt_rt: None,
        cargo_rt: None,
        cargo_crt_rt: None,
        balance_rt: None,
        balance_crt_rt: None,
        scene_target: None,
        scene_depth: None,
        wall_textures: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// module_init
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub fn module_init(ctx: &dyn RenderCtx, phys: &dyn PhysicsCtx) {
    let state = get_or_insert_state(ctx, || init_state(ctx));
    // Reinitialize function pointers for hot-reload safety
    state.world.def_storage = world::DefStorage {
        ship_defs: init::init_ships(),
        weapon_defs: init::init_weapons(),
        gear_defs: init::init_gear(),
        mission_defs: init::init_missions(),
    };
    let w = &mut state.world;
    let ship_name = w.ship.def.name.clone();
    w.ship.def = w.ship_def_by_name(&ship_name);
    let weapon_name = w.main_weapon.as_ref().map(|wp| wp.def.name.clone());
    if let Some(name) = weapon_name {
        let new_def = w.weapon_def_by_name(&name);
        w.main_weapon.as_mut().unwrap().def = new_def;
    }
    let mission_name = w.mission.as_ref().map(|m| m.def.name.clone());
    if let Some(name) = mission_name {
        let new_def = w.mission_def_by_name(&name);
        w.mission.as_mut().unwrap().def = new_def;
    }
    let scanner_name = w.mission_scanner.def.name.clone();
    w.mission_scanner.def = w.mission_def_by_name(&scanner_name);

    // Create physics buttons (idempotent)
    if state.buttons.is_empty() {
        let defs = room::button_definitions();
        for def in &defs {
            let body = phys.create_body(&physics::BodyDesc {
                shape: physics::ShapeDesc::Sphere { radius: def.radius },
                position: def.position,
                motion_type: physics::MotionType::Static,
                object_layer: physics::ObjectLayer::NonMoving,
                ..Default::default()
            });
            state.buttons.push(body);
        }
    }

    info!("extracteroid module_init");
}

// ---------------------------------------------------------------------------
// module_render: just request rerender (all rendering happens in module_ui)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub fn module_render(ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx, _cf: Format, _df: Format) {
    ctx.request_rerender();
}

// ---------------------------------------------------------------------------
// CRT post-process helper
// ---------------------------------------------------------------------------

fn apply_crt(
    ctx: &dyn RenderCtx,
    crt_variants: &mut ShaderVariants,
    quad_mesh: &MeshAlloc,
    source: RenderTarget,
    target: RenderTarget,
    w: u32,
    h: u32,
    time: f64,
) {
    let uni = staging_write_uniform(
        ctx,
        crt::CrtArgsBlockStd140 {
            args: crt::CrtArgsStd140 {
                resolution: [w as f32, h as f32],
                time: time as f32,
                bend: 0.04,
            },
        },
    );

    let pipeline = crt_variants.pipeline(ctx, VertexChannels::NONE, Format::R8G8B8A8_UNORM, None);

    let mut b = descriptor_set(ctx, pipeline, crt::CRT_UBO.set);
    b.bind_staging_alloc(crt::CRT_UBO.binding, &uni);
    let set1 = b.build();

    let mut b = descriptor_set(ctx, pipeline, crt::TEX.set);
    b.bind_sampler(crt::SMP.binding, ctx.shared_samplers().linear_clamp);
    b.bind_texture(crt::TEX.binding, source.texture);
    let set2 = b.build();

    let mut writer = DrawStreamWriter::new();
    writer.set_pipeline(pipeline);
    writer.set_descriptor_set(1, set1);
    writer.set_descriptor_set(2, set2);
    writer.set_mesh_index_buffer(quad_mesh);
    writer.set_triangle_count(1);
    writer.set_instance_count(1);
    writer.emit(true);

    ctx.submit_draw_stream_to_target(
        writer.finish(),
        &RenderTargetDesc {
            color: target,
            depth: None,
            color_load_op: LoadOp::Clear,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        },
    );
}

// ---------------------------------------------------------------------------
// Static noise overlay helper
// ---------------------------------------------------------------------------

fn apply_static_noise(
    ctx: &dyn RenderCtx,
    noise_variants: &mut ShaderVariants,
    quad_mesh: &MeshAlloc,
    target: RenderTarget,
    w: u32,
    h: u32,
    time: f32,
    intensity: f32,
) {
    let uni = staging_write_uniform(
        ctx,
        static_noise::NoiseArgsBlockStd140 {
            args: static_noise::NoiseArgsStd140 {
                resolution: [w as f32, h as f32],
                time,
                intensity,
            },
        },
    );

    let pipeline = noise_variants.pipeline(ctx, VertexChannels::NONE, Format::R8G8B8A8_UNORM, None);

    let mut b = descriptor_set(ctx, pipeline, static_noise::NOISE_UBO.set);
    b.bind_staging_alloc(static_noise::NOISE_UBO.binding, &uni);
    let set1 = b.build();

    let mut writer = DrawStreamWriter::new();
    writer.set_pipeline(pipeline);
    writer.set_descriptor_set(1, set1);
    writer.set_mesh_index_buffer(quad_mesh);
    writer.set_triangle_count(1);
    writer.set_instance_count(1);
    writer.emit(true);

    ctx.submit_draw_stream_to_target(
        writer.finish(),
        &RenderTargetDesc {
            color: target,
            depth: None,
            color_load_op: LoadOp::DontCare, // blend over existing
            clear_color: [0.0, 0.0, 0.0, 0.0],
        },
    );
}

// ---------------------------------------------------------------------------
// Helper: render a canvas to RT, then CRT-postprocess it
// ---------------------------------------------------------------------------

fn render_canvas_and_crt(
    ctx: &dyn RenderCtx,
    crt_variants: &mut ShaderVariants,
    quad_mesh: &MeshAlloc,
    raw_rt: &mut Option<RenderTarget>,
    crt_rt: &mut Option<RenderTarget>,
    w: u32,
    h: u32,
    time: f64,
    draw_fn: impl FnOnce(&mut canvas::Canvas),
) -> RenderTarget {
    let raw = ensure_render_target(ctx, raw_rt, w, h, Format::R8G8B8A8_UNORM, [0.0, 0.0, 0.0, 1.0]);
    {
        let mut canvas = ctx.acquire_canvas(raw);
        canvas.set_clear_color(canvas::Color::srgba(0.02, 0.01, 0.05, 1.0));
        draw_fn(&mut canvas);
        ctx.submit_canvas(canvas);
    }
    let crt = ensure_render_target(ctx, crt_rt, w, h, Format::R8G8B8A8_UNORM, [0.0, 0.0, 0.0, 1.0]);
    apply_crt(ctx, crt_variants, quad_mesh, raw, crt, w, h, time);
    crt
}

// ---------------------------------------------------------------------------
// module_ui: game logic + viewport rendering
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub fn module_ui(ui: &mut egui::Ui, ctx: &dyn RenderCtx, phys: &dyn PhysicsCtx) {
    let state = get_or_insert_state(ctx, || init_state(ctx));

    // --- Update time from engine clock ---
    let engine_time = ctx.time();
    state.world.update_frame_t(engine_time.time_secs);
    let frame_time = engine_time.time_secs;
    let dt = engine_time.delta_secs;

    // --- Tick game logic ---
    if !state.world.render_main_menu() {
        game::game_tick(&mut state.world);
    }
    if state.world.render_main_menu() {
        game::scanner_tick(&mut state.world);
    }
    game::handle_mission_end(&mut state.world);

    // --- Detect game over transitions ---
    if state.world.ship.destroyed() && state.world.game_over_reason == 0 {
        state.world.game_over_reason = 1;
        state.world.game_over_t = frame_time;
        state.world.game_over_locale_idx = state.world.rng.gen_range_u32(0, 14) as u8;
    }

    // --- Viewport ---
    let cf = Format::B8G8R8A8_UNORM;
    let df = Format::D32_SFLOAT;

    game_viewport::show(ctx, ui, "game", |info, input| {
        // ---- Input registration (idempotent) ----
        input.action(intern("move_fb"), input::ActionKind::Axis1D)
            .bind(input::Binding::Key { key: input::KeyCode::W, scale: 1.0 })
            .bind(input::Binding::Key { key: input::KeyCode::S, scale: -1.0 });
        input.action(intern("move_lr"), input::ActionKind::Axis1D)
            .bind(input::Binding::Key { key: input::KeyCode::D, scale: 1.0 })
            .bind(input::Binding::Key { key: input::KeyCode::A, scale: -1.0 });
        input.action(intern("look"), input::ActionKind::Axis2D)
            .bind(input::Binding::MouseDelta { sensitivity: [0.003, 0.003] });
        input.action(intern("click"), input::ActionKind::Button)
            .bind(input::Binding::MouseButton { button: input::MouseButton::Left });
        input.action(intern("deploy"), input::ActionKind::Button)
            .bind(input::Binding::Key { key: input::KeyCode::Enter, scale: 1.0 });
        input.action(intern("extract"), input::ActionKind::Button)
            .bind(input::Binding::Key { key: input::KeyCode::Space, scale: 1.0 });
        input.action(intern("scroll"), input::ActionKind::Axis1D)
            .bind(input::Binding::Scroll { scale: 1.0 });
        input.action(intern("cycle_weapon"), input::ActionKind::Button)
            .bind(input::Binding::Key { key: input::KeyCode::E, scale: 1.0 });
        input.action(intern("cycle_ship"), input::ActionKind::Button)
            .bind(input::Binding::Key { key: input::KeyCode::Q, scale: 1.0 });
        input.action(intern("escape"), input::ActionKind::Button)
            .bind(input::Binding::Key { key: input::KeyCode::Escape, scale: 1.0 });

        // ---- Query input ----
        let mut layer = input.layer();
        let move_fb = layer.axis1d(intern("move_fb"));
        let move_lr = layer.axis1d(intern("move_lr"));
        let look = layer.axis2d(intern("look"));
        let clicked = layer.button_pressed(intern("click"));
        let deploy_pressed = layer.button_pressed(intern("deploy"));
        let extract_pressed = layer.button_pressed(intern("extract"));
        let scroll = layer.axis1d(intern("scroll"));
        let cycle_weapon_pressed = layer.button_pressed(intern("cycle_weapon"));
        let cycle_ship_pressed = layer.button_pressed(intern("cycle_ship"));
        let escape_pressed = layer.button_pressed(intern("escape"));
        let mouse_pos = layer.mouse_position();
        drop(layer);

        // ---- Cursor grab management ----
        if clicked && !state.grabbed {
            state.grabbed = true;
        }
        if escape_pressed {
            state.grabbed = false;
        }

        // ---- Update FPS camera ----
        if state.grabbed {
            state.camera.update([move_lr, move_fb], look, dt);
        }

        // ---- Handle deploy/extract keys ----
        if deploy_pressed && state.world.render_main_menu() {
            game::deploy_mission(&mut state.world);
        }
        if extract_pressed {
            game::try_extract(&mut state.world);
        }

        // ---- Scroll → cargo dial ----
        if scroll.abs() > 0.01 {
            let cargo_len = state.world.cargo().len() as i32;
            if cargo_len > 0 {
                let new_idx = state.world.cargo_selected_idx as i32 - scroll.signum() as i32;
                state.world.cargo_selected_idx = new_idx.clamp(0, cargo_len - 1) as usize;
            }
        }

        // ---- Sync cargo scroll offset ----
        {
            let max_visible: usize = 9;
            let selected = state.world.cargo_selected_idx;
            let mut offset = state.world.cargo_scroll_offset.max(0) as usize;
            if selected < offset {
                offset = selected;
            } else if selected >= offset + max_visible {
                offset = selected - max_visible + 1;
            }
            state.world.cargo_scroll_offset = offset as i32;
        }

        // ---- Cycle weapon/ship (menu only) ----
        if cycle_weapon_pressed && state.world.render_main_menu() {
            let available: alloc::vec::Vec<alloc::string::String> = state.world.equipped_weapon_slots()
                .iter().map(|(name, _)| name.clone()).collect();
            if available.len() > 1 {
                let current = &state.world.metaprog.cfg_weapon_choice;
                let idx = available.iter().position(|n| n == current).unwrap_or(0);
                let next = (idx + 1) % available.len();
                state.world.metaprog.cfg_weapon_choice = available[next].clone();
                state.world.try_set_active_ship_and_weapon();
            }
        }
        if cycle_ship_pressed && state.world.render_main_menu() {
            let available: alloc::vec::Vec<alloc::string::String> = state.world.equipped_ship_slots()
                .iter().map(|(name, _)| name.clone()).collect();
            if available.len() > 1 {
                let current = &state.world.metaprog.cfg_ship_choice;
                let idx = available.iter().position(|n| n == current).unwrap_or(0);
                let next = (idx + 1) % available.len();
                state.world.metaprog.cfg_ship_choice = available[next].clone();
                state.world.try_set_active_ship_and_weapon();
            }
        }

        // ---- Raycast for button clicks ----
        if clicked {
            let vp_size = info.size;
            let aspect = vp_size[0] / vp_size[1];
            let (origin, dir) = room::screen_to_ray(mouse_pos, vp_size, &state.camera, aspect);

            if let Some(hit) = phys.cast_ray(origin, dir, 50.0) {
                // Match hit body to button
                let button_defs = room::button_definitions();
                for (i, body) in state.buttons.iter().enumerate() {
                    if hit.body == *body && i < button_defs.len() {
                        match button_defs[i].action {
                            room::ButtonAction::Deploy => {
                                if state.world.render_main_menu() {
                                    game::deploy_mission(&mut state.world);
                                }
                            }
                            room::ButtonAction::Extract => {
                                game::try_extract(&mut state.world);
                            }
                            room::ButtonAction::Scan => {
                                if !state.world.mission_scanner.scanning {
                                    state.world.init_scan();
                                }
                            }
                            room::ButtonAction::CargoDial => {
                                let cargo_len = state.world.cargo().len() as i32;
                                if cargo_len > 0 {
                                    let new_idx = (state.world.cargo_selected_idx as i32 + 1) % cargo_len;
                                    state.world.cargo_selected_idx = new_idx as usize;
                                }
                            }
                            room::ButtonAction::CargoInteract => {
                                game::handle_cargo_interact(&mut state.world);
                            }
                            room::ButtonAction::CargoSell => {
                                game::handle_cargo_sell(&mut state.world);
                            }
                            room::ButtonAction::VendAmmo => {
                                game::handle_vend_ammo(&mut state.world);
                            }
                            room::ButtonAction::VendFuel => {
                                game::handle_vend_fuel(&mut state.world);
                            }
                        }
                        break;
                    }
                }
            }
        }

        // ================================================================
        // Rendering — 6 canvas+CRT passes, then 3D scene
        // ================================================================

        // ---- 1. Gameplay ----
        let gp_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.gameplay_rt, &mut state.gameplay_crt_rt,
            GP_W, GP_H, frame_time,
            |canvas| {
                if !state.world.render_main_menu() {
                    render_2d::render_gameplay(canvas, &state.world, ctx, state.font);
                    render_2d::render_extraction_hud(canvas, &state.world, ctx, state.font);
                } else if state.world.game_over_reason > 0 {
                    render_2d::render_game_over(canvas, &state.world, ctx, state.font, state.world.game_over_reason);
                } else {
                    // Title screen
                    canvas.set_mvp(
                        glam::Mat4::orthographic_rh(0., GP_W as f32, GP_H as f32, 0., -1., 1.).to_cols_array_2d(),
                    );
                    let text = "EXTRACTEROID";
                    let metrics = ctx.measure_text(state.font, text, 64.0);
                    canvas.draw_text(
                        state.font, text,
                        GP_W as f32 / 2.0 - metrics.width / 2.0,
                        GP_H as f32 / 2.0,
                        64.0,
                        canvas::Color::srgb(0.0, 1.0, 0.4),
                    );
                    let sub = "Press DEPLOY to begin";
                    let sm = ctx.measure_text(state.font, sub, 32.0);
                    canvas.draw_text(
                        state.font, sub,
                        GP_W as f32 / 2.0 - sm.width / 2.0,
                        GP_H as f32 / 2.0 + 80.0,
                        32.0,
                        canvas::Color::srgb(0.0, 0.8, 0.3),
                    );
                }
            },
        );

        // Static noise overlay on gameplay CRT if force_static_t active
        if let Some(end_t) = state.world.force_static_t {
            if frame_time < end_t {
                let intensity = ((end_t - frame_time) / 1.75).clamp(0.0, 1.0) as f32;
                apply_static_noise(
                    ctx, &mut state.static_noise_variants, &state.quad_mesh,
                    gp_crt, GP_W, GP_H, frame_time as f32, intensity,
                );
            } else {
                state.world.force_static_t = None;
            }
        }

        // ---- 2. Console ----
        let con_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.console_rt, &mut state.console_crt_rt,
            CON_W, CON_H, frame_time,
            |canvas| {
                if !state.world.render_main_menu() {
                    render_2d::render_drone_console(
                        canvas, &state.world, ctx, state.font,
                        CON_W as f32, CON_H as f32,
                    );
                }
            },
        );

        // ---- 3. Enterprises ----
        let ent_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.enterprises_rt, &mut state.enterprises_crt_rt,
            TV_W, TV_H, frame_time,
            |canvas| {
                render_2d::render_enterprises_tv(
                    canvas, &state.world, ctx, state.font,
                    TV_W as f32, TV_H as f32,
                );
            },
        );

        // ---- 4. Scanner ----
        let scan_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.scanner_rt, &mut state.scanner_crt_rt,
            TV_W, TV_H, frame_time,
            |canvas| {
                render_2d::render_scanner_tv(
                    canvas, &state.world, ctx, state.font,
                    TV_W as f32, TV_H as f32,
                );
            },
        );

        // ---- 5. Cargo ----
        let cargo_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.cargo_rt, &mut state.cargo_crt_rt,
            TV_W, TV_H, frame_time,
            |canvas| {
                render_2d::render_cargo_tv(
                    canvas, &state.world, ctx, state.font,
                    TV_W as f32, TV_H as f32,
                );
            },
        );

        // ---- 6. Balance ----
        let bal_crt = render_canvas_and_crt(
            ctx, &mut state.crt_variants, &state.quad_mesh,
            &mut state.balance_rt, &mut state.balance_crt_rt,
            BAL_W, BAL_H, frame_time,
            |canvas| {
                render_2d::render_balance_tv(
                    canvas, &state.world, ctx, state.font,
                    BAL_W as f32, BAL_H as f32,
                );
            },
        );

        // ================================================================
        // 3D Scene: room walls + 6 TV screen quads + button labels
        // ================================================================

        let vp_w = info.size_pixels[0].max(1);
        let vp_h = info.size_pixels[1].max(1);
        let aspect = vp_w as f32 / vp_h as f32;

        let scene_rt = ensure_render_target(
            ctx, &mut state.scene_target, vp_w, vp_h, cf, [0.02, 0.01, 0.05, 1.0],
        );
        let scene_depth = ensure_depth_target(ctx, &mut state.scene_depth, vp_w, vp_h, df);

        let view_proj = state.camera.view_proj(aspect);

        let uni = staging_write_uniform(
            ctx,
            mesh3d::Mesh3dArgsBlockStd140 {
                args: mesh3d::Mesh3dArgsStd140 {
                    view_proj: view_proj.to_cols_array_2d(),
                },
            },
        );

        let mesh3d_pipeline = state.mesh3d_variants.pipeline(
            ctx, state.quad_mesh.channels, cf, Some(df),
        );

        // Shared UBO + vertex buffer (set 1)
        let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::MESH3D_UBO.set);
        b.bind_staging_alloc(mesh3d::MESH3D_UBO.binding, &uni);
        b.bind_mesh_vertices(mesh3d::VERTEX_BUFFER.binding, &state.quad_mesh);
        let set_ubo = b.build();

        // Per-quad colored wall textures (lazily created)
        if state.wall_textures.is_empty() {
            for quad in &room::room_quads() {
                let c = quad.color;
                let rgba = [
                    (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                ];
                state.wall_textures.push(ctx.upload_texture(1, 1, Format::R8G8B8A8_UNORM, &rgba));
            }
        }

        let mut writer = DrawStreamWriter::new();
        writer.set_pipeline(mesh3d_pipeline);
        writer.set_descriptor_set(1, set_ubo);
        writer.set_mesh_index_buffer(&state.quad_mesh);
        let tri_count = state.quad_mesh.index_count / 3;

        // ---- Room wall quads ----
        let room = room::room_quads();
        for (qi, quad) in room.iter().enumerate() {
            let mut inst = staging_alloc_slice::<[[f32; 4]; 4]>(ctx, 1);
            inst.write(quad.model.to_cols_array_2d());

            let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::INSTANCE_BUFFER.set);
            b.bind_slice_writer(mesh3d::INSTANCE_BUFFER.binding, &inst);
            let set_inst = b.build();

            let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::TEX.set);
            b.bind_sampler(mesh3d::SMP.binding, ctx.shared_samplers().linear_clamp);
            b.bind_texture(mesh3d::TEX.binding, state.wall_textures[qi]);
            let set_tex = b.build();

            writer.set_descriptor_set(0, set_inst);
            writer.set_descriptor_set(2, set_tex);
            writer.set_triangle_count(tri_count);
            writer.set_instance_count(1);
            writer.emit(false);
        }

        // ---- TV screen quads ----
        let tv_slots = room::tv_slots();
        let crt_textures = [
            gp_crt.texture,
            con_crt.texture,
            ent_crt.texture,
            scan_crt.texture,
            cargo_crt.texture,
            bal_crt.texture,
        ];

        for (slot, &tex) in tv_slots.iter().zip(crt_textures.iter()) {
            let mut inst = staging_alloc_slice::<[[f32; 4]; 4]>(ctx, 1);
            inst.write(slot.model.to_cols_array_2d());

            let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::INSTANCE_BUFFER.set);
            b.bind_slice_writer(mesh3d::INSTANCE_BUFFER.binding, &inst);
            let set_inst = b.build();

            let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::TEX.set);
            b.bind_sampler(mesh3d::SMP.binding, ctx.shared_samplers().linear_clamp);
            b.bind_texture(mesh3d::TEX.binding, tex);
            let set_tex = b.build();

            writer.set_descriptor_set(0, set_inst);
            writer.set_descriptor_set(2, set_tex);
            writer.set_triangle_count(tri_count);
            writer.set_instance_count(1);
            writer.emit(false);
        }

        ctx.submit_draw_stream_to_target(
            writer.finish(),
            &RenderTargetDesc {
                color: scene_rt,
                depth: Some(scene_depth),
                color_load_op: LoadOp::Clear,
                clear_color: [0.02, 0.01, 0.05, 1.0],
            },
        );

        // ---- Button labels (projected to screen, drawn on overlay canvas) ----
        {
            let mut canvas = ctx.acquire_canvas(scene_rt);
            canvas.set_mvp(
                glam::Mat4::orthographic_rh(0., vp_w as f32, vp_h as f32, 0., -1., 1.)
                    .to_cols_array_2d(),
            );

            let button_defs = room::button_definitions();
            for def in &button_defs {
                let label_pos = def.position + glam::Vec3::new(0.0, 0.65, 0.0);
                let clip = view_proj * label_pos.extend(1.0);
                if clip.w <= 0.0 {
                    continue;
                }
                let ndc = glam::Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
                let sx = (ndc.x * 0.5 + 0.5) * vp_w as f32;
                let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * vp_h as f32;

                let font_size = 16.0;
                let m = ctx.measure_text(state.font, def.label, font_size);
                let col = canvas::Color::srgba(def.color[0], def.color[1], def.color[2], 0.9);
                canvas.draw_text(state.font, def.label, sx - m.width / 2.0, sy, font_size, col);

                // Price labels for vending buttons
                match def.action {
                    room::ButtonAction::VendAmmo => {
                        if let Some(ref weapon) = state.world.main_weapon {
                            let needed = weapon.def.magazine - weapon.ammo.max(0);
                            let cost = needed as f64 * weapon.def.projectile_stats.cost;
                            let price = format!("{:.2}c", cost);
                            let pm = ctx.measure_text(state.font, &price, font_size - 2.0);
                            canvas.draw_text(state.font, &price, sx - pm.width / 2.0, sy + font_size + 2.0, font_size - 2.0, canvas::Color::WHITE);
                        }
                    }
                    room::ButtonAction::VendFuel => {
                        let needed = state.world.ship.def.fuel_capacity - state.world.ship.fuel;
                        let cost = needed * world::FUEL_COST;
                        let price = format!("{:.2}c", cost);
                        let pm = ctx.measure_text(state.font, &price, font_size - 2.0);
                        canvas.draw_text(state.font, &price, sx - pm.width / 2.0, sy + font_size + 2.0, font_size - 2.0, canvas::Color::WHITE);
                    }
                    _ => {}
                }

                // Draw a small filled circle at button screen position
                let btn_clip = view_proj * def.position.extend(1.0);
                if btn_clip.w > 0.0 {
                    let btn_ndc = glam::Vec3::new(
                        btn_clip.x / btn_clip.w,
                        btn_clip.y / btn_clip.w,
                        btn_clip.z / btn_clip.w,
                    );
                    let bx = (btn_ndc.x * 0.5 + 0.5) * vp_w as f32;
                    let by = (1.0 - (btn_ndc.y * 0.5 + 0.5)) * vp_h as f32;
                    let screen_r = (def.radius * 40.0 / btn_clip.w).clamp(3.0, 30.0);
                    canvas.fill_circle(bx, by, screen_r, col);
                }
            }

            // ---- Crosshair (X-shaped reticle at screen center) ----
            {
                let cx = vp_w as f32 / 2.0;
                let cy = vp_h as f32 / 2.0;
                let r = 8.0_f32;
                let rc = canvas::Color::srgba(1.0, 1.0, 1.0, 0.7);
                let line1: canvas::Shape = canvas::Shape::Path(
                    [canvas::PathEl::MoveTo(cx - r, cy - r), canvas::PathEl::LineTo(cx + r, cy + r)]
                        .into_iter().collect(),
                );
                canvas.stroke_shape(line1, rc, canvas::StrokeStyle { width: 2.0, ..Default::default() });
                let line2: canvas::Shape = canvas::Shape::Path(
                    [canvas::PathEl::MoveTo(cx + r, cy - r), canvas::PathEl::LineTo(cx - r, cy + r)]
                        .into_iter().collect(),
                );
                canvas.stroke_shape(line2, rc, canvas::StrokeStyle { width: 2.0, ..Default::default() });
            }

            ctx.submit_canvas(canvas);
        }

        // Return viewport output
        input::ViewportOutput {
            texture: scene_rt.texture,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            cursor_grab: state.grabbed,
            cursor_visible: !state.grabbed,
            cursor_icon: egui::CursorIcon::Default,
            overlay_has_keyboard_focus: false,
            overlay_using_pointer: false,
        }
    });
}
