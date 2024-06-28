use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
};

use macroquad::prelude::*;
pub struct Emulator {
    memory: [u8; 4096],
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: u16,
    index_register: u16,
    registers: [u8; 16],
    keys_pressed: Vec<u8>,
    display_buffer: [u8; 2048],
    dirty_rects: HashMap<(usize, usize), Rect>,
}

impl Emulator {
    pub fn new() -> Self {
        let mut out = Self {
            memory: [0; 4096],
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0x200,
            index_register: 0,
            registers: [0; 16],
            keys_pressed: Vec::new(),
            display_buffer: [0; 2048],
            dirty_rects: HashMap::new(),
        };
        out.memory[0x50..=0x9F].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]);
        out
    }
    pub fn fetch_instruction(&mut self) -> u16 {
        let address = self.program_counter as usize;
        self.program_counter += 2;
        ((self.memory[address] as u16) << 8) | (self.memory[address + 1] as u16)
    }
    pub fn extract_instruction(&self, instruction: u16) -> (u8, u8, u8, u8) {
        (
            ((instruction & 0xF000) >> 12) as u8,
            ((instruction & 0x0F00) >> 8) as u8,
            ((instruction & 0x00F0) >> 4) as u8,
            (instruction & 0x000F) as u8,
        )
    }
    pub fn decrement_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
    pub fn update_input(&mut self, map: &HashMap<KeyCode, u8>) {
        for key in map.keys() {
            if is_key_pressed(*key) {
                self.keys_pressed.push(map[key]);
            }
            if is_key_released(*key) {
                let index = self
                    .keys_pressed
                    .iter()
                    .position(|x| *x == map[key])
                    .unwrap();
                self.keys_pressed.remove(index);
            }
        }
    }
    pub fn execute(&mut self, instruction: (u8, u8, u8, u8)) {
        match instruction {
            (0x0, 0x0, 0xE, 0x0) => self.display_buffer.fill(0),
            (0x0, 0x0, 0xE, 0xE) => self.return_subroutine(),
            (0x1, h, m, l) => self.jump(self.to_u16(h, m, l)),
            (0x2, h, m, l) => self.jump_subroutine(self.to_u16(h, m, l)),
            (0x3, x, m, l) => self.skip_eq(x, self.to_u16(0, m, l)),
            (0x4, x, m, l) => self.skip_ne(x, self.to_u16(0, m, l)),
            (0x5, x, y, 0x0) => self.skip_x_eq_y(x, y),
            (0x6, x, m, l) => self.set_register(x, self.to_u16(0, m, l)),
            (0x7, x, m, l) => self.add_register(x, self.to_u16(0, m, l)),
            (0x8, x, y, 0x0) => self.set_registerx_y(x, y),
            (0x8, x, y, 0x1) => self.or_xy(x, y),
            (0x8, x, y, 0x2) => self.and_xy(x, y),
            (0x8, x, y, 0x3) => self.xor_xy(x, y),
            (0x8, x, y, 0x4) => self.add_xy(x, y),
            (0x8, x, y, 0x5) => self.sub_xy(x, y),
            (0x8, x, y, 0x6) => self.shift_x_right(x, y),
            (0x8, x, y, 0x7) => self.sub_yx(x, y),
            (0x8, x, y, 0xE) => self.shift_x_left(x, y),
            (0x9, x, y, 0x0) => self.skip_x_ne_y(x, y),
            (0xA, h, m, l) => self.set_iregister(self.to_u16(h, m, l)),
            (0xB, h, m, l) => self.jump_offset(self.to_u16(h, m, l)),
            (0xC, x, m, l) => self.generate_random(x, self.to_u16(0, m, l)),
            (0xD, x, y, n) => self.draw_buffer(x, y, n),
            (0xE, x, 0x9, 0xE) => self.jump_ifkey(x),
            (0xE, x, 0xA, 0x1) => self.jump_ifnkey(x),
            (0xF, x, 0x0, 0x7) => self.get_delay(x),
            (0xF, x, 0x0, 0xA) => self.wait_key(x),
            (0xF, x, 0x1, 0x5) => self.set_delay(x),
            (0xF, x, 0x1, 0x8) => self.set_sound(x),
            (0xF, x, 0x1, 0xE) => self.add_iregister(x),
            (0xF, x, 0x2, 0x9) => self.char_address(x),
            (0xF, x, 0x3, 0x3) => self.to_bcd(x),
            (0xF, x, 0x5, 0x5) => self.store_registers(x),
            (0xF, x, 0x6, 0x5) => self.load_registers(x),
            _ => (),
        }
    }
    fn to_u16(&self, h: u8, m: u8, l: u8) -> u16 {
        ((h as u16) << 8) | ((m as u16) << 4) | l as u16
    }
    fn return_subroutine(&mut self) {
        self.program_counter = self.stack.pop().unwrap_or(self.program_counter);
    }
    fn jump(&mut self, address: u16) {
        self.program_counter = address;
    }
    fn jump_subroutine(&mut self, address: u16) {
        self.stack.push(self.program_counter);
        self.program_counter = address;
    }
    fn skip_eq(&mut self, register: u8, val: u16) {
        let register = self.registers[register as usize];
        if register == val as u8 {
            self.program_counter += 2;
        }
    }
    fn skip_ne(&mut self, register: u8, val: u16) {
        let register = self.registers[register as usize];
        if register != val as u8 {
            self.program_counter += 2;
        }
    }
    fn skip_x_eq_y(&mut self, x_register: u8, y_register: u8) {
        let x_register = self.registers[x_register as usize];
        let y_register = self.registers[y_register as usize];
        if x_register == y_register {
            self.program_counter += 2;
        }
    }
    fn skip_x_ne_y(&mut self, x_register: u8, y_register: u8) {
        let x_register = self.registers[x_register as usize];
        let y_register = self.registers[y_register as usize];
        if x_register != y_register {
            self.program_counter += 2;
        }
    }
    fn set_register(&mut self, register: u8, val: u16) {
        self.registers[register as usize] = val as u8;
    }
    fn set_iregister(&mut self, val: u16) {
        self.index_register = val;
    }
    fn add_register(&mut self, register: u8, val: u16) {
        self.registers[register as usize] =
            self.registers[register as usize].wrapping_add(val as u8);
    }
    fn set_registerx_y(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        *x_register = y_register.clone();
    }
    fn or_xy(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        *x_register = *x_register | y_register.clone();
    }
    fn and_xy(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        *x_register = *x_register & y_register.clone();
    }
    fn xor_xy(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        *x_register = *x_register ^ y_register.clone();
    }
    fn add_xy(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        let vf: bool;
        (*x_register, vf) = x_register.overflowing_add(y_register);
        self.registers[0xF as usize] = vf.into();
    }
    fn sub_xy(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        let vf: bool;
        (*x_register, vf) = x_register.overflowing_sub(y_register);
        self.registers[0xF as usize] = (!vf).into();
    }
    fn shift_x_right(&mut self, x_register: u8, y_register: u8) {
        let x_register = &mut self.registers[y_register as usize];
        // let x_register = &mut self.registers[x_register as usize];
        let tregister = x_register.clone();

        *x_register = *x_register >> 1;
        self.registers[0xF as usize] = tregister & 0b00000001;
    }
    fn sub_yx(&mut self, x_register: u8, y_register: u8) {
        let y_register = self.registers[y_register as usize];
        let x_register = &mut self.registers[x_register as usize];
        let vf: bool;
        (*x_register, vf) = y_register.overflowing_sub(*x_register);
        self.registers[0xF as usize] = (!vf).into();
    }
    fn shift_x_left(&mut self, x_register: u8, y_register: u8) {
        let x_register = &mut self.registers[y_register as usize];
        // let x_register = &mut self.registers[x_register as usize];
        let tregister = x_register.clone();

        *x_register = *x_register << 1;
        self.registers[0xF as usize] = (tregister & 0b10000000) >> 7;
    }
    fn jump_offset(&mut self, address: u16) {
        self.program_counter = address + self.registers[0] as u16;
    }
    fn generate_random(&mut self, x_register: u8, val: u16) {
        let x_register = &mut self.registers[x_register as usize];
        *x_register = macroquad::rand::gen_range(0, 255) & val as u8;
    }
    fn jump_ifkey(&mut self, x_register: u8) {
        if self
            .keys_pressed
            .contains(&self.registers[x_register as usize])
        {
            self.program_counter += 2;
        }
    }
    fn jump_ifnkey(&mut self, x_register: u8) {
        if !self
            .keys_pressed
            .contains(&self.registers[x_register as usize])
        {
            self.program_counter += 2;
        }
    }
    fn get_delay(&mut self, x_register: u8) {
        self.registers[x_register as usize] = self.delay_timer;
    }
    fn wait_key(&mut self, x_register: u8) {
        if self.keys_pressed.len() != 0 {
            self.registers[x_register as usize] = self.keys_pressed[0];
        } else {
            self.program_counter -= 2;
        }
    }
    fn set_delay(&mut self, x_register: u8) {
        self.delay_timer = self.registers[x_register as usize];
    }
    fn set_sound(&mut self, x_register: u8) {
        self.sound_timer = self.registers[x_register as usize];
    }
    fn add_iregister(&mut self, register: u8) {
        self.index_register = self
            .index_register
            .wrapping_add(self.registers[register as usize] as u16);
        if self.index_register >= 0x1000 {
            self.registers[0xF as usize] = 1;
        }
    }
    fn char_address(&mut self, x_register: u8) {
        let chara = self.registers[x_register as usize];
        self.index_register = (0x50 + chara * 5) as u16;
    }
    fn to_bcd(&mut self, x_register: u8) {
        let x: &str = &format!("{:03}", self.registers[x_register as usize]);

        let index = self.index_register;
        for r in 0..3 {
            self.memory[index as usize + r as usize] = x.chars().collect::<Vec<char>>()[r]
                .to_digit(10)
                .unwrap_or(0) as u8;
        }
    }
    fn store_registers(&mut self, x_register: u8) {
        let index = self.index_register;
        for r in 0..=x_register {
            self.memory[index as usize + r as usize] = self.registers[r as usize];
        }
    }
    fn load_registers(&mut self, x_register: u8) {
        let index = self.index_register;
        for r in 0..=x_register {
            self.registers[r as usize] = self.memory[index as usize + r as usize];
        }
    }
    fn draw_buffer(&mut self, x_register: u8, y_register: u8, height: u8) {
        let x: usize = (self.registers[x_register as usize] % 64) as usize;
        let y: usize = (self.registers[y_register as usize] % 32) as usize;
        self.registers[0xF as usize] = 0;
        for i in 0..height {
            for b in 0..8 {
                let set = (self.memory[self.index_register as usize + i as usize] >> 7 - b) & 0x01;
                let address: usize = x + b + (y + i as usize) * 64;
                if address < 2048 {
                    let bit = self.display_buffer[address];
                    if bit == 1 && set == 1 {
                        self.registers[0xF as usize] = 1;
                        self.display_buffer[address] = 0;
                        self.dirty_rects.remove(&(x + b, y + i as usize));
                    } else if set == 1 {
                        self.display_buffer[address] = 1;
                        self.dirty_rects.insert(
                            (x + b, y + i as usize),
                            Rect::new((x + b) as f32, (y + i as usize) as f32, 1., 1.),
                        );
                    }
                }
            }
        }
    }
    pub async fn draw_px(&self, color: &Color, px_size: &Vec2, offset: &Vec2) {
        for (_, px) in self.dirty_rects.iter() {
            draw_rectangle(
                px.x * px_size.x + offset.x,
                px.y * px_size.y + offset.y,
                px_size.x,
                px_size.y,
                *color,
            );
        }
    }
    pub fn load_file_memory(&mut self, file: &mut File) -> io::Result<usize> {
        let mut reader = std::io::BufReader::new(file);
        reader.read(&mut self.memory[0x200..])
    }
}
