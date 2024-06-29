mod emulator;
use std::{collections::HashMap, fs::File, ops::Sub, thread::sleep, time};

use emulator::Emulator;
pub use macroquad::{input::KeyCode, prelude::*};
#[macroquad::main("Chip-8")]
async fn main() {
    let mut instruction_persec = 100000;
    let instruction_duration =
        |instruction_persec| time::Duration::from_secs_f64(1. / instruction_persec as f64);

    let mut emulator_instance = Emulator::new();

    let file_str: &str = "roms/TETRIS";
    let mut file = File::open(file_str).unwrap();
    emulator_instance.load_file_memory(&mut file).unwrap();

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
            let instruction = emulator_instance.fetch_instruction();
            println!("{:04x}", instruction);
            emulator_instance.execute(emulator_instance.extract_instruction(instruction));
            paused.1 = false;
        }
        let now_elapsed = now.elapsed();
        if now_elapsed.lt(&instruction_duration(instruction_persec)) {
            sleep(instruction_duration(instruction_persec).sub(now_elapsed));
        }

        let now_elapsed = now.elapsed();
        if now_elapsed.lt(&time::Duration::from_secs_f64(1. / 60. as f64)) {
            if is_key_pressed(KeyCode::Tab) {
                emulator_instance = Emulator::new();
                let mut file = File::open(file_str).unwrap();
                println!("reset");
                emulator_instance.load_file_memory(&mut file).unwrap();
            }
            if is_key_pressed(KeyCode::Space) {
                paused = (!paused.0, false);
            }
            if is_key_down(KeyCode::KpAdd) {
                instruction_persec += 1;
                println!("{}", instruction_persec);
            }
            if is_key_down(KeyCode::KpSubtract) {
                instruction_persec -= 1;
                println!("{}", instruction_persec);
            }
            if is_key_pressed(KeyCode::N) {
                paused = (true, true);
            }
            if !paused.0 | paused.1 {
                emulator_instance.decrement_timers();
            }
            emulator_instance.update_input(&key_map);

            let px_size = vec2(screen_width() / 64., screen_width() / 64.);
            clear_background(BLACK);
            emulator_instance
                .draw_px(&GREEN, &px_size, &vec2(0., 0.))
                .await;
            draw_text(
                &format!("{:09.2}", 1. / now_elapsed.as_secs_f64()),
                10.,
                30.,
                30.,
                WHITE,
            );

            next_frame().await;
        }
    }
}
