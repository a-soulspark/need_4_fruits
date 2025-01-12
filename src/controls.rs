use crate::common_components::Aim;
use crate::{KeyboardControls, MainCamera, TexturesHandles, AIM_SCALE, DASH_DURATION};
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;

//region This resource defines the player's movements, defined by the keyboard/controller/mouse
#[derive(Debug)]
pub struct Movement {
    pub x: f32,
    pub jump: bool,
    pub jumped: usize, // Times jumped
    pub is_fast_falling: bool,
    pub lock_x: bool,
}

pub struct MouseCoordinates {
    pub x: f32,
    pub y: f32,
}

pub struct Dash {
    // Whether player clicked the dash button.
    // This may not result in a dash, for instance
    // If the player is in a wall, if the player
    // Has no dashes left or if the player is already
    // Dashing
    pub trying_to_dash: bool,

    // Whether the player is dashing
    // (whether it passed the trying_to_dash checks)
    pub is_dashing: bool,

    // Times dashed since last wall.
    // If this is bigger or equal to MAX_PLAYER_DASHES_MIDAIR
    // Defines whether the player is dashing
    pub dashed: usize,
    // The direction in which the player is dashing
    pub direction: DashDirection,

    // Timer for the player dash
    pub duration: Timer,
}

// The direction of a dash
#[derive(Clone)]
pub struct DashDirection {
    pub x: f32,
    pub y: f32,
}

impl DashDirection {
    fn add(&self, to_add: &DashDirection) -> DashDirection {
        DashDirection {
            x: self.x + to_add.x,
            y: self.y + to_add.y,
        }
    }
    fn is_empty(&self) -> bool {
        self.x == 0. && self.y == 0.
    }
}

impl Default for DashDirection {
    fn default() -> Self {
        DashDirection { x: 0.0, y: 0.0 }
    }
}

impl Default for Dash {
    fn default() -> Self {
        Dash {
            trying_to_dash: false,
            is_dashing: false,
            dashed: 0,
            direction: DashDirection { x: 0.0, y: 0.0 },
            duration: Timer::default(),
            
        }
    }
}

impl Dash {
    pub fn apply_time(&mut self, time: &Res<Time>) {
        self.duration.tick(time.delta());
    }
}

impl Default for Movement {
    fn default() -> Self {
        Movement {
            x: 0.0,
            jump: false,
            jumped: 0,
            is_fast_falling: false,
            lock_x: false,
        }
    }
}

impl Default for MouseCoordinates {
    fn default() -> Self {
        MouseCoordinates { x: 0.0, y: 0.0 }
    }
}
//endregion

//region Plugin boilerplate
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Movement::default())
            .insert_resource(MouseCoordinates::default())
            .insert_resource(Dash::default())
            .add_system(cursor_system)
            .add_system(keyboard_controls_system)
            .add_system(dash_direction_arrows);
    }
}
//endregion

fn keyboard_controls_system(keyboard: Res<Input<KeyCode>>, mut movement: ResMut<Movement>) {
    // You can add whatever controls you want to this list
    let controls = KeyboardControls {
        up: vec![KeyCode::W],   // In this case, jump
        down: vec![KeyCode::S], // In this case, fast fall
        right: vec![KeyCode::D],
        left: vec![KeyCode::A],
    };

    if KeyboardControls::is_just_pressed(&keyboard, &controls.up) {
        movement.jump = true;
    } // Jump will be turned to false once the value is read

    if KeyboardControls::is_pressed(&keyboard, &controls.down) {
        movement.is_fast_falling = true;
    } // You cancel fast falling by jumping or dashing

    let mut sides = 0.;
    if KeyboardControls::is_pressed(&keyboard, &controls.right) {
        sides += 1.;
    }
    if KeyboardControls::is_pressed(&keyboard, &controls.left) {
        sides -= 1.;
    }
    movement.x = sides;

    // dbg!(&movement);
}

fn joystick_control_system() {
    todo!()
}

fn cursor_system(
    mut commands: Commands,
    q_aim_entity: Query<Entity, With<Aim>>,
    mut q_aim_transform: Query<&mut Transform, With<Aim>>,
    textures: Res<TexturesHandles>,

    // need to get window dimensions
    wnds: Res<Windows>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,

    mut mouse_res: ResMut<MouseCoordinates>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // get the window that the camera is displaying to (or the primary window)
    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        // If there is no aim, spawn it
        if q_aim_entity.get_single().is_err() {
            commands
                .spawn_bundle(SpriteBundle {
                    texture: textures.aim.clone(),
                    transform: Transform {
                        scale: AIM_SCALE,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Aim);
        }

        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        let world_pos: Vec2 = world_pos.truncate();

        mouse_res.x = world_pos.x;
        mouse_res.y = world_pos.y;
    } else {
        // If there is aim, despawn it
        if let Ok(aim_ent) = q_aim_entity.get_single() {
            commands.entity(aim_ent).despawn();
        }
    }

    for mut aim_tf in q_aim_transform.iter_mut() {
        let mut translation = &mut aim_tf.translation;
        translation.x = mouse_res.x;
        translation.y = mouse_res.y;
    }
}

fn dash_direction_arrows(
    kb: Res<Input<KeyCode>>,
    mut dash: ResMut<Dash>,
) {
    // You can't change the direction while you are dashing
    // if dash.is_dashing {
    //     return;
    // }

    // You can add whatever controls you want to this list
    let controls = KeyboardControls {
        up: vec![KeyCode::Up],
        down: vec![KeyCode::Down],
        right: vec![KeyCode::Right],
        left: vec![KeyCode::Left],
    };

    // Convert whether the input has just been clicked to a number
    let to_num = |x| {
        if KeyboardControls::is_just_pressed(&kb, x) {
            1.
        } else {
            0.
        }
    };

    // Get inputs
    let up = DashDirection {
        y: to_num(&controls.up),
        x: 0.,
    };
    let down = DashDirection {
        y: -to_num(&controls.down),
        x: 0.,
    };
    let left = DashDirection {
        y: 0.,
        x: -to_num(&controls.left),
    };
    let right = DashDirection {
        y: 0.,
        x: to_num(&controls.right),
    };

    // Get diagonals
    let mut direction: DashDirection = [up, down, left, right]
        .iter()
        .fold(dash.direction.clone(), |direction, udlr| {
            direction.add(udlr) // Add all the directions for instance: x: 1 + x: -1 = x: 0
        });

    //if direction.x != 0. && direction.y != 0. {
    //    direction.x *= 0.5;
    //    direction.y *= 0.5;
    //}
    
    if !direction.is_empty() {
        dash.trying_to_dash = true;
        dash.direction = direction;
    }


}