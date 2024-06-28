mod emulator;
use std::{collections::HashMap, fs::File, ops::Sub, thread::sleep, time};

use emulator::Emulator;
pub use macroquad::{input::KeyCode, prelude::*};
#[macroquad::main("Chip-8")]
async fn main() {
    let mut insts_sec = 2000;
    let mut inst_duration = time::Duration::from_secs_f64(1. / insts_sec as f64);

    let mut emu = Emulator::new();

    let file: &str = "TETRIS";
    let mut rom = File::open(file).unwrap();
    emu.load_file_memory(&mut rom).unwrap();

    let key_map: HashMap<KeyCode, u8> = HashMap::from([
        (KeyCode::Key1, 0),
        (KeyCode::Key2, 1),
        (KeyCode::Key3, 2),
        (KeyCode::Key4, 3),
        (KeyCode::Q, 4),
        (KeyCode::W, 5),
        (KeyCode::E, 6),
        (KeyCode::R, 7),
        (KeyCode::A, 8),
        (KeyCode::S, 9),
        (KeyCode::D, 0xA),
        (KeyCode::F, 0xB),
        (KeyCode::Z, 0xC),
        (KeyCode::X, 0xD),
        (KeyCode::C, 0xE),
        (KeyCode::V, 0xF),
    ]);

    let mut paused = (true, false);
    loop {
        let now = time::Instant::now();

        if !paused.0 | paused.1 {
            let inst = emu.fetch_inst();
            println!("{:04x}", inst);
            emu.execute(emu.extract_inst(inst));
            paused.1 = false;
        }
        let t = now.elapsed();
        if t.lt(&inst_duration) {
            sleep(inst_duration.sub(t));
        }

        let t = now.elapsed();
        if t.lt(&time::Duration::from_secs_f64(1. / 60. as f64)) {
            if is_key_pressed(KeyCode::Tab) {
                emu = Emulator::new();
                let mut rom = File::open(file).unwrap();
                emu.load_file_memory(&mut rom).unwrap();
                println!("reset");
            }
            if is_key_pressed(KeyCode::Space) {
                paused = (!paused.0, false);
            }
            if is_key_down(KeyCode::KpAdd) {
                insts_sec += 1;
                inst_duration = time::Duration::from_secs_f64(1. / insts_sec as f64);
                println!("{}", insts_sec);
            }
            if is_key_down(KeyCode::KpSubtract) {
                insts_sec -= 1;
                inst_duration = time::Duration::from_secs_f64(1. / insts_sec as f64);
                println!("{}", insts_sec);
            }
            if is_key_pressed(KeyCode::N) {
                paused = (true, true);
            }
            if !paused.0 | paused.1 {
                emu.decrement_timers();
            }
            emu.update_input(&key_map);

            let px_size = vec2(screen_width() / 64., screen_width() / 64.);
            clear_background(BLACK);
            emu.draw_px(&GREEN, &px_size, &vec2(0., 0.)).await;
            draw_text(&format!("{:09.2}", 1. / t.as_secs_f64()), 10., 30., 30., WHITE);

            next_frame().await;
        }
    }
}
