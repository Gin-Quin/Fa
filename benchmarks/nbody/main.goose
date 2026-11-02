// N-body simulation of the Jovian planets (Goose port of main.rs): the same
// operations in the same order, on a growable array of bodies.

import std;
import os;
import math;

let SOLAR_MASS = 4.0 * PI * PI;
let DAYS_PER_YEAR = 365.24;

struct Body { x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, mass: f64 }

fn initial_bodies(bodies: Body[>..]&) {
    // Sun
    bodies.push(Body { 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS });
    // Jupiter
    bodies.push(Body { 4.841431442464721, -1.1603200440274284, -0.10362204447112311,
        0.001660076642744037 * DAYS_PER_YEAR, 0.007699011184197404 * DAYS_PER_YEAR,
        -6.90460016972063e-5 * DAYS_PER_YEAR, 0.0009547919384243266 * SOLAR_MASS });
    // Saturn
    bodies.push(Body { 8.34336671824458, 4.124798564124305, -0.4035234171143214,
        -0.002767425107268624 * DAYS_PER_YEAR, 0.004998528012349172 * DAYS_PER_YEAR,
        2.3041729757376393e-5 * DAYS_PER_YEAR, 0.0002858859806661308 * SOLAR_MASS });
    // Uranus
    bodies.push(Body { 12.894369562139131, -15.111151401698631, -0.22330757889265573,
        0.002964601375647616 * DAYS_PER_YEAR, 0.0023784717395948095 * DAYS_PER_YEAR,
        -2.9658956854023756e-5 * DAYS_PER_YEAR, 4.366244043351563e-5 * SOLAR_MASS });
    // Neptune
    bodies.push(Body { 15.379697114850917, -25.919314609987964, 0.17925877295037118,
        0.0026806777249038932 * DAYS_PER_YEAR, 0.001628241700382423 * DAYS_PER_YEAR,
        -9.515922545197159e-5 * DAYS_PER_YEAR, 5.1513890204661145e-5 * SOLAR_MASS });
}

fn offset_momentum(bodies: Body[:]) {
    var px = 0.0;
    var py = 0.0;
    var pz = 0.0;
    for body in bodies {
        px += body.vx * body.mass;
        py += body.vy * body.mass;
        pz += body.vz * body.mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

fn energy(bodies: Body[:]) -> f64 {
    var e = 0.0;
    for body, i in bodies {
        e += 0.5 * body.mass * (body.vx * body.vx + body.vy * body.vy + body.vz * body.vz);
        for j in i + 1..bodies.len {
            let other = bodies[j];
            let dx = body.x - other.x;
            let dy = body.y - other.y;
            let dz = body.z - other.z;
            e -= body.mass * other.mass / sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return e;
}

fn advance(bodies: Body[:], step: f64) {
    for i in bodies.len {
        for j in i + 1..bodies.len {
            let dx = bodies[i].x - bodies[j].x;
            let dy = bodies[i].y - bodies[j].y;
            let dz = bodies[i].z - bodies[j].z;
            let squared = dx * dx + dy * dy + dz * dz;
            let magnitude = step / (squared * sqrt(squared));
            bodies[i].vx -= dx * bodies[j].mass * magnitude;
            bodies[i].vy -= dy * bodies[j].mass * magnitude;
            bodies[i].vz -= dz * bodies[j].mass * magnitude;
            bodies[j].vx += dx * bodies[i].mass * magnitude;
            bodies[j].vy += dy * bodies[i].mass * magnitude;
            bodies[j].vz += dz * bodies[i].mass * magnitude;
        }
    }
    for &body in bodies {
        body.x += step * body.vx;
        body.y += step * body.vy;
        body.z += step * body.vz;
    }
}

// The energy rounded to 9 decimals, as an integer: prints the same in every language.
fn rounded(e: f64) -> i64 { round(e * 1e9) as i64 }

fn main() {
    let a = args();
    var steps = 15000000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { steps = v; } }
    var bodies: Body[>..] = [];
    initial_bodies(bodies);
    offset_momentum(bodies);
    print("energy before: ", rounded(energy(bodies)), " (x 1e-9)");
    for s in steps { advance(bodies, 0.01); }
    print("energy after: ", rounded(energy(bodies)), " (x 1e-9)");
}
