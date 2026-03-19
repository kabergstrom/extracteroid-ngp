#![no_std]
#[macro_use]
extern crate alloc;

#[derive(newgameplus_api::NgpModule)]
struct Module;
use newgameplus_api::*;
// Re-export so rafx-generated shader modules can use `crate::ShaderResourceBindingKey`.
pub use newgameplus_api::ShaderResourceBindingKey;

mod shaders;

pub struct GameState {
    frame: u64,
    crt_variants: ShaderVariants,
    noise_variants: ShaderVariants,
    mesh3d_variants: ShaderVariants,
    scene_target: Option<RenderTarget>,
}

fn init_state(ctx: &(impl RenderCtx + ?Sized)) -> GameState {
    let crt_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/crt.cookedshaderpackage"),
    );
    let noise_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/static_noise.cookedshaderpackage"),
    );
    let mesh3d_variants = ShaderVariants::new(
        ctx,
        include_bytes!("../cooked_shaders/mesh3d.cookedshaderpackage"),
    )
    .depth_test(true);

    GameState {
        frame: 0,
        crt_variants,
        noise_variants,
        mesh3d_variants,
        scene_target: None,
    }
}

#[unsafe(no_mangle)]
pub fn module_init(ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx) {
    let _state = get_or_insert_state(ctx, || init_state(ctx));
    info!("extracteroid module_init");
}

#[unsafe(no_mangle)]
pub fn module_render(ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx, _cf: Format, _df: Format) {
    ctx.request_rerender();
}

#[unsafe(no_mangle)]
pub fn module_ui(ui: &mut egui::Ui, ctx: &dyn RenderCtx, _phys: &dyn PhysicsCtx) {
    let state = get_or_insert_state(ctx, || init_state(ctx));
    state.frame += 1;
    ui.heading("Extracteroid");
    ui.label(format!("frame {}", state.frame));
}
