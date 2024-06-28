use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
};

use macroquad::prelude::*;
pub struct Emulator {
    memory: [u8; 4096],
    display_buf: [u8; 2048],
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: u16,
    index_register: u16,
    registers: [u8; 16],
    keys_pressed: Vec<u8>,
}

impl Emulator {
    pub fn new() -> Self {
        let mut out = Self {
            memory: [0; 4096],
            display_buf: [0; 2048],
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0x200,
            index_register: 0,
            registers: [0; 16],
            keys_pressed: Vec::new(),
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
    pub fn fetch_inst(&mut self) -> u16 {
        let addr = self.program_counter as usize;
        self.program_counter += 2;
        ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16)
    }
    pub fn extract_inst(&self, inst: u16) -> (u8, u8, u8, u8) {
        (
            ((inst & 0xF000) >> 12) as u8,
            ((inst & 0x0F00) >> 8) as u8,
            ((inst & 0x00F0) >> 4) as u8,
            (inst & 0x000F) as u8,
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
    pub fn execute(&mut self, inst: (u8, u8, u8, u8)) {
        match inst {
            (0x0, 0x0, 0xE, 0x0) => self.display_buf.fill(0),
            (0x0, 0x0, 0xE, 0xE) => self.ret_subr(),
            (0x1, h, m, l) => self.jump(self.to_u16(h, m, l)),
            (0x2, h, m, l) => self.jump_subr(self.to_u16(h, m, l)),
            (0x3, x, m, l) => self.skip_eq(x, self.to_u16(0, m, l)),
            (0x4, x, m, l) => self.skip_ne(x, self.to_u16(0, m, l)),
            (0x5, x, y, 0x0) => self.skip_eqxy(x, y),
            (0x6, x, m, l) => self.set_reg(x, self.to_u16(0, m, l)),
            (0x7, x, m, l) => self.add_reg(x, self.to_u16(0, m, l)),
            (0x8, x, y, 0x0) => self.set_regx_y(x, y),
            (0x8, x, y, 0x1) => self.or_xy(x, y),
            (0x8, x, y, 0x2) => self.and_xy(x, y),
            (0x8, x, y, 0x3) => self.xor_xy(x, y),
            (0x8, x, y, 0x4) => self.add_xy(x, y),
            (0x8, x, y, 0x5) => self.sub_xy(x, y),
            (0x8, x, y, 0x6) => self.shift_rx(x, y),
            (0x8, x, y, 0x7) => self.sub_yx(x, y),
            (0x8, x, y, 0xE) => self.shift_lx(x, y),
            (0x9, x, y, 0x0) => self.skip_nexy(x, y),
            (0xA, h, m, l) => self.set_ireg(self.to_u16(h, m, l)),
            (0xB, h, m, l) => self.jump_offset(self.to_u16(h, m, l)),
            (0xC, x, m, l) => self.gen_rand(x, self.to_u16(0, m, l)),
            (0xD, x, y, n) => self.draw_buf(x, y, n),
            (0xE, x, 0x9, 0xE) => self.jump_ifkey(x),
            (0xE, x, 0xA, 0x1) => self.jump_ifnkey(x),
            (0xF, x, 0x0, 0x7) => self.get_delay(x),
            (0xF, x, 0x0, 0xA) => self.wait_key(x),
            (0xF, x, 0x1, 0x5) => self.set_delay(x),
            (0xF, x, 0x1, 0x8) => self.set_sound(x),
            (0xF, x, 0x1, 0xE) => self.add_ireg(x),
            (0xF, x, 0x2, 0x9) => self.char_addr(x),
            (0xF, x, 0x3, 0x3) => self.to_bcd(x),
            (0xF, x, 0x5, 0x5) => self.store_regs(x),
            (0xF, x, 0x6, 0x5) => self.load_regs(x),
            _ => (),
        }
    }
    fn to_u16(&self, h: u8, m: u8, l: u8) -> u16 {
        ((h as u16) << 8) | ((m as u16) << 4) | l as u16
    }
    fn ret_subr(&mut self) {
        self.program_counter = self.stack.pop().unwrap_or(self.program_counter);
    }
    fn jump(&mut self, addr: u16) {
        self.program_counter = addr;
    }
    fn jump_subr(&mut self, addr: u16) {
        self.stack.push(self.program_counter);
        self.program_counter = addr;
    }
    fn skip_eq(&mut self, reg: u8, val: u16) {
        let reg = self.registers[reg as usize];
        if reg == val as u8 {
            self.program_counter += 2;
        }
    }
    fn skip_ne(&mut self, reg: u8, val: u16) {
        let reg = self.registers[reg as usize];
        if reg != val as u8 {
            self.program_counter += 2;
        }
    }
    fn skip_eqxy(&mut self, regx: u8, regy: u8) {
        let regx = self.registers[regx as usize];
        let regy = self.registers[regy as usize];
        if regx == regy {
            self.program_counter += 2;
        }
    }
    fn skip_nexy(&mut self, regx: u8, regy: u8) {
        let regx = self.registers[regx as usize];
        let regy = self.registers[regy as usize];
        if regx != regy {
            self.program_counter += 2;
        }
    }
    fn set_reg(&mut self, reg: u8, val: u16) {
        self.registers[reg as usize] = val as u8;
    }
    fn set_ireg(&mut self, val: u16) {
        self.index_register = val;
    }
    fn add_reg(&mut self, reg: u8, val: u16) {
        self.registers[reg as usize] = self.registers[reg as usize].wrapping_add(val as u8);
    }
    fn set_regx_y(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        *regx = regy.clone();
    }
    fn or_xy(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        *regx = *regx | regy.clone();
    }
    fn and_xy(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        *regx = *regx & regy.clone();
    }
    fn xor_xy(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        *regx = *regx ^ regy.clone();
    }
    fn add_xy(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        let vf: bool;
        (*regx, vf) = regx.overflowing_add(regy);
        self.registers[0xF as usize] = vf.into();
    }
    fn sub_xy(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        let vf: bool;
        (*regx, vf) = regx.overflowing_sub(regy);
        self.registers[0xF as usize] = (!vf).into();
    }
    fn shift_rx(&mut self, regx: u8, regy: u8) {
        let regx = &mut self.registers[regy as usize];
        // let regx = &mut self.registers[regx as usize];
        let treg = regx.clone();

        *regx = *regx >> 1;
        self.registers[0xF as usize] = treg & 0b00000001;
    }
    fn sub_yx(&mut self, regx: u8, regy: u8) {
        let regy = self.registers[regy as usize];
        let regx = &mut self.registers[regx as usize];
        let vf: bool;
        (*regx, vf) = regy.overflowing_sub(*regx);
        self.registers[0xF as usize] = (!vf).into();
    }
    fn shift_lx(&mut self, regx: u8, regy: u8) {
        let regx = &mut self.registers[regy as usize];
        // let regx = &mut self.registers[regx as usize];
        let treg = regx.clone();

        *regx = *regx << 1;
        self.registers[0xF as usize] = (treg & 0b10000000) >> 7;
    }
    fn jump_offset(&mut self, addr: u16) {
        self.program_counter = addr + self.registers[0] as u16;
    }
    fn gen_rand(&mut self, regx: u8, val: u16) {
        let regx = &mut self.registers[regx as usize];
        *regx = macroquad::rand::gen_range(0, 255) & val as u8;
    }
    fn jump_ifkey(&mut self, regx: u8) {
        if self.keys_pressed.contains(&self.registers[regx as usize]) {
            self.program_counter += 2;
        }
    }
    fn jump_ifnkey(&mut self, regx: u8) {
        if !self.keys_pressed.contains(&self.registers[regx as usize]) {
            self.program_counter += 2;
        }
    }
    fn get_delay(&mut self, regx: u8) {
        self.registers[regx as usize] = self.delay_timer;
    }
    fn wait_key(&mut self, regx: u8) {
        if self.keys_pressed.len() != 0 {
            self.registers[regx as usize] = self.keys_pressed[0];
        } else {
            self.program_counter -= 2;
        }
    }
    fn set_delay(&mut self, regx: u8) {
        self.delay_timer = self.registers[regx as usize];
    }
    fn set_sound(&mut self, regx: u8) {
        self.sound_timer = self.registers[regx as usize];
    }
    fn add_ireg(&mut self, reg: u8) {
        self.index_register = self
            .index_register
            .wrapping_add(self.registers[reg as usize] as u16);
        if self.index_register >= 0x1000 {
            self.registers[0xF as usize] = 1;
        }
    }
    fn char_addr(&mut self, regx: u8) {
        let chara = self.registers[regx as usize];
        self.index_register = (0x50 + chara * 5) as u16;
    }
    fn to_bcd(&mut self, regx: u8) {
        let x: &str = &format!("{:03}", self.registers[regx as usize]);

        let index = self.index_register;
        for r in 0..3 {
            self.memory[index as usize + r as usize] = x.chars().collect::<Vec<char>>()[r]
                .to_digit(10)
                .unwrap_or(0) as u8;
        }
    }
    fn store_regs(&mut self, regx: u8) {
        let index = self.index_register;
        for r in 0..=regx {
            self.memory[index as usize + r as usize] = self.registers[r as usize];
        }
    }
    fn load_regs(&mut self, regx: u8) {
        let index = self.index_register;
        for r in 0..=regx {
            self.registers[r as usize] = self.memory[index as usize + r as usize];
        }
    }
    fn draw_buf(&mut self, xreg: u8, yreg: u8, height: u8) {
        let x: usize = (self.registers[xreg as usize] % 64) as usize;
        let y: usize = (self.registers[yreg as usize] % 32) as usize;
        self.registers[0xF as usize] = 0;
        for i in 0..height {
            for b in 0..8 {
                let set = (self.memory[self.index_register as usize + i as usize] >> 7 - b) & 0x01;
                let addr: usize = x + b + (y + i as usize) * 64;
                if addr < 2048 {
                    let bit = self.display_buf[addr];
                    if bit == 1 && set == 1 {
                        self.registers[0xF as usize] = 1;
                        self.display_buf[addr] = 0;
                    } else if set == 1 {
                        self.display_buf[addr] = 1;
                    }
                }
            }
        }
    }
    pub async fn draw_px(&self, color: &Color, px_size: &Vec2, offset: &Vec2) {
        for (i, on) in self.display_buf.iter().enumerate() {
            let i = i;
            if *on == 1 {
                draw_rectangle(
                    (i % 64) as f32 * px_size.x + offset.x,
                    (i / 64) as f32 * px_size.y + offset.y,
                    px_size.x,
                    px_size.y,
                    *color,
                );
            }
        }
    }
    pub fn load_file_memory(&mut self, file: &mut File) -> io::Result<usize> {
        let mut reader = std::io::BufReader::new(file);
        reader.read(&mut self.memory[0x200..])
    }
}
