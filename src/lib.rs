#![no_std]
mod dotfont;

extern crate embedded_hal as hal;

use crate::dotfont::{DOT_CHAR_WIDTH, DOT_FONT};
use hal::blocking::i2c::Write;

pub const DEFAULT_I2C_ADDRESS: u8 = 0x61;
pub const SECONDARY_I2C_ADDRESS: u8 = 0x63;

pub const I2C_ADDRESS_ALTERNATE1: u8 = 0x62;
pub const I2C_ADDRESS_ALTERNATE2: u8 = 0x63;
pub const DEFAULT_BRIGHTNESS: u8 = 64;
pub const MAX_BRIGHTNESS: u8 = 127;
pub const DEFAULT_ON_LEVEL: u8 = 0x7f;
pub const WIDTH: u8 = 10;
pub const HEIGHT: u8 = 7;

const MODE: u8 = 0b00011000;
const OPTS: u8 = 0b00001110; // 1110 = 35mA, 0000 = 40mA
const CMD_BRIGHTNESS: u8 = 0x19;
const CMD_MODE: u8 = 0x00;
const CMD_UPDATE: u8 = 0x0C;
const CMD_OPTIONS: u8 = 0x0D;

const CMD_MATRIX_L: u8 = 0x0E;
const CMD_MATRIX_R: u8 = 0x01;

const BUFFER_LENGTH: u8 = 8;
const BUFFER_CMD: u8 = 1;

pub struct Is31fl3730<I2C> {
    i2c: I2C,
    address: u8,
    brightness: u8,
    buf_matrix_left: [u8; BUFFER_LENGTH as usize + BUFFER_CMD as usize],
    buf_matrix_right: [u8; BUFFER_LENGTH as usize + BUFFER_CMD as usize],
}

impl<I2C, E> Is31fl3730<I2C>
where
    I2C: Write<Error = E>,
{
    pub fn new(i2c: I2C, address: u8) -> Self {
        Is31fl3730 {
            i2c,
            address,
            brightness: 0,
            buf_matrix_left: [CMD_MATRIX_L, 0, 0, 0, 0, 0, 0, 0, 0],
            buf_matrix_right: [CMD_MATRIX_R, 0, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    pub fn init(&mut self) -> Result<(), E> {
        self.set_brightness(DEFAULT_BRIGHTNESS, true)?;
        self.clear()?;
        Ok(())
    }

    pub fn send_cmd(&mut self, cmd: u8, data: u8) -> Result<(), E> {
        self.i2c.write(self.address, &[cmd, data])
    }

    pub fn set_brightness(&mut self, brightness: u8, update: bool) -> Result<(), E> {
        self.brightness = if brightness < MAX_BRIGHTNESS {
            brightness
        } else {
            MAX_BRIGHTNESS
        };

        if update {
            self.send_cmd(CMD_BRIGHTNESS, self.brightness)?;
        }
        Ok(())
    }

    pub fn set_decimal(&mut self, left: bool, right: bool) -> Result<(), E> {
        if left {
            self.buf_matrix_left[(7 + BUFFER_CMD) as usize] |= 0b01000000;
        } else {
            self.buf_matrix_left[(7 + BUFFER_CMD) as usize] &= 0b10111111;
        }

        if right {
            self.buf_matrix_right[(6 + BUFFER_CMD) as usize] |= 0b10000000;
        } else {
            self.buf_matrix_right[(6 + BUFFER_CMD) as usize] &= 0b01111111;
        }
        Ok(())
    }

    pub fn set_pixel(&mut self, x: u8, y: u8, c: bool) -> Result<(), E> {
        // Left Matrix
        if x < 5 {
            if c {
                self.buf_matrix_left[(x + BUFFER_CMD) as usize] |= 1 << y;
            } else {
                self.buf_matrix_left[(x + BUFFER_CMD) as usize] &= !(1 << y);
            }
        }
        // Right Matrix
        else {
            let x = x - 5;
            if c {
                self.buf_matrix_right[(y + BUFFER_CMD) as usize] |= 1 << x;
            } else {
                self.buf_matrix_right[(y + BUFFER_CMD) as usize] &= !(1 << x);
            }
        }
        Ok(())
    }

    pub fn set_character(&mut self, x: u8, c: char) -> Result<(), E> {
        let s = DOT_FONT[0].ch;
        let e = DOT_FONT[DOT_FONT.len() - 1].ch;
        if c as u8 >= s && c as u8 <= e {
            let index = (c as u8 - s) as usize;
            let data = DOT_FONT[index].data;
            for cx in 0..DOT_CHAR_WIDTH {
                for cy in 0..HEIGHT {
                    let bit = data[cx as usize] & (1 << cy);
                    self.set_pixel(x + cx, cy, bit != 0)?;
                }
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), E> {
        for i in 0..BUFFER_LENGTH {
            self.buf_matrix_left[(i + BUFFER_CMD) as usize] = 0x00;
            self.buf_matrix_right[(i + BUFFER_CMD) as usize] = 0x00;
        }
        Ok(())
    }

    pub fn show(&mut self) -> Result<(), E> {
        self.i2c.write(self.address, &self.buf_matrix_left)?;
        self.i2c.write(self.address, &self.buf_matrix_right)?;

        self.send_cmd(CMD_MODE, MODE)?;
        self.send_cmd(CMD_OPTIONS, OPTS)?;
        self.send_cmd(CMD_BRIGHTNESS, self.brightness)?;
        self.send_cmd(CMD_UPDATE, 0x01)?;
        Ok(())
    }
}
