use rand::{thread_rng, Rng};
use sdl2::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    pixels::Color,
    rect::Point,
};
use std::{
    f32::consts::TAU,
    time::{Duration, Instant},
};

fn reset_player(
    px: &mut f32,
    py: &mut f32,
    vx: &mut f32,
    vy: &mut f32,
    viewport: (u32, u32),
    angle: &mut f32,
    health: &mut i32,
    score: &mut i32,
    last_hit: &mut Instant,
) {
    *px = viewport.0 as f32 / 2.0;
    *py = viewport.1 as f32 / 2.0;
    *vx = 0.0;
    *vy = 0.0;
    *angle = 0.0;
    *health = 3;
    *score = 0;
    *last_hit = Instant::now() - Duration::from_secs(5);
}

// take ship coords, apply offset, return position
fn translate_coords_to_pos(points: &[Point], offset: Point) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point::new(p.x + offset.x, p.y + offset.y))
        .collect()
}

fn scale_outline(points: &[Point], scale: f32) -> Vec<Point> {
    points
        .iter()
        .map(|p| {
            let x = p.x as f32 * scale;
            let y = p.y as f32 * scale;
            Point::new(x.round() as i32, y.round() as i32)
        })
        .collect()
}

// keep position relative to screen
fn adjust_pos_for_resize(
    px: &mut f32,
    py: &mut f32,
    old_viewport: (u32, u32),
    new_viewport: (u32, u32),
) {
    let old_center_x = old_viewport.0 as f32 / 2.0;
    let old_center_y = old_viewport.1 as f32 / 2.0;
    let offset_x = *px - old_center_x;
    let offset_y = *py - old_center_y;

    let new_center_x = new_viewport.0 as f32 / 2.0;
    let new_center_y = new_viewport.1 as f32 / 2.0;
    *px = new_center_x + offset_x;
    *py = new_center_y + offset_y;
}

fn rotate(points: &[Point], angle: f32) -> Vec<Point> {
    let (sin_a, cos_a) = angle.sin_cos();
    points
        .iter()
        .map(|p| {
            let x = p.x as f32;
            let y = p.y as f32;
            let rot_x = x * cos_a - y * sin_a;
            let rot_y = x * sin_a + y * cos_a;
            Point::new(rot_x.round() as i32, rot_y.round() as i32)
        })
        .collect()
}

struct Projectile {
    pos: (f32, f32),
    vel: (f32, f32),
    angle: f32,
    radius: f32,
}

#[derive(Clone, Copy)]
enum AsteroidSize {
    Large,
    Medium,
}

impl AsteroidSize {
    fn next(self) -> Option<Self> {
        match self {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => None,
        }
    }

    fn scale_range(self) -> std::ops::Range<f32> {
        match self {
            AsteroidSize::Large => 1.1..1.5,
            AsteroidSize::Medium => 0.7..1.0,
        }
    }
}

struct Asteroid {
    pos: (f32, f32),
    vel: (f32, f32),
    angle: f32,
    shape: Vec<Point>,
    radius: f32,
    size: AsteroidSize,
}

fn spawn_asteroid(
    base_shape: &[Point],
    size: AsteroidSize,
    pos: (f32, f32),
    vel: (f32, f32),
    angle: f32,
    rng: &mut impl Rng,
) -> Asteroid {
    let scale = rng.gen_range(size.scale_range());
    let shape = scale_outline(base_shape, scale);
    let radius = shape
        .iter()
        .map(|p| ((p.x.pow(2) + p.y.pow(2)) as f32).sqrt())
        .fold(0.0, f32::max); // max dist from origin
    Asteroid {
        pos,
        vel,
        angle,
        shape,
        radius,
        size,
    }
}

fn split_asteroid(
    asteroid: &Asteroid,
    rng: &mut impl Rng,
    shapes: &[&[Point]],
    speed_range: std::ops::Range<f32>,
) -> Vec<Asteroid> {
    let mut pieces = Vec::new();
    if let Some(next_size) = asteroid.size.next() {
        for _ in 0..2 {
            let base = shapes[rng.gen_range(0..shapes.len())];
            let vel = pick_random_velocity(rng, speed_range.clone());
            let angle = rng.gen_range(0.0..TAU);
            pieces.push(spawn_asteroid(
                base,
                next_size,
                asteroid.pos,
                vel,
                angle,
                rng,
            ));
        }
    }
    pieces
}

fn pick_spawn_point(rng: &mut impl Rng, viewport: (u32, u32), margin: f32) -> (f32, f32) {
    let (w, h) = (viewport.0 as f32, viewport.1 as f32);
    match rng.gen_range(0..4) {
        0 => (-margin, rng.gen_range(0.0..h)),
        1 => (w + margin, rng.gen_range(0.0..h)),
        2 => (rng.gen_range(0.0..w), -margin),
        _ => (rng.gen_range(0.0..w), h + margin),
    }
}

fn pick_random_velocity(rng: &mut impl Rng, speed_range: std::ops::Range<f32>) -> (f32, f32) {
    let angle = rng.gen_range(0.0..TAU);
    let speed = rng.gen_range(speed_range.clone());
    let (sin, cos) = angle.sin_cos();
    (speed * sin, -speed * cos)
}

fn check_collision(a_pos: (f32, f32), a_radius: f32, b_pos: (f32, f32), b_radius: f32) -> bool {
    let dx = a_pos.0 - b_pos.0;
    let dy = a_pos.1 - b_pos.1;
    let sum = a_radius + b_radius;
    dx * dx + dy * dy <= sum * sum
}

fn wrap_position(pos: &mut (f32, f32), viewport: (u32, u32), margin: f32) {
    let (w, h) = (viewport.0 as f32, viewport.1 as f32);
    if pos.0 < -margin {
        pos.0 = w + margin;
    } else if pos.0 > w + margin {
        pos.0 = -margin;
    }

    if pos.1 < -margin {
        pos.1 = h + margin;
    } else if pos.1 > h + margin {
        pos.1 = -margin;
    }
}

fn wrap_player(px: &mut f32, py: &mut f32, viewport: (u32, u32), margin: f32) {
    let mut p = (*px, *py);
    wrap_position(&mut p, viewport, margin);
    *px = p.0;
    *py = p.1;
}

fn main() -> Result<(), String> {
    let window_title = "Blasteroids";
    let window_width: u32 = 1280;
    let window_height: u32 = 840; // compiler infers u32 so technically no type hint is needed
    let mut viewport = (window_width, window_height);
    let mut rng = thread_rng();

    // asteroids
    let asteroid_outline_a = [
        Point::new(0, -34),
        Point::new(18, -30),
        Point::new(28, -16),
        Point::new(20, -6),
        Point::new(30, 4),
        Point::new(18, 18),
        Point::new(4, 12),
        Point::new(-4, 30),
        Point::new(-20, 18),
        Point::new(-30, 10),
        Point::new(-18, 0),
        Point::new(-32, -10),
        Point::new(-18, -24),
        Point::new(-6, -14),
        Point::new(0, -34),
    ];

    let asteroid_outline_b = [
        Point::new(-4, -28),
        Point::new(16, -24),
        Point::new(24, -12),
        Point::new(12, -8),
        Point::new(30, -2),
        Point::new(22, 12),
        Point::new(8, 10),
        Point::new(10, 24),
        Point::new(-4, 26),
        Point::new(-12, 14),
        Point::new(-24, 26),
        Point::new(-20, 6),
        Point::new(-32, 0),
        Point::new(-22, -14),
        Point::new(-8, -18),
        Point::new(-4, -28),
    ];

    let asteroid_outline_c = [
        Point::new(0, -30),
        Point::new(12, -22),
        Point::new(8, -12),
        Point::new(24, -8),
        Point::new(26, 2),
        Point::new(14, 8),
        Point::new(18, 22),
        Point::new(4, 18),
        Point::new(-2, 28),
        Point::new(-12, 14),
        Point::new(-26, 18),
        Point::new(-20, 4),
        Point::new(-30, -6),
        Point::new(-14, -22),
        Point::new(-4, -12),
        Point::new(0, -30),
    ];

    let asteroid_shapes = [
        &asteroid_outline_a[..],
        &asteroid_outline_b[..],
        &asteroid_outline_c[..],
    ];
    let mut asteroids = Vec::new();
    let speed_range = 1.0..3.0;
    let asteroid_margin = 40.0;
    let mut dead_asteroids = Vec::new();

    const STARTING_ASTEROIDS: usize = 15;
    while asteroids.len() < STARTING_ASTEROIDS {
        let base = asteroid_shapes[rng.gen_range(0..asteroid_shapes.len())];
        let pos = pick_spawn_point(&mut rng, viewport, asteroid_margin);
        let vel = pick_random_velocity(&mut rng, speed_range.clone());
        let angle = rng.gen_range(0.0..TAU);
        asteroids.push(spawn_asteroid(
            base,
            AsteroidSize::Large,
            pos,
            vel,
            angle,
            &mut rng,
        ));
    }

    // shooting projectile
    let projectile_outline = scale_outline(&[Point::new(0, -5), Point::new(0, -12)], 1.5);
    let mut projectiles: Vec<Projectile> = Vec::new();
    let projectile_speed = 9.0;
    let mut dead_projectiles = Vec::new();

    // ship outlines
    let ship_outline = scale_outline(
        &[
            Point::new(0, -14),
            Point::new(10, 12),
            Point::new(0, 6),
            Point::new(-10, 12),
            Point::new(0, -14),
        ],
        1.5,
    );
    let ship_thrust_outline = scale_outline(
        &[
            Point::new(-10, 12),
            Point::new(0, 6),
            Point::new(10, 12),
            Point::new(0, 26),
            Point::new(-10, 12),
        ],
        1.5,
    );
    // Player vars
    let mut angle: f32 = 0.0;
    let mut vx: f32 = 0.0;
    let mut vy: f32 = 0.0;
    let mut px: f32 = (window_width / 2) as f32;
    let mut py: f32 = (window_height / 2) as f32;
    let turn_speed: f32 = 0.07; // in radian
    let acceleration: f32 = 0.2;
    let ship_radius = 10.0;
    let player_margin = 0.0;
    let mut thrusting: bool = false;
    let mut turning_left: bool = false;
    let mut turning_right: bool = false;
    let mut player_health = 3;
    let mut player_score = 0;
    const IFRAME_DURATION: Duration = Duration::from_millis(800);
    let mut last_hit = std::time::Instant::now() - Duration::from_secs(5);

    // init systems / window
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let ttf_ctx = sdl2::ttf::init().map_err(|err| err.to_string())?;
    let _audio = sdl.audio()?;

    // mixer stuff
    sdl2::mixer::open_audio(44_100, sdl2::mixer::AUDIO_S16LSB, 5, 1_024)?;
    sdl2::mixer::init(sdl2::mixer::InitFlag::OGG | sdl2::mixer::InitFlag::MP3)?;
    sdl2::mixer::allocate_channels(16);

    let window = video
        .window(window_title, window_width, window_height)
        .position_centered()
        .resizable()
        .build()
        .map_err(|error| format!("Failed to create window: {}", error))?;
    let mut canvas = window.into_canvas().accelerated().build().unwrap();
    let mut events = sdl.event_pump()?;
    let texture_creator = canvas.texture_creator();

    // load font
    let font_path = "assets/upheavtt.ttf";
    let font_size = 50;
    let font = ttf_ctx.load_font(font_path, font_size)?;

    // load sfx
    let laser_sfx = sdl2::mixer::Chunk::from_file("assets/shoot.wav")?;
    let explosion_sfx = sdl2::mixer::Chunk::from_file("assets/explosion.wav")?;
    let hurt_sfx = sdl2::mixer::Chunk::from_file("assets/hurt.wav")?;

    // Game loop
    'running: loop {
        for event in events.poll_iter() {
            match event {
                // close window
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                // update viewport on resize
                Event::Window {
                    win_event: WindowEvent::Resized(w, h),
                    ..
                } => {
                    let old_viewport = viewport;
                    viewport = (w as u32, h as u32);
                    adjust_pos_for_resize(&mut px, &mut py, old_viewport, viewport);
                }

                // player controls
                Event::KeyDown {
                    keycode: Some(code),
                    repeat: false,
                    ..
                } => match code {
                    Keycode::Up => thrusting = true,
                    Keycode::Left => turning_left = true,
                    Keycode::Right => turning_right = true,
                    Keycode::Space => {
                        let (sin, cos) = angle.sin_cos();
                        sdl2::mixer::Channel::all().play(&laser_sfx, 0)?;
                        projectiles.push(Projectile {
                            pos: (px, py),
                            vel: (projectile_speed * sin, -projectile_speed * cos),
                            angle,
                            radius: 3.0,
                        });
                    }
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(code),
                    ..
                } => match code {
                    Keycode::Up => thrusting = false,
                    Keycode::Left => turning_left = false,
                    Keycode::Right => turning_right = false,
                    _ => {}
                },
                // default
                _ => {}
            }
        }
        // update projectiles
        for p in projectiles.iter_mut() {
            p.pos.0 += p.vel.0;
            p.pos.1 += p.vel.1;
        }

        // update asteroids
        for asteroid in asteroids.iter_mut() {
            asteroid.pos.0 += asteroid.vel.0;
            asteroid.pos.1 += asteroid.vel.1;
            asteroid.angle = (asteroid.angle + 0.01) % TAU;
            wrap_position(&mut asteroid.pos, viewport, asteroid_margin);
        }

        // Movement
        if thrusting {
            let (sin, cos) = angle.sin_cos();
            vx += acceleration * sin;
            vy += -acceleration * cos;
        } else if !thrusting {
            let drag = 0.98_f32; // smaller number -> stronger breaking
            vx *= drag;
            vy *= drag;
        }

        // Rotation
        if turning_left {
            angle -= turn_speed;
        }
        if turning_right {
            angle += turn_speed;
        }

        // compute new position
        px += vx;
        py += vy;
        wrap_player(&mut px, &mut py, viewport, player_margin);

        // clamp to screen border
        let (vw, vh) = viewport;
        /* let max_x = vw as f32 - margin;
        let max_y = vh as f32 - margin;
        px = px.clamp(margin, max_x);
        py = py.clamp(margin, max_y);  */

        // Check player collision
        let now = Instant::now();
        let invulnerable_elapsed = now.duration_since(last_hit);
        let invulnerable = invulnerable_elapsed < IFRAME_DURATION;

        for asteroid in &asteroids {
            if now.duration_since(last_hit) >= IFRAME_DURATION
                && check_collision((px, py), ship_radius, asteroid.pos, asteroid.radius)
            {
                player_health -= 1;
                sdl2::mixer::Channel::all().play(&hurt_sfx, 0)?;
                last_hit = now;
            }
        }

        if player_health == 0 {
            reset_player(
                &mut px,
                &mut py,
                &mut vx,
                &mut vy,
                viewport,
                &mut angle,
                &mut player_health,
                &mut player_score,
                &mut last_hit,
            );
            asteroids.clear();
            projectiles.clear();
            continue;
        }

        let mut spawned_children = Vec::new();

        // Check projectile collision
        'asteroid_scan: for (ai, asteroid) in asteroids.iter().enumerate() {
            for (pi, projectile) in projectiles.iter().enumerate() {
                if check_collision(
                    projectile.pos,
                    projectile.radius,
                    asteroid.pos,
                    asteroid.radius,
                ) {
                    player_score += 10;
                    sdl2::mixer::Channel::all().play(&explosion_sfx, 0)?;
                    dead_asteroids.push(ai);
                    dead_projectiles.push(pi);
                    spawned_children.extend(split_asteroid(
                        asteroid,
                        &mut rng,
                        &asteroid_shapes,
                        speed_range.clone(),
                    ));
                    break 'asteroid_scan;
                }
            }
        }
        // delete when destroyed
        dead_asteroids.sort_unstable();
        dead_asteroids.dedup();
        for &idx in dead_asteroids.iter().rev() {
            asteroids.remove(idx);
        }
        dead_asteroids.clear();

        for &idx in dead_projectiles.iter().rev() {
            projectiles.remove(idx);
        }
        dead_projectiles.clear();

        asteroids.extend(spawned_children.into_iter());

        // continuesly spawn asteroids
        while asteroids.len() < STARTING_ASTEROIDS {
            let base = asteroid_shapes[rng.gen_range(0..asteroid_shapes.len())];
            let pos = pick_spawn_point(&mut rng, viewport, asteroid_margin);
            let vel = pick_random_velocity(&mut rng, speed_range.clone());
            let angle = rng.gen_range(0.0..TAU);
            asteroids.push(spawn_asteroid(
                base,
                AsteroidSize::Large,
                pos,
                vel,
                angle,
                &mut rng,
            ));
        }

        // destroy off-screen projectiles
        projectiles.retain(|p| {
            let x = p.pos.0;
            let y = p.pos.1;
            x >= 0.0 && x <= vw as f32 && y >= 0.0 && y <= vh as f32
        });

        // keep angle < 360
        angle = (angle + TAU) % TAU;

        // movement & rotation
        let rot_ship = rotate(&ship_outline, angle);
        let rot_thrust = rotate(&ship_thrust_outline, angle);
        let player_pos = Point::new(px.round() as i32, py.round() as i32);
        let ship_screen_points = translate_coords_to_pos(&rot_ship, player_pos); // ship pos on screen
        let thrust_screen_points = translate_coords_to_pos(&rot_thrust, player_pos);

        // draw score
        let score_text = format!("{}", player_score);
        let score_surface = font
            .render(&score_text)
            .blended(Color::RGB(255, 255, 255))
            .map_err(|err| err.to_string())?;
        let (text_w, text_h) = score_surface.size();
        let score_texture = texture_creator
            .create_texture_from_surface(&score_surface)
            .map_err(|err| err.to_string())?;
        let margin = 12;
        let x = (viewport.0 as i32) - (text_w as i32) - margin;
        let y = margin - 10;
        let score_dest = sdl2::rect::Rect::new(x, y, text_w, text_h);

        // draw bg
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        // draw score in top right
        canvas.copy(&score_texture, None, Some(score_dest))?;

        // draw lives in top left
        let hud_margin = 36.0_f32;
        let hud_spacing = 36.0_f32;
        for i in 0..player_health {
            let x = hud_margin + i as f32 * hud_spacing;
            let y = hud_margin;

            let life_pos = Point::new(x.round() as i32, y.round() as i32);
            let life_screen_points = translate_coords_to_pos(&ship_outline, life_pos);
            canvas.draw_lines(life_screen_points.as_slice())?;
        }

        // draw asteroids
        for asteroid in &asteroids {
            let rotated = rotate(&asteroid.shape, asteroid.angle);
            let pos = Point::new(asteroid.pos.0.round() as i32, asteroid.pos.1.round() as i32);
            let screen_points = translate_coords_to_pos(&rotated, pos);
            canvas.draw_lines(screen_points.as_slice())?;
        }

        // draw projectiles
        for p in &projectiles {
            let pos = Point::new(p.pos.0.round() as i32, p.pos.1.round() as i32);
            let rot_projectile = rotate(&projectile_outline, p.angle);
            let translated = translate_coords_to_pos(&rot_projectile, pos);
            canvas.draw_lines(translated.as_slice())?;
        }

        // draw player
        if !invulnerable || (invulnerable_elapsed.as_millis() / 100) % 2 == 0 {
            canvas.draw_lines(ship_screen_points.as_slice())?;
            if thrusting {
                canvas.draw_lines(thrust_screen_points.as_slice())?;
            }
        }

        // render
        canvas.present();

        // time between frames
        std::thread::sleep(Duration::from_millis(16)); // ~60 fps
    }
    Ok(())
}
