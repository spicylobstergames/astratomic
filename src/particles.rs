use crate::prelude::*;

#[derive(Component)]
pub struct Particle {
    pub atom: Atom,
    pub velocity: Vec2,
    //Used when initiating particle
    pub pos: IVec2,
}

#[derive(Component)]
pub struct Hydrated;

pub fn hydrate_particles(mut commands: Commands, particles: Query<&Particle, Without<Hydrated>>) {
    for particle in particles.iter() {
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(
                    particle.atom.color[0] as f32 / 255.,
                    particle.atom.color[1] as f32 / 255.,
                    particle.atom.color[2] as f32 / 255.,
                ),
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-50., 0., 0.)),
            ..default()
        });
    }
}

pub fn update_particles() {}
