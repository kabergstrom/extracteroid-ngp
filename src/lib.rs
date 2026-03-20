#![no_std]
extern crate alloc;

#[derive(newgameplus_api::NgpModule)]
struct Module;
use newgameplus_api::*;
// Re-export so rafx-generated shader modules can use `crate::ShaderResourceBindingKey`.
pub use newgameplus_api::ShaderResourceBindingKey;

mod shaders;
use shaders::{crt, mesh3d};

static FONT_BYTES: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

pub struct GameState {
    frame: u64,
    crt_variants: ShaderVariants,
    mesh3d_variants: ShaderVariants,
    font: canvas::FontHandle,
    plane_mesh: MeshAlloc,
    slug_target: Option<RenderTarget>,
    scene_target: Option<RenderTarget>,
    scene_depth: Option<RenderTarget>,
    crt_target: Option<RenderTarget>,
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

    let font = ctx.register_font(FONT_BYTES, 0);

    // Plane quad: 4 VertexPU, aspect ~4:1 (X: -2..+2, Y: -0.5..+0.5, Z=0)
    let verts = [
        vertex_formats::VertexPU::new([-2.0, -0.5, 0.0], [0.0, 1.0]),
        vertex_formats::VertexPU::new([2.0, -0.5, 0.0], [1.0, 1.0]),
        vertex_formats::VertexPU::new([2.0, 0.5, 0.0], [1.0, 0.0]),
        vertex_formats::VertexPU::new([-2.0, 0.5, 0.0], [0.0, 0.0]),
    ];
    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
    let channels = VertexChannels::POSITION.union(VertexChannels::UV0);
    let plane_mesh = upload_mesh(ctx, &verts, &indices, channels);

    GameState {
        frame: 0,
        crt_variants,
        mesh3d_variants,
        font,
        plane_mesh,
        slug_target: None,
        scene_target: None,
        scene_depth: None,
        crt_target: None,
    }
}

#[unsafe(no_mangle)]
pub fn module_init(ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx) {
    let _state = get_or_insert_state(ctx, || init_state(ctx));
    info!("extracteroid module_init");
}

#[unsafe(no_mangle)]
pub fn module_render(ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx, cf: Format, df: Format) {
    let state = get_or_insert_state(ctx, || init_state(ctx));
    ctx.request_rerender();

    // ---- Pass 1: Slug canvas text → slug_target ----
    let slug_rt = ensure_render_target(
        ctx,
        &mut state.slug_target,
        512,
        256,
        Format::R8G8B8A8_UNORM,
        [0.0, 0.0, 0.0, 1.0],
    );
    {
        let mut canvas = ctx.acquire_canvas(slug_rt);
        canvas.set_clear_color(canvas::Color::srgba(0.05, 0.0, 0.1, 1.0));
        let text = "EXTRACTEROID";
        let metrics = ctx.measure_text(state.font, text, 48.0);
        let x = 256.0 - metrics.width / 2.0;
        let y = 128.0 - metrics.height / 2.0 + metrics.ascent;
        canvas.draw_text(
            state.font,
            text,
            x,
            y,
            48.0,
            canvas::Color::srgb(0.0, 1.0, 0.4),
        );
        ctx.submit_canvas(canvas);
    }

    // ---- Pass 2: CRT post-process on slug texture → crt_target ----
    let crt_rt = ensure_render_target(
        ctx,
        &mut state.crt_target,
        512,
        256,
        Format::R8G8B8A8_UNORM,
        [0.0, 0.0, 0.0, 1.0],
    );
    {
        let crt_uni = staging_write_uniform(
            ctx,
            crt::CrtArgsBlockStd140 {
                args: crt::CrtArgsStd140 {
                    resolution: [512.0, 256.0],
                    time: state.frame as f32 * 0.016,
                    bend: 0.04,
                },
            },
        );

        let crt_pipeline = state.crt_variants.pipeline(
            ctx,
            VertexChannels::NONE,
            Format::R8G8B8A8_UNORM,
            None,
        );

        let mut b = descriptor_set(ctx, crt_pipeline, crt::CRT_UBO.set);
        b.bind_staging_alloc(crt::CRT_UBO.binding, &crt_uni);
        let set1 = b.build();

        let mut b = descriptor_set(ctx, crt_pipeline, crt::TEX.set);
        b.bind_sampler(crt::SMP.binding, ctx.shared_samplers().linear_clamp);
        b.bind_texture(crt::TEX.binding, slug_rt.texture);
        let set2 = b.build();

        let mut writer = DrawStreamWriter::new();
        writer.set_pipeline(crt_pipeline);
        writer.set_descriptor_set(1, set1);
        writer.set_descriptor_set(2, set2);
        // Reuse plane mesh index buffer — first 3 indices are [0,1,2],
        // which is exactly what gl_VertexIndex needs for the fullscreen triangle.
        writer.set_mesh_index_buffer(&state.plane_mesh);
        writer.set_triangle_count(1);
        writer.set_instance_count(1);
        writer.emit(true);

        ctx.submit_draw_stream_to_target(
            writer.finish(),
            &RenderTargetDesc {
                color: crt_rt,
                depth: None,
                color_load_op: LoadOp::Clear,
                clear_color: [0.0, 0.0, 0.0, 1.0],
            },
        );
    }

    // ---- Pass 3: 3D plane textured with CRT result → scene_target ----
    let scene_rt = ensure_render_target(
        ctx,
        &mut state.scene_target,
        800,
        600,
        cf,
        [0.02, 0.01, 0.05, 1.0],
    );
    let scene_depth = ensure_depth_target(ctx, &mut state.scene_depth, 800, 600, df);
    {
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(0.0, 0.5, 4.0),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        );
        let proj =
            glam::Mat4::perspective_rh(45.0_f32.to_radians(), 800.0 / 600.0, 0.1, 100.0);
        let model = glam::Mat4::from_rotation_y(state.frame as f32 * 0.01);
        let mvp = proj * view * model;

        let uni = staging_write_uniform(
            ctx,
            mesh3d::Mesh3dArgsBlockStd140 {
                args: mesh3d::Mesh3dArgsStd140 {
                    view_proj: mvp.to_cols_array_2d(),
                },
            },
        );

        let mut instances = staging_alloc_slice::<[[f32; 4]; 4]>(ctx, 1);
        instances.write(glam::Mat4::IDENTITY.to_cols_array_2d());

        let mesh3d_pipeline = state.mesh3d_variants.pipeline(
            ctx,
            state.plane_mesh.channels,
            cf,
            Some(df),
        );

        let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::INSTANCE_BUFFER.set);
        b.bind_slice_writer(mesh3d::INSTANCE_BUFFER.binding, &instances);
        let set0 = b.build();

        let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::MESH3D_UBO.set);
        b.bind_staging_alloc(mesh3d::MESH3D_UBO.binding, &uni);
        b.bind_mesh_vertices(mesh3d::VERTEX_BUFFER.binding, &state.plane_mesh);
        let set1 = b.build();

        let mut b = descriptor_set(ctx, mesh3d_pipeline, mesh3d::TEX.set);
        b.bind_sampler(mesh3d::SMP.binding, ctx.shared_samplers().linear_clamp);
        b.bind_texture(mesh3d::TEX.binding, crt_rt.texture);
        let set2 = b.build();

        let mut writer = DrawStreamWriter::new();
        writer.set_pipeline(mesh3d_pipeline);
        writer.set_descriptor_set(0, set0);
        writer.set_descriptor_set(1, set1);
        writer.set_descriptor_set(2, set2);
        writer.set_mesh_index_buffer(&state.plane_mesh);
        writer.set_triangle_count(state.plane_mesh.index_count / 3);
        writer.set_instance_count(1);
        writer.emit(true);

        ctx.submit_draw_stream_to_target(
            writer.finish(),
            &RenderTargetDesc {
                color: scene_rt,
                depth: Some(scene_depth),
                color_load_op: LoadOp::Clear,
                clear_color: [0.02, 0.01, 0.05, 1.0],
            },
        );
    }
}

#[unsafe(no_mangle)]
pub fn module_ui(ui: &mut egui::Ui, ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx) {
    let state = get_or_insert_state(ctx, || init_state(ctx));
    state.frame += 1;
    ui.heading("Extracteroid");
    let rect = ui.available_rect_before_wrap();
    if let Some(scene_rt) = state.scene_target {
        paint_texture::paint(ctx, ui, rect, scene_rt.texture);
    }
}
