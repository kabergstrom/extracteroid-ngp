//! 3D room geometry, TV positions, FPS camera, and physics button definitions.

use core::f32::consts::PI;

use glam::{Mat4, Vec3};

// ---------------------------------------------------------------------------
// TV slots — where screens are placed in the room
// ---------------------------------------------------------------------------

pub struct TvSlot {
    pub model: Mat4,
    pub screen_size: [f32; 2],
}

/// Six TV positions matching the original game layout.
/// All TVs face toward positive X (the player stands near x=0 looking at x=-10).
pub fn tv_slots() -> [TvSlot; 6] {
    [
        // 0: Gameplay — back wall center, large
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-6.0, 2.5, 0.0))
                * Mat4::from_rotation_y(PI / 2.0)
                * Mat4::from_scale(Vec3::new(3.2, 2.4, 1.0)),
            screen_size: [3.2, 2.4],
        },
        // 1: Console — below-right of gameplay
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-6.0, 0.5, 2.8))
                * Mat4::from_rotation_y(PI / 2.0 - 0.3)
                * Mat4::from_scale(Vec3::new(1.3, 1.0, 1.0)),
            screen_size: [1.3, 1.0],
        },
        // 2: Enterprises — left wall upper
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-10.0, 5.5, -5.0))
                * Mat4::from_rotation_y(PI / 3.0)
                * Mat4::from_scale(Vec3::new(1.6, 1.2, 1.0)),
            screen_size: [1.6, 1.2],
        },
        // 3: Scanner — left wall lower (actually upper right in original)
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-10.0, 5.5, 5.0))
                * Mat4::from_rotation_y(-PI / 3.0)
                * Mat4::from_scale(Vec3::new(1.6, 1.2, 1.0)),
            screen_size: [1.6, 1.2],
        },
        // 4: Cargo — right wall
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-10.0, 2.5, 5.0))
                * Mat4::from_rotation_y(-PI / 3.0)
                * Mat4::from_scale(Vec3::new(1.6, 1.2, 1.0)),
            screen_size: [1.6, 1.2],
        },
        // 5: Balance — above gameplay
        TvSlot {
            model: Mat4::from_translation(Vec3::new(-6.0, 5.5, 0.0))
                * Mat4::from_rotation_y(PI / 2.0)
                * Mat4::from_scale(Vec3::new(1.6, 1.0, 1.0)),
            screen_size: [1.6, 1.0],
        },
    ]
}

// ---------------------------------------------------------------------------
// Room wall quads
// ---------------------------------------------------------------------------

pub struct RoomQuad {
    pub model: Mat4,
    pub color: [f32; 4],
}

/// Six wall/floor/ceiling quads forming the room.
pub fn room_quads() -> [RoomQuad; 6] {
    let dark = [0.06, 0.04, 0.08, 1.0];
    let floor = [0.04, 0.03, 0.06, 1.0];
    let ceiling = [0.03, 0.02, 0.05, 1.0];

    [
        // Floor (Y=0, facing up)
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(-5.0, 0.0, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(12.0, 14.0, 1.0)),
            color: floor,
        },
        // Ceiling (Y=8)
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(-5.0, 8.0, 0.0))
                * Mat4::from_rotation_x(PI / 2.0)
                * Mat4::from_scale(Vec3::new(12.0, 14.0, 1.0)),
            color: ceiling,
        },
        // Back wall (X=-11)
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(-11.0, 4.0, 0.0))
                * Mat4::from_rotation_y(PI / 2.0)
                * Mat4::from_scale(Vec3::new(14.0, 8.0, 1.0)),
            color: dark,
        },
        // Left wall (Z=-7)
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(-5.0, 4.0, -7.0))
                * Mat4::from_scale(Vec3::new(12.0, 8.0, 1.0)),
            color: dark,
        },
        // Right wall (Z=7)
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(-5.0, 4.0, 7.0))
                * Mat4::from_rotation_y(PI)
                * Mat4::from_scale(Vec3::new(12.0, 8.0, 1.0)),
            color: dark,
        },
        // Front wall (X=1) — behind the player
        RoomQuad {
            model: Mat4::from_translation(Vec3::new(1.0, 4.0, 0.0))
                * Mat4::from_rotation_y(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(14.0, 8.0, 1.0)),
            color: dark,
        },
    ]
}

// ---------------------------------------------------------------------------
// FPS camera
// ---------------------------------------------------------------------------

pub struct FpsCamera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(-4.0, 4.0, 0.0),
            yaw: PI, // looking toward -X (the back wall)
            pitch: 0.0,
        }
    }
}

impl FpsCamera {
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);
        proj * self.view_matrix()
    }

    pub fn update(&mut self, move_input: [f32; 2], look_input: [f32; 2], dt: f32) {
        // Look
        self.yaw += look_input[0];
        self.pitch = (self.pitch - look_input[1]).clamp(-PI / 2.2, PI / 2.2);

        // Move (forward/back = move_input[1], strafe = move_input[0])
        let speed = 5.0 * dt;
        let fwd = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize();
        let right = fwd.cross(Vec3::Y).normalize();

        self.position += fwd * move_input[1] * speed;
        self.position += right * move_input[0] * speed;

        // Clamp to room bounds
        self.position.x = self.position.x.clamp(-10.0, 0.5);
        self.position.y = self.position.y.clamp(1.0, 7.0);
        self.position.z = self.position.z.clamp(-6.5, 6.5);
    }
}

// ---------------------------------------------------------------------------
// Physics buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Deploy,
    Extract,
    Scan,
    CargoDial,
    CargoInteract,
    CargoSell,
    VendAmmo,
    VendFuel,
}

pub struct ButtonDef {
    pub position: Vec3,
    pub radius: f32,
    pub color: [f32; 4],
    pub label: &'static str,
    pub action: ButtonAction,
}

pub fn button_definitions() -> [ButtonDef; 8] {
    let r = 0.4;
    [
        ButtonDef {
            position: Vec3::new(-8.0, 3.3, -1.0),
            radius: r,
            color: [0.16, 0.98, 0.41, 1.0], // green
            label: "DEPLOY",
            action: ButtonAction::Deploy,
        },
        ButtonDef {
            position: Vec3::new(-8.0, 3.3, 1.0),
            radius: r,
            color: [0.78, 0.45, 0.28, 1.0], // orange
            label: "EXTRACT",
            action: ButtonAction::Extract,
        },
        ButtonDef {
            position: Vec3::new(-10.0, 5.0, 4.5),
            radius: r,
            color: [0.07, 0.30, 0.74, 1.0], // blue
            label: "SCAN",
            action: ButtonAction::Scan,
        },
        ButtonDef {
            position: Vec3::new(-10.7, 2.0, 4.9),
            radius: r,
            color: [0.5, 0.0, 0.5, 1.0], // purple
            label: "DIAL",
            action: ButtonAction::CargoDial,
        },
        ButtonDef {
            position: Vec3::new(-10.0, 2.0, 4.5),
            radius: r,
            color: [0.85, 0.84, 0.19, 1.0], // yellow
            label: "INTERACT",
            action: ButtonAction::CargoInteract,
        },
        ButtonDef {
            position: Vec3::new(-9.3, 2.0, 4.1),
            radius: r,
            color: [0.78, 0.28, 0.28, 1.0], // red
            label: "SELL",
            action: ButtonAction::CargoSell,
        },
        ButtonDef {
            position: Vec3::new(-10.0, 3.5, -3.0),
            radius: r,
            color: [0.0, 1.0, 1.0, 1.0], // cyan
            label: "AMMO",
            action: ButtonAction::VendAmmo,
        },
        ButtonDef {
            position: Vec3::new(-10.0, 3.5, -4.0),
            radius: r,
            color: [1.0, 0.0, 1.0, 1.0], // magenta
            label: "FUEL",
            action: ButtonAction::VendFuel,
        },
    ]
}

// ---------------------------------------------------------------------------
// Screen-to-ray for raycast picking
// ---------------------------------------------------------------------------

pub fn screen_to_ray(
    mouse_pos: [f32; 2],
    viewport_size: [f32; 2],
    camera: &FpsCamera,
    aspect: f32,
) -> (Vec3, Vec3) {
    let ndc_x = (2.0 * mouse_pos[0] / viewport_size[0]) - 1.0;
    let ndc_y = 1.0 - (2.0 * mouse_pos[1] / viewport_size[1]);

    let view_proj = camera.view_proj(aspect);
    let inv = view_proj.inverse();

    let near = inv.project_point3(Vec3::new(ndc_x, ndc_y, -1.0));
    let far = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

    let direction = (far - near).normalize();
    (near, direction)
}
