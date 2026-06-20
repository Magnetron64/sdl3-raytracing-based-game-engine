use std::f32::consts::PI;

use crate::data::{self, QuantType};
use cgmath::{
    num_traits::{pow, Float, Inv, Pow},
    perspective, Angle, Array, Deg, ElementWise, EuclideanSpace, InnerSpace, Matrix4, MetricSpace,
    Point3, Quaternion, Rad, Rotation, Rotation3, Vector3, Zero,
};
use core::time::Duration;

#[derive(Clone)]
pub struct Universe {
    pub planets: Vec<Planet>,
}

impl Universe {
    pub fn new() -> Self {
        Self {
            planets: vec![Planet::new(Vector3::new(0.0, 0.0, 0.0), 5.0, 1000)],
        }
    }

    pub fn update(&mut self, dt: Duration) {
        for planet in &mut self.planets {
            planet.update(dt);
        }
    }
}

#[derive(Clone)]
pub struct Planet {
    pub pos: Vector3<f32>,
    pub radius: f32,
    pub quants: Vec<Quant>,
}

impl Planet {
    pub fn new(pos: Vector3<f32>, radius: f32, count: u32) -> Self {
        let mut quants = vec![];
        for c in 0..count {
            let qpos = pos
                + Vector3::new(
                    (fastrand::f32() * radius * 2.0 + radius * 1.1)
                        * fastrand::choice([-1.0, 1.0].iter()).unwrap(),
                    (fastrand::f32() * radius * 2.0 + radius * 1.1)
                        * fastrand::choice([-1.0, 1.0].iter()).unwrap(),
                    (fastrand::f32() * radius * 2.0 + radius * 1.1)
                        * fastrand::choice([-1.0, 1.0].iter()).unwrap(),
                );

            let base_matter = vec![QuantType::Stone as u32, QuantType::Water as u32];

            quants.push(Quant::new(
                base_matter[fastrand::usize(0..2)],
                qpos,
                Vector3::new(
                    fastrand::f32() * 2.0 - 1.0,
                    fastrand::f32() * 2.0 - 1.0,
                    fastrand::f32() * 2.0 - 1.0,
                ),
                Vector3::new(
                    fastrand::f32() * 2.0 - 1.0,
                    fastrand::f32() * 2.0 - 1.0,
                    fastrand::f32() * 2.0 - 1.0,
                ),
                1.0,
                1.9,
            ));
        }

        Self {
            pos,
            radius,
            quants,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        let quants_cloned = self.quants.clone();
        let total_mass = quants_cloned.iter().fold(0.0, |acc, x| acc + x.mass);
        for i in 0..self.quants.len() {
            let (left, right) = self.quants.split_at_mut(i + 1);
            let (quant, rest) = left.split_last_mut().unwrap();
            quant.gravity_force(self.pos, 0.5, total_mass, dt);
            quant.move_quant(0.5, dt);
            // quant.cohesive_force(rest, dt);
            // quant.cohesive_force(right, dt);
            quant.collide(rest, self.pos, self.radius, total_mass);
            quant.collide(right, self.pos, self.radius, total_mass);
        }
    }
}

#[derive(Clone, Copy)]
pub struct Quant {
    pub quant_type: u32,
    pub pos: Vector3<f32>,
    pub color: Vector3<f32>,
    pub radius: f32,
    pub mass: f32,
    pub velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
    pub temperature: f32,
    pub gravity: f32,
}

impl Quant {
    pub fn new(
        quant_type: u32,
        pos: Vector3<f32>,
        velocity: Vector3<f32>,
        acceleration: Vector3<f32>,
        temperature: f32,
        gravity: f32,
    ) -> Self {
        let properties = data::QUANT_PROPERTIES.get(&quant_type).unwrap();
        Self {
            quant_type,
            pos,
            color: properties.2,
            radius: properties.1,
            mass: properties.0,
            velocity,
            acceleration,
            temperature,
            gravity,
        }
    }

    pub fn move_quant(&mut self, lerp_factor: f32, dt: Duration) {
        self.pos += self.velocity * dt.as_secs_f32() * lerp_factor;
        self.velocity += self.acceleration * dt.as_secs_f32() * lerp_factor;
    }

    pub fn gravity_force(
        &mut self,
        center_pos: Vector3<f32>,
        lerp_factor: f32,
        planet_mass: f32,
        dt: Duration,
    ) {
        let dir = center_pos - self.pos;
        let dist = dir.magnitude();
        let gacc = dir * ((self.gravity * planet_mass) / dist.powi(3));
        self.acceleration += gacc * dt.as_secs_f32() * lerp_factor;
    }

    pub fn cohesive_force(&mut self, quants: &mut [Quant], dt: Duration) {
        for quant in quants {
            let dist = self.pos.distance2(quant.pos);
            let dir1 = quant.pos - self.pos;
            let dir2 = -dir1;
            if self.quant_type == quant.quant_type {
                self.acceleration = dir1 * (100.0 / dist) * dt.as_secs_f32();
                quant.acceleration = dir2 * (100.0 / dist) * dt.as_secs_f32();
            }
        }
    }

    pub fn collide(
        &mut self,
        quants: &mut [Quant],
        center: Vector3<f32>,
        radius: f32,
        planet_mass: f32,
    ) {
        for quant in quants {
            let dist = self.pos.distance2(quant.pos);
            let dist_to_surfacei_s = self.pos.distance2(center);
            let dist_to_surfacei_o = quant.pos.distance2(center);
            if dist < pow(self.radius, 2) && self.pos != quant.pos {
                let dir = -quant.pos + self.pos;
                let depth = self.radius + quant.radius - dist.sqrt() * 0.5;

                self.pos += dir.normalize_to(depth);
                quant.pos -= dir.normalize_to(depth);
                let v1f = (self.mass - quant.mass) / (self.mass + quant.mass) * self.velocity
                    + (2.0 * quant.mass) / (self.mass + quant.mass) * quant.velocity;
                let v2f = (2.0 * self.mass) / (self.mass + quant.mass) * self.velocity
                    + (quant.mass - self.mass) / (self.mass + quant.mass) * quant.velocity;

                self.velocity = v1f * 0.8;
                quant.velocity = v2f * 0.8;
            }

            if dist_to_surfacei_o < radius.powi(2) {
                let dir = quant.pos - center;
                let depth = quant.radius + radius - dist_to_surfacei_o.sqrt() * 0.5;
                quant.pos -= dir.normalize_to(depth);

                // let vqf = (planet_mass - quant.mass) / (planet_mass + quant.mass) * Vector3::zero()
                //     + (2.0 * quant.mass) / (planet_mass + quant.mass) * quant.velocity;
                // quant.velocity = vqf * 0.00001;
                quant.velocity = Vector3::zero();
                quant.acceleration = Vector3::zero();
            }

            if dist_to_surfacei_s < radius.powi(2) {
                let dir = self.pos - center;
                let depth = self.radius + radius - dist_to_surfacei_s.sqrt() * 0.5;
                self.pos -= dir.normalize_to(depth);

                // let vsf = (planet_mass - self.mass) / (planet_mass + self.mass) * Vector3::zero()
                // + (2.0 * self.mass) / (planet_mass + self.mass) * self.velocity;
                // self.velocity = vsf * 0.00001;
                self.velocity = Vector3::zero();
                self.acceleration = Vector3::zero();
            }
        }
    }
}

pub struct Player {
    pub pos: Vector3<f32>,
    pub cam: Camera,
    pub speed: f32,
    pub up: Vector3<f32>,
}

impl Player {
    pub fn new(cam: Camera, speed: f32) -> Self {
        Self {
            pos: cam.pos,
            cam,
            speed,
            up: Vector3::unit_y(),
        }
    }

    pub fn update(&mut self, universe: &mut Universe) {
        self.pos = self.cam.pos;
        self.up = self.calculate_surface_normal(universe);
        self.cam.update();
        self.cam.cview(self.up);
    }

    pub fn calculate_surface_normal(&self, universe: &mut Universe) -> Vector3<f32> {
        if let Some(current_planet) = universe
            .planets
            .iter()
            .find(|p| p.pos.distance2(self.pos) < pow(5.0, 2))
        {
            -(current_planet.pos - self.pos).normalize()
        } else {
            self.up
        }
    }

    pub fn step_forward(&mut self) {
        self.cam.translate(self.cam.look(self.up) * self.speed);
    }
    pub fn step_backward(&mut self) {
        self.cam.translate(self.cam.look(self.up) * -self.speed);
    }
    pub fn step_right(&mut self) {
        self.cam
            .translate(self.cam.look(self.up).cross(self.up) * -self.speed);
    }
    pub fn step_left(&mut self) {
        self.cam
            .translate(self.cam.look(self.up).cross(self.up) * self.speed);
    }

    pub fn get_visible_terrain(&self, universe: &mut Universe) -> Vec<Quant> {
        if let Some(current_planet) = universe
            .planets
            .iter()
            .find(|p| p.pos.distance2(self.pos) < pow(15.0, 2))
        {
            current_planet
                .quants
                .iter()
                .filter(|q| q.pos.distance2(self.pos) < pow(15.0, 2))
                .copied()
                .collect()
        } else {
            vec![]
        }
    }
}

pub fn view(eye: Point3<f32>, direction: Vector3<f32>, up: Vector3<f32>) -> Matrix4<f32> {
    Matrix4::look_to_rh(eye, direction, up)
}

pub struct Camera {
    pub pos: Vector3<f32>,
    target_pos: Vector3<f32>,
    pub pitch: Rad<f32>,
    target_pitch: f32,
    pub yaw: Rad<f32>,
    target_yaw: f32,
    pub speed: f32,
    pub projection: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub aspect: f32,
    pub lerp_rotation: f32,
    pub lerp_translation: f32,
}

impl Camera {
    pub fn new(
        pos: Vector3<f32>,
        speed: f32,
        aspect: f32,
        lerp_rotation: f32,
        lerp_translation: f32,
    ) -> Self {
        Self {
            pos,
            target_pos: pos,
            pitch: Rad(0.0),
            target_pitch: 0.0,
            yaw: Rad(0.0),
            target_yaw: 0.0,
            speed,
            projection: Matrix4::zero(),
            view: Matrix4::zero(),
            aspect,
            lerp_rotation,
            lerp_translation,
        }
    }

    pub fn look(&self, up: Vector3<f32>) -> Vector3<f32> {
        let forward = Vector3::new(
            self.pitch.0.cos() * self.yaw.0.cos(),
            self.pitch.0.sin(),
            self.pitch.0.cos() * self.yaw.0.sin(),
        )
        .normalize();
        let rotation: Quaternion<_> = Quaternion::between_vectors(Vector3::unit_y(), up);
        rotation * forward
    }

    pub fn cview(&mut self, up: Vector3<f32>) {
        self.view = view(Point3::from_vec(self.pos), self.look(up), up);
    }

    pub fn rotate(&mut self, m_pitch: f32, m_yaw: f32) {
        self.target_pitch -= m_pitch * self.speed;
        self.target_yaw += m_yaw * self.speed;
    }

    pub fn translate(&mut self, t_pos: Vector3<f32>) {
        self.target_pos += t_pos;
    }

    pub fn update(&mut self) {
        self.yaw = Rad(self.yaw.0 + (self.target_yaw - self.yaw.0) * self.lerp_rotation);
        self.pitch = Rad(self.pitch.0 + (self.target_pitch - self.pitch.0) * self.lerp_rotation);
        let delta = Vector3::new(
            if self.target_pos.x - self.pos.x > 0.0 {
                (self.target_pos.x - self.pos.x).max(0.1)
            } else if self.target_pos.x - self.pos.x < 0.0 {
                (self.target_pos.x - self.pos.x).min(-0.1)
            } else {
                fastrand::f32() * 0.001 - 0.0005
            },
            if self.target_pos.y - self.pos.y > 0.0 {
                (self.target_pos.y - self.pos.y).max(0.1)
            } else if self.target_pos.y - self.pos.y < 0.0 {
                (self.target_pos.y - self.pos.y).min(-0.1)
            } else {
                fastrand::f32() * 0.001 - 0.0005
            },
            if self.target_pos.z - self.pos.z > 0.0 {
                (self.target_pos.z - self.pos.z).max(0.1)
            } else if self.target_pos.z - self.pos.z < 0.0 {
                (self.target_pos.z - self.pos.z).min(-0.1)
            } else {
                fastrand::f32() * 0.001 - 0.0005
            },
        );
        self.pos += delta * self.lerp_translation;

        let pcap = Rad(89.0 * PI / 180.0);
        if self.pitch < -pcap {
            self.pitch = -pcap
        } else if self.pitch > pcap {
            self.pitch = pcap
        }
    }
}
