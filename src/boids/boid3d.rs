use cgmath::{num_traits::Float, BaseNum, InnerSpace, MetricSpace, Vector3};
use rand::{distributions::Standard, prelude::Distribution, Rng};
use std::ops::{AddAssign, Div, DivAssign, Mul, MulAssign, Sub};

use super::{limits::limit_magnitude_v3, Boid, BoidWeights};
use crate::flock::Flock;

/// A Boid in 3 dimensions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Boid3D<U: BaseNum + Float> {
    /// Boid position
    pub position: Vector3<U>,
    /// Boid velocity
    pub velocity: Vector3<U>,
    /// Boid acceleration
    pub acceleration: Vector3<U>,
    /// Boid maximum speed
    pub max_speed: U,
    /// Boid maximum force
    pub max_force: U,
    /// Boid maximum turn rate
    pub r: U,
    /// Boid weights
    pub weights: BoidWeights<U>,
}

impl<U: BaseNum + Float> Boid3D<U> {
    /// Create a new Boid3D from a position and angle
    pub fn new_with_angle(position: Vector3<U>, angle: U) -> Self {
        Self {
            position,
            velocity: Vector3::new(angle.cos(), angle.sin(), U::zero()),
            acceleration: Vector3::new(U::zero(), U::zero(), U::zero()),
            r: U::one() + U::one(),
            max_speed: U::one() + U::one(),
            max_force: U::from(0.03).unwrap(),
            weights: BoidWeights::default(),
        }
    }

    /// Create a new Boid3D from a position and random angle
    pub fn new(position: Vector3<U>) -> Self
    where
        Standard: Distribution<U>,
    {
        let angle = rand::thread_rng().gen::<U>() * U::from(std::f64::consts::PI * 2.0).unwrap();
        Self::new_with_angle(position, angle)
    }
}

impl<U: BaseNum + Float> Boid<Boid3D<U>, U> for Boid3D<U> {
    fn separate(&self, flock: &Flock<Boid3D<U>, U>) -> Vector3<U> {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        // Alloc a steering force
        let mut steer = Vector3::new(U::zero(), U::zero(), U::zero());

        // Tracker for number of boids nearby
        let mut count = U::zero();

        // Steer away from nearby boids
        for boid in flock.boids.iter() {
            let distance = self.position.distance(boid.position());

            // Only operate on nearby boids
            if distance > U::zero() && distance < flock.goal_separation {
                // Calculate vector pointing away from neighbor
                let diff = (self.position - boid.position()).normalize().div(distance);
                steer.add_assign(diff);
                count += U::one();
            }
        }

        // Average the steering factor
        if count > U::zero() {
            steer.div_assign(count);
        }

        // Implement Reynolds: Limit the steering force to max_force
        if steer.magnitude() > U::zero() {
            steer = limit_magnitude_v3(
                steer.normalize().mul(self.max_speed).sub(self.velocity),
                self.max_force,
            );
        }

        steer
    }

    fn align(&self, flock: &Flock<Boid3D<U>, U>) -> Vector3<U> {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        // Alloc an alignment force
        let mut align = Vector3::new(U::zero(), U::zero(), U::zero());

        // Tracker for number of boids nearby
        let mut count = U::zero();

        // Align with nearby boids
        for boid in flock.boids.iter() {
            let distance = self.position.distance(boid.position());

            // Only operate on nearby boids
            if distance > U::zero() && distance < flock.goal_alignment {
                align.add_assign(boid.velocity());
                count += U::one();
            }
        }

        // Average the alignment factor
        if count > U::zero() {
            align.div_assign(count);

            // Implement Reynolds: Limit the steering force to max_force
            limit_magnitude_v3(
                align.normalize().mul(self.max_speed).sub(self.velocity),
                self.max_force,
            )
        } else {
            Vector3::new(U::zero(), U::zero(), U::zero())
        }
    }

    fn cohesion(&self, flock: &Flock<Boid3D<U>, U>) -> Vector3<U> {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        // Alloc a steering force
        let mut cohesion = Vector3::new(U::zero(), U::zero(), U::zero());

        // Tracker for number of boids nearby
        let mut count = U::zero();

        // Steer towards nearby boids
        for boid in flock.boids.iter() {
            let distance = self.position.distance(boid.position());

            // Only operate on nearby boids
            if distance > U::zero() && distance < flock.goal_cohesion {
                cohesion.add_assign(boid.position());
                count += U::one();
            }
        }

        // Average the cohesion factor
        if count > U::zero() {
            cohesion.div_assign(count);
            cohesion = cohesion.sub(self.position);

            // Implement Reynolds: Limit the steering force to max_force
            limit_magnitude_v3(
                cohesion.normalize().mul(self.max_speed).sub(self.velocity),
                self.max_force,
            )
        } else {
            Vector3::new(U::zero(), U::zero(), U::zero())
        }
    }

    fn set_weights(&mut self, weights: BoidWeights<U>) {
        self.weights = weights;
    }

    fn get_weights<'a>(&'a self) -> &'a BoidWeights<U> {
        &self.weights
    }

    fn with_force(&self, force: Vector3<U>) -> Boid3D<U> {
        // Alloc a new boid
        let mut boid = self.clone();

        // Apply acceleration to velocity
        boid.velocity.add_assign(force);

        // Limit the speed
        boid.velocity = limit_magnitude_v3(boid.velocity, self.max_speed);

        // Apply velocity to position
        boid.position.add_assign(boid.velocity);

        // Reset acceleration
        boid.acceleration.mul_assign(U::zero());
        boid
    }

    fn position(&self) -> Vector3<U> {
        self.position
    }

    fn velocity(&self) -> Vector3<U> {
        self.velocity
    }

    fn acceleration(&self) -> Vector3<U> {
        self.acceleration
    }

    fn update(&self, flock: &Flock<Boid3D<U>, U>) -> Boid3D<U> {
        #[cfg(feature = "puffin")]
        puffin::profile_function!();

        let weights = self.get_weights();
        let separation = self.separate(flock).mul(weights.separation);
        let alignment = self.align(flock).mul(weights.alignment);
        let cohesion = self.cohesion(flock).mul(weights.cohesion);
        let targeting = flock
            .target
            .map(|target| target.sub(self.position))
            .unwrap_or(Vector3::new(U::zero(), U::zero(), U::zero()))
            .mul(weights.targeting);
        self.with_force(separation)
            .with_force(alignment)
            .with_force(cohesion)
            .with_force(targeting)
    }
}
