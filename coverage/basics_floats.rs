fn main() {
    let x = 2.0f64; let y = 0.5f32;
    println!("{} {} {}", x.sqrt(), x.powi(10), x.powf(0.5));
    println!("{:.6} {:.6} {:.6}", (1.0f64).sin(), (1.0f64).cos(), (1.0f64).tan());
    println!("{:.6} {:.6}", (1.0f64).exp(), (10.0f64).ln());
    println!("{} {} {} {}", 2.7f64.floor(), 2.3f64.ceil(), 2.5f64.round(), -2.7f64.trunc());
    println!("{} {}", (-3.5f64).abs(), 1.5f64.min(0.5));
    println!("{} {}", f64::NAN.is_nan(), (1.0f64 / 0.0).is_infinite());
    println!("{} {}", f64::INFINITY, f64::NEG_INFINITY);
    println!("{:.4} {:.4}", std::f64::consts::PI, std::f64::consts::E);
    println!("{:.4}", 180.0f64.to_radians());
    println!("{:.4}", 3.0f64.hypot(4.0));
    println!("{:.4}", (1.0f64).atan2(1.0));
    println!("{}", y * 3.0);
    println!("{} {}", 1e10, 1.5e-7);
    println!("{} {}", 0.1 + 0.2, 1.0 / 3.0);
    println!("{:e}", 1234.5f64);
    println!("{}", (2.0f64).mul_add(3.0, 1.0));
    println!("{} {}", 7.0f64 % 2.5, -7.5f64 % 2.0);
    println!("{}", f64::EPSILON > 0.0);
    println!("{} {}", 3.9f64 as i32, -3.9f64 as i32);
    println!("{} {}", 300.0f64 as u8, -5.0f64 as u8);
    println!("{}", (0.1f32 + 0.2f32) as f64);
    println!("{:?} {:?}", 1.0f64, 0.1f32);
    println!("{}", f64::MAX > 1e300);
    println!("{}", 2.0f64.clamp(0.0, 1.0));
}
