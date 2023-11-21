use bevy::{ecs::query::QuerySingleError, log::LogPlugin, prelude::*, window::WindowMode};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

pub const LAUNCHER_TITLE: &str = "integral";

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct Config {
    show_function: bool,
    #[inspector(min = 0, max = 40)]
    n: u8,
    show_full_grid: bool,
    show_incremental_cubes: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            n: 1,
            show_full_grid: true,
            show_incremental_cubes: true,
            // TODO
            // show_function: true,
            show_function: false,
        }
    }
}

pub fn app(fullscreen: bool) -> App {
    let mode = if false && fullscreen {
        WindowMode::BorderlessFullscreen
    } else {
        WindowMode::Windowed
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    mode,
                    title: LAUNCHER_TITLE.to_string(),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: true,
                    present_mode: bevy::window::PresentMode::AutoVsync,
                    decorations: false,
                    ..default()
                }),
                ..default()
            })
            .disable::<LogPlugin>(),
    )
    .add_plugins(ResourceInspectorPlugin::<Config>::default())
    .add_plugins(PanOrbitCameraPlugin)
    .add_plugins(bevy_touch_camera::TouchCameraPlugin::default());

    app.init_resource::<Config>()
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .register_type::<Config>()
        .add_event::<AddCubes>()
        .add_event::<DeleteCubes>();

    app.add_systems(Startup, setup)
        .add_systems(Update, grid)
        .add_systems(Update, plane)
        .add_systems(Update, change_cubes)
        .add_systems(Update, delete_cubes.after(change_cubes))
        .add_systems(Update, add_cubes.after(delete_cubes));

    #[cfg(feature = "inspect")]
    app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());

    app
}

#[derive(Component)]
struct Plane;

#[derive(Component)]
struct Cube {
    size_n: u8,
}

#[derive(Event)]
struct DeleteCubes {
    prev_n: u8,
    new_n: u8,
}

#[derive(Event)]
struct AddCubes {
    prev_n: u8,
    new_n: u8,
    show_incremental_cubes: bool,
}

type Float = f32;

fn f(x: Float, y: Float) -> Float {
    x + y
}

fn add_cubes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut er: EventReader<AddCubes>,
) {
    let mut run = |n: u8, prev_n: u8| {
        let pow2n = 2i32.pow(n.into());
        let size = Float::powi(2.0, -(n as i32));
        let prev_size = if prev_n == 0 {
            0.0
        } else {
            Float::powi(2.0, -(prev_n as i32))
        };

        for i in 0..pow2n {
            for j in 0..pow2n {
                let x = size * i as Float;
                let y = size * j as Float;
                let previous_height = f(
                    prev_size * (i / 2i32.pow((n - prev_n).into())) as Float,
                    prev_size * (j / 2i32.pow((n - prev_n).into())) as Float,
                );
                let this_height = f(x, y);
                if this_height < 1e-8 || previous_height == this_height {
                    continue;
                }
                assert!(
                    previous_height <= this_height,
                    "n={n}, i={i}, j={j}, prev={previous_height}, this={this_height}"
                );

                commands.spawn((
                    Cube { size_n: n },
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Box {
                            min_x: x,
                            max_x: x + size,
                            min_y: previous_height,
                            max_y: this_height,
                            min_z: y,
                            max_z: y + size,
                        })),
                        material: materials
                            .add(Color::rgb_u8(124, (40 * n).min(255), 255 / n).into()),
                        // transform: Transform::from_xyz(x, z, y),
                        ..default()
                    },
                ));
            }
        }
    };
    for ev in er.read() {
        if ev.show_incremental_cubes {
            dbg!("HE");
            for n in (ev.prev_n + 1)..=ev.new_n {
                run(n, n - 1);
            }
        } else {
            dbg!("HA");
            run(ev.new_n, 0);
        }
    }
}

fn delete_cubes(
    query: Query<(Entity, &Cube)>,
    mut commands: Commands,
    mut er: EventReader<DeleteCubes>,
) {
    for ev in er.read() {
        for (id, cube) in &query {
            if ev.new_n < cube.size_n {
                commands.entity(id).despawn_recursive();
            }
        }
    }
}

fn plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<Config>,
    mut last_config: Local<Option<Config>>,
    mut plane: Query<&mut Visibility, With<Plane>>,
) {
    let mut do_spawn = false;
    if config.is_changed() {
        if let Some(last_config) = &*last_config {
            match (last_config.show_function, config.show_function) {
                (true, true) => {}
                (false, false) => {}
                (true, false) => match plane.get_single_mut() {
                    Ok(mut vis_map) => {
                        *vis_map = Visibility::Hidden;
                    }
                    Err(e) => unreachable!("{:?}", e),
                },
                (false, true) => {
                    do_spawn = true;
                }
            }
        } else {
            do_spawn = Config::default().show_function;
        }
        *last_config = Some((*config).clone());
    }

    if do_spawn {
        match plane.get_single_mut() {
            Ok(mut vis_map) => {
                *vis_map = Visibility::Visible;
            }
            Err(QuerySingleError::NoEntities(_)) => {}
            Err(QuerySingleError::MultipleEntities(e)) => unreachable!("{:?}", e),
        }
        use bevy::render::mesh::Indices;
        use bevy::render::render_resource::PrimitiveTopology;
        let mesh = Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 1.0],
                    [1.0, 2.0, 1.0],
                ],
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
            .with_indices(Some(Indices::U32(vec![
                0, 2, 1, // front lower
                1, 2, 3, // front upper
                0, 1, 2, // back lower
                1, 3, 2, // back upper
            ])));

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(Color::rgb_u8(200, 50, 200).into()),
                ..default()
            },
            Plane,
        ));
    }
}

fn change_cubes(
    config: Res<Config>,
    mut last_config: Local<Option<Config>>,
    mut add: EventWriter<AddCubes>,
    mut delete: EventWriter<DeleteCubes>,
) {
    if config.is_changed() {
        if let Some(last_config) = &*last_config {
            // TODO remove this
            delete.send(DeleteCubes {
                prev_n: last_config.n,
                new_n: 0,
            });

            add.send(AddCubes {
                prev_n: 0,
                new_n: config.n,
                show_incremental_cubes: config.show_incremental_cubes,
            });
            /*
            if config.n > last_config.n {
                if !config.show_incremental_cubes {
                    delete.send(DeleteCubes {
                        prev_n: last_config.n,
                        new_n: config.n - 1,
                    });
                }
                add.send(AddCubes {
                    prev_n: last_config.n,
                    new_n: config.n,
                    show_incremental_cubes: config.show_incremental_cubes,
                });
            } else if config.n < last_config.n {
                delete.send(DeleteCubes {
                    prev_n: last_config.n,
                    new_n: config.n,
                });
            } else if config.show_incremental_cubes
                && !last_config.show_incremental_cubes
                && config.n > 0
            {
                add.send(AddCubes {
                    prev_n: 0,
                    new_n: config.n,
                    show_incremental_cubes: config.show_incremental_cubes,
                });
            } else if !config.show_incremental_cubes
                && last_config.show_incremental_cubes
                && last_config.n > 0
            {
                delete.send(DeleteCubes {
                    prev_n: last_config.n,
                    new_n: config.n - 1,
                });
                add.send(AddCubes {
                    prev_n: 0,
                    new_n: config.n,
                    show_incremental_cubes: config.show_incremental_cubes,
                });
            }*/
        } else {
            add.send(AddCubes {
                prev_n: 0,
                new_n: config.n,
                show_incremental_cubes: config.show_incremental_cubes,
            })
        }
        *last_config = Some((*config).clone());
    }
}

fn grid(mut gizmos: Gizmos, orbit_cameras: Query<&PanOrbitCamera>, config: Res<Config>) {
    let pan_cam = orbit_cameras.get_single().unwrap();
    let target_radius = pan_cam.target_radius;

    let x_axis_color = Color::rgb(1.0, 0.2, 0.2);
    let y_axis_color = Color::rgb(0.2, 1.0, 0.2);
    let z_axis_color = Color::rgb(0.2, 0.2, 1.0);
    let minor_line_color = Color::rgba(0.01, 0.01, 0.01, 0.01);
    let major_line_color = Color::rgba(0.25, 0.25, 0.25, 0.5);

    let fadeout_distance = (target_radius / 2.).clamp(1.0, 3.0).round();
    let fadeout_distance_int = fadeout_distance as i32;

    let minor_per_major = 6. / target_radius.sqrt();
    let minor_per_major = minor_per_major.round() as i32;

    // major axis
    for (axis, color) in [
        (Vec3::X, x_axis_color),
        (Vec3::Y, y_axis_color),
        (Vec3::Z, z_axis_color),
    ] {
        // gizmos.ray(Vec3::ZERO, axis * 2. * fadeout_distance, color);
        gizmos.ray(Vec3::ZERO, axis * fadeout_distance.ceil(), color);
    }

    if config.show_full_grid {
        let directions = [
            |x, y| (Vec3 { x, y, z: 0. }, Vec3::Z),
            |x, z| (Vec3 { x, y: 0., z }, Vec3::Y),
            |y, z| (Vec3 { x: 0., y, z }, Vec3::X),
        ];
        for direction in directions {
            for a in 0..=(fadeout_distance_int * minor_per_major) {
                for b in 0..=(fadeout_distance_int * minor_per_major) {
                    let color = if a % minor_per_major == 0 || b % minor_per_major == 0 {
                        major_line_color
                    } else {
                        minor_line_color
                    };
                    let (offset, dir) = direction(
                        a as f32 / minor_per_major as f32,
                        b as f32 / minor_per_major as f32,
                    );
                    gizmos.ray(offset, fadeout_distance * dir, color);
                }
            }
        }
    } else {
        let directions = [
            (Vec3::X, [Vec3::Y, Vec3::Z]),
            (Vec3::Y, [Vec3::Z, Vec3::X]),
            (Vec3::Z, [Vec3::X, Vec3::Y]),
        ];
        for (main, dirs) in directions {
            for dir in dirs {
                for a in 0..=(fadeout_distance_int * minor_per_major) {
                    let color = if a % minor_per_major == 0 {
                        major_line_color
                    } else {
                        minor_line_color
                    };
                    gizmos.ray(
                        main * a as f32 / minor_per_major as f32,
                        fadeout_distance * dir,
                        color,
                    );
                }
            }
        }
    }
}

// const SCALE: Float = 4.;

fn setup(mut commands: Commands, mut gizmo: ResMut<GizmoConfig>) {
    // circular base
    /*commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Circle::new(4.0).into()),
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    */
    // cube
    /*commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        material: materials.add(Color::rgb_u8(200, 50, 200).into()),
        transform: Transform::from_xyz(1.0, 0.0, 1.0),
        ..default()
    });*/

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-6., 3.5, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
    ));

    // set grid line width
    gizmo.line_width = 0.5;
}
