//! N-body simulation of the Jovian planets: floating-point arithmetic on an array of objects.

use std::env;
use std::f64::consts::PI;

const DEFAULT_SIZE: i64 = 15_000_000;
const SOLAR_MASS: f64 = 4.0 * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;

struct Body {
	x: f64,
	y: f64,
	z: f64,
	vx: f64,
	vy: f64,
	vz: f64,
	mass: f64,
}

fn initial_bodies() -> Vec<Body> {
	vec![
		// Sun
		Body {
			x: 0.0,
			y: 0.0,
			z: 0.0,
			vx: 0.0,
			vy: 0.0,
			vz: 0.0,
			mass: SOLAR_MASS,
		},
		// Jupiter
		Body {
			x: 4.841431442464721,
			y: -1.1603200440274284,
			z: -0.10362204447112311,
			vx: 0.001660076642744037 * DAYS_PER_YEAR,
			vy: 0.007699011184197404 * DAYS_PER_YEAR,
			vz: -6.90460016972063e-5 * DAYS_PER_YEAR,
			mass: 0.0009547919384243266 * SOLAR_MASS,
		},
		// Saturn
		Body {
			x: 8.34336671824458,
			y: 4.124798564124305,
			z: -0.4035234171143214,
			vx: -0.002767425107268624 * DAYS_PER_YEAR,
			vy: 0.004998528012349172 * DAYS_PER_YEAR,
			vz: 2.3041729757376393e-5 * DAYS_PER_YEAR,
			mass: 0.0002858859806661308 * SOLAR_MASS,
		},
		// Uranus
		Body {
			x: 12.894369562139131,
			y: -15.111151401698631,
			z: -0.22330757889265573,
			vx: 0.002964601375647616 * DAYS_PER_YEAR,
			vy: 0.0023784717395948095 * DAYS_PER_YEAR,
			vz: -2.9658956854023756e-5 * DAYS_PER_YEAR,
			mass: 4.366244043351563e-5 * SOLAR_MASS,
		},
		// Neptune
		Body {
			x: 15.379697114850917,
			y: -25.919314609987964,
			z: 0.17925877295037118,
			vx: 0.0026806777249038932 * DAYS_PER_YEAR,
			vy: 0.001628241700382423 * DAYS_PER_YEAR,
			vz: -9.515922545197159e-5 * DAYS_PER_YEAR,
			mass: 5.1513890204661145e-5 * SOLAR_MASS,
		},
	]
}

fn offset_momentum(bodies: &mut [Body]) {
	let mut px = 0.0;
	let mut py = 0.0;
	let mut pz = 0.0;
	for body in bodies.iter() {
		px += body.vx * body.mass;
		py += body.vy * body.mass;
		pz += body.vz * body.mass;
	}
	let sun = &mut bodies[0];
	sun.vx = -px / SOLAR_MASS;
	sun.vy = -py / SOLAR_MASS;
	sun.vz = -pz / SOLAR_MASS;
}

fn energy(bodies: &[Body]) -> f64 {
	let mut energy = 0.0;
	for (i, body) in bodies.iter().enumerate() {
		energy += 0.5 * body.mass * (body.vx * body.vx + body.vy * body.vy + body.vz * body.vz);
		for other in &bodies[i + 1..] {
			let dx = body.x - other.x;
			let dy = body.y - other.y;
			let dz = body.z - other.z;
			energy -= body.mass * other.mass / (dx * dx + dy * dy + dz * dz).sqrt();
		}
	}
	energy
}

fn advance(bodies: &mut [Body], step: f64) {
	for i in 0..bodies.len() {
		let (left, right) = bodies.split_at_mut(i + 1);
		let body = &mut left[i];
		for other in right.iter_mut() {
			let dx = body.x - other.x;
			let dy = body.y - other.y;
			let dz = body.z - other.z;
			let squared = dx * dx + dy * dy + dz * dz;
			let magnitude = step / (squared * squared.sqrt());
			body.vx -= dx * other.mass * magnitude;
			body.vy -= dy * other.mass * magnitude;
			body.vz -= dz * other.mass * magnitude;
			other.vx += dx * body.mass * magnitude;
			other.vy += dy * body.mass * magnitude;
			other.vz += dz * body.mass * magnitude;
		}
	}
	for body in bodies.iter_mut() {
		body.x += step * body.vx;
		body.y += step * body.vy;
		body.z += step * body.vz;
	}
}

/// The energy rounded to 9 decimals, as an integer: prints the same on every target.
fn rounded(energy: f64) -> i64 {
	(energy * 1e9).round() as i64
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let steps = size_argument(DEFAULT_SIZE);
	let mut bodies = initial_bodies();
	offset_momentum(&mut bodies);
	println!("energy before: {} (x 1e-9)", rounded(energy(&bodies)));
	for _ in 0..steps {
		advance(&mut bodies, 0.01);
	}
	println!("energy after: {} (x 1e-9)", rounded(energy(&bodies)));
}
