use bevy::{ecs::query::QuerySingleError, log::LogPlugin, prelude::*, window::WindowMode};
use bevy_inspector_egui::prelude::*;
#[cfg(feature = "inspect")]
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

pub const LAUNCHER_TITLE: &str = "integral";

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct Config {
    #[inspector(min = 0, max = 40)]
    n: u8,
    show_incremental_cubes: bool,
    show_function: bool,
    show_full_grid: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            n: 1,
            show_full_grid: true,
            show_incremental_cubes: true,
            show_function: true,
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
        .add_systems(Update, button_system)
        .add_systems(Update, change_cubes)
        .add_systems(Update, delete_cubes.after(change_cubes))
        .add_systems(Update, add_cubes.after(delete_cubes));

    #[cfg(feature = "inspect")]
    app.add_plugins(ResourceInspectorPlugin::<Config>::default());
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
                            min_x: 0.,
                            max_x: size,
                            min_y: 0.,
                            max_y: this_height - previous_height,
                            min_z: 0.,
                            max_z: size,
                        })),
                        transform: Transform::from_xyz(x, previous_height, y),
                        material: materials
                            .add(Color::rgb_u8(124, (40 * n).min(255), 255 / n).into()),
                        ..default()
                    },
                ));
            }
        }
    };
    for ev in er.read() {
        if ev.show_incremental_cubes {
            for n in (ev.prev_n + 1)..=ev.new_n {
                run(n, n - 1);
            }
        } else {
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
            transform: Transform::from_xyz(-4.5, 2., 1.0)
                .looking_at(Vec3::new(2., 2., 2.), Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            focus: Vec3::new(0.8, 1.5, 0.7),
            target_focus: Vec3::new(0.8, 1.5, 0.7),
            alpha: Some(-1.5),
            target_alpha: -1.5,
            beta: Some(0.2),
            target_beta: 0.2,
            radius: Some(5.5),
            target_radius: 5.5,
            scale: Some(1.0),
            initialized: false,
            ..default()
        },
    ));
    commands.spawn(
        TextBundle::from_section("Nutze deine Maus, um die Kamera zu bewegen. linke Maus - drehen | rechte Maus - bewegen | zoom Maus - nicht zoomen\nFalls du ein Mensch bist und noch keine Maus gefangen hast, benutze deine Finger (eine Maus ist aber immer von Vorteil!)",
            TextStyle { font_size: 16., ..default() }).with_style(
            Style {
                position_type: PositionType::Absolute,
                top: Val::Px(25.0),
                left: Val::Px(25.0),
                ..default()
            },
        ),
    );
    commands
        .spawn(NodeBundle {
            style: Style {
                left: Val::Px(25.0),
                width: Val::Percent(20.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                //align_self: AlignSelf::Center,
                // align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            let button = || ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(5.0)),
                    margin: UiRect::vertical(Val::Px(5.0)),
                    ..default()
                },
                border_color: BorderColor(Color::BLACK),
                background_color: NORMAL_BUTTON.into(),
                ..default()
            };
            let text_child = |s: String| {
                move |parent: &mut ChildBuilder| {
                    parent.spawn(TextBundle::from_section(
                        s,
                        TextStyle {
                            font_size: 20.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                }
            };
            /*parent.spawn((
                TextBundle::from_section(
                    format!("n: {}", Config::default().n),
                    TextStyle {
                        font_size: 22.,
                        ..default()
                    },
                )
                .with_style(Style {
                    width: Val::Px(200.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(5.0)),
                    margin: UiRect::vertical(Val::Px(5.0)),
                    ..default()
                }),
                NText,
            ));*/
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(5.0)),
                        margin: UiRect::vertical(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((TextBundle::from_section(
                        format!("n: {}", Config::default().n),
                        TextStyle {
                            font_size: 22.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ), NText));
                });

            parent
                .spawn((button(), ConfigMore))
                .with_children(text_child(String::from("Meeehr")));
            parent
                .spawn((button(), ConfigLess))
                .with_children(text_child(String::from("(weniger)")));
            parent
                .spawn((button(), ConfigFunctionGraph))
                .with_children(text_child(format!("{} :)", SHOW_FUN)));
            parent
                .spawn((button(), ConfigIncremental))
                .with_children(text_child(format!("{} :)", SHOW_INC)));
            parent
                .spawn((button(), ConfigCoord))
                .with_children(text_child(format!("{} :)", SHOW_COORD)));
        });

    // set grid line width
    gizmo.line_width = 0.5;
}

const SHOW_FUN: &'static str = "Zeige Funktionsgraph";
const SHOW_INC: &'static str = "Zeige Zwischendinge";
const SHOW_COORD: &'static str = "Zeige alle Koordinaten";

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

#[derive(Component)]
struct ConfigMore;
#[derive(Component)]
struct ConfigLess;
#[derive(Component)]
struct ConfigFunctionGraph;
#[derive(Component)]
struct ConfigIncremental;
#[derive(Component)]
struct ConfigCoord;
#[derive(Component)]
struct NText;

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
            (
                Option<&ConfigMore>,
                Option<&ConfigLess>,
                Option<&ConfigFunctionGraph>,
                Option<&ConfigIncremental>,
                Option<&ConfigCoord>,
            ),
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text, Without<NText>>,
    mut n_text_query: Query<&mut Text, With<NText>>,
    mut config: ResMut<Config>,
) {
    for (interaction, mut color, mut border_color, children, (more, less, fun, inc, coord)) in
        &mut interaction_query
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        let mut n_text = n_text_query.get_single_mut().unwrap();
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
                if more.is_some() {
                    config.n += 1;
                    n_text.sections[0].value = format!("n: {}", config.n);
                } else if less.is_some() {
                    config.n = config.n.saturating_sub(1);
                    n_text.sections[0].value = format!("n: {}", config.n);
                } else if fun.is_some() {
                    config.show_function = !config.show_function;
                    text.sections[0].value = format!(
                        "{}{}",
                        SHOW_FUN,
                        if config.show_function { " :) " } else { "" }
                    );
                } else if inc.is_some() {
                    config.show_incremental_cubes = !config.show_incremental_cubes;
                    text.sections[0].value = format!(
                        "{}{}",
                        SHOW_INC,
                        if config.show_incremental_cubes {
                            " :) "
                        } else {
                            ""
                        }
                    );
                } else if coord.is_some() {
                    config.show_full_grid = !config.show_full_grid;
                    text.sections[0].value = format!(
                        "{}{}",
                        SHOW_COORD,
                        if config.show_full_grid { " :) " } else { "" }
                    );
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
