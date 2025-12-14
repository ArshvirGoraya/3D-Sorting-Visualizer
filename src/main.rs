// use bevy::prelude::*;

use bevy::log;
use bevy::prelude::{
    App, Commands, Component, IntoScheduleConfigs, Plugin, Query, Res, ResMut, Resource, Startup,
    Update,
};

#[derive(Resource)]
struct MyGreet(String);
#[derive(Component)]
struct MySayer(String);

fn main() {
    log::info!("hellope");
    // plugins can be used to add entire sections of your app at once.
    App::new().add_plugins(MyWholeApp).run();
}

pub struct MyWholeApp;

impl Plugin for MyWholeApp {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy::log::LogPlugin {
                level: log::Level::DEBUG,
                filter: "".to_string(),
                // filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
                custom_layer: |_| None,
                fmt_layer: |_| None,
            },
            bevy::app::PanicHandlerPlugin,
        ));
        app.insert_resource(MyGreet("Hello, World!".to_string())); // Resources must be inserted to add to global state.
        app.add_systems(Startup, spawn_sayers); // components must be spawned to add to game world.
        app.add_systems(
            Update,
            (sayer, change_my_sayer, change_my_greet, sayer, test_panic).chain(),
        );
        // Components and resources can be accessed/mutated by systems.
        // .chain() can be used to enforce run order during. Otherwise all Update systems run in
        // parallel.
    }
}

fn spawn_sayers(mut commands: Commands) {
    commands.spawn(MySayer("Rust".to_string()));
}

fn change_my_sayer(mut query: Query<&mut MySayer>) {
    if let Some(mut sayer) = (&mut query).into_iter().next() {
        sayer.0 = "Odin".to_string();
    }
}

fn change_my_greet(mut my_greet: ResMut<MyGreet>) {
    my_greet.0 = "Hellope!".to_string();
}

fn sayer(my_greet: Res<MyGreet>, query: Query<&MySayer>) {
    log::info!("{} says {}", query.single().unwrap().0, my_greet.0)
    // println!("{} says {}", query.single().unwrap().0, my_greet.0)
}

fn test_panic() {
    panic!("THIS IS JUST A TEST DO NOT BE ALARMED!")
}
