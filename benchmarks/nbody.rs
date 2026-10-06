// Floating point and struct access: a small gravitational simulation.
#[derive(Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    mass: f64,
}

fn energy(bodies: &[Body]) -> f64 {
    let mut e = 0.0;
    for i in 0..bodies.len() {
        let b = &bodies[i];
        e += 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
        for j in i + 1..bodies.len() {
            let c = &bodies[j];
            let (dx, dy, dz) = (b.x - c.x, b.y - c.y, b.z - c.z);
            e -= b.mass * c.mass / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    e
}

fn advance(bodies: &mut [Body], dt: f64) {
    let n = bodies.len();
    for i in 0..n {
        for j in i + 1..n {
            let (dx, dy, dz) = (bodies[i].x - bodies[j].x, bodies[i].y - bodies[j].y, bodies[i].z - bodies[j].z);
            let d2 = dx * dx + dy * dy + dz * dz;
            let mag = dt / (d2 * d2.sqrt());
            let (mi, mj) = (bodies[i].mass, bodies[j].mass);
            bodies[i].vx -= dx * mj * mag;
            bodies[i].vy -= dy * mj * mag;
            bodies[i].vz -= dz * mj * mag;
            bodies[j].vx += dx * mi * mag;
            bodies[j].vy += dy * mi * mag;
            bodies[j].vz += dz * mi * mag;
        }
    }
    for b in bodies.iter_mut() {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}

fn main() {
    let mut bodies = vec![
        Body { x: 0.0, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0, mass: 39.47 },
        Body { x: 4.84, y: -1.16, z: -0.10, vx: 0.606, vy: 2.81, vz: -0.02, mass: 0.037 },
        Body { x: 8.34, y: 4.12, z: -0.4, vx: -1.01, vy: 1.82, vz: 0.008, mass: 0.011 },
        Body { x: 12.89, y: -15.11, z: -0.22, vx: 1.08, vy: 0.868, vz: -0.01, mass: 0.0017 },
        Body { x: 15.37, y: -25.9, z: 0.179, vx: 0.979, vy: 0.594, vz: -0.034, mass: 0.002 },
    ];
    println!("{:.6}", energy(&bodies));
    for _ in 0..2_000_000 {
        advance(&mut bodies, 0.01);
    }
    println!("{:.6}", energy(&bodies));
}
