//cargo build --release
//avrdude -p m328p -c avrisp2 -P usb -U flash:w:target/avr-atmega328p/release/my-rpn-calc.elf:e

#![no_std]
#![no_main]

use core::fmt::Write;
use core::panic::PanicInfo;
use arduino_hal::I2c;
use embedded_hal::{prelude::_embedded_hal_blocking_i2c_Write, digital::v2::OutputPin};
use ufmt::uWrite;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    let mut w = MyWrite {};
    clear_lcd();
    write!(&mut w, "panic").unwrap();
    loop {}
}

static mut I2C_GLOBAL: Option<I2c> = None;
static mut FONT_GLOBAL: [[u8; 5]; 41] = [[0u8; 5]; 41];

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();

    let pins = arduino_hal::pins!(dp);
    let mut col0 = pins.d0.into_output();
    let mut col1 = pins.d1.into_output();
    let mut col2 = pins.d2.into_output();
    let mut col3 = pins.d3.into_output();

    let row0 = pins.a0.into_pull_up_input();
    let row1 = pins.a1.into_pull_up_input();
    let row2 = pins.a2.into_pull_up_input();
    let row3 = pins.a3.into_pull_up_input();

    let mut i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        800000,
    );

    i2c.write(0x3C, &[0b00000000, 0xAE]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xA8, 31]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xD3, 0]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0x40]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xA1]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xC8]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xDA, 0x02]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0x81, 0x7F]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xA4]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xA6]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0xD5, 0x80]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0x8D, 0x14]).unwrap();
    i2c.write(0x3C, &[0b00000000, 0x20, 0x0]).unwrap();

    unsafe {
        I2C_GLOBAL = Some(i2c);
        FONT_GLOBAL = make_font();
    };
    clear_lcd();

    reset_pos_lcd();
    let mut w = MyWrite {};

    let mut col_out = |b:u8| {
        col0.set_state((b&0b0001 != 0).into()).unwrap();
        col1.set_state((b&0b0010 != 0).into()).unwrap();
        col2.set_state((b&0b0100 != 0).into()).unwrap();
        col3.set_state((b&0b1000 != 0).into()).unwrap();
    };

    let row_in = ||->u8 {
        let mut b = 0u8;
        if row0.is_low() { b |= 0b0001; }
        if row1.is_low() { b |= 0b0010; }
        if row2.is_low() { b |= 0b0100; }
        if row3.is_low() { b |= 0b1000; }
        b
    };

    let mut scan_key = || -> Option<char> {
        col_out(0b1110);
        arduino_hal::delay_us(5);
        let b = row_in();
        if b&0b0001 != 0 {return Some('0');}
        if b&0b0010 != 0 {return Some('1');}
        if b&0b0100 != 0 {return Some('4');}
        if b&0b1000 != 0 {return Some('7');}

        col_out(0b1101);
        arduino_hal::delay_us(5);
        let b = row_in();
        if b&0b0001 != 0 {return Some('.');}
        if b&0b0010 != 0 {return Some('2');}
        if b&0b0100 != 0 {return Some('5');}
        if b&0b1000 != 0 {return Some('8');}

        col_out(0b1011);
        arduino_hal::delay_us(5);
        let b = row_in();
        if b&0b0001 != 0 {return Some('e');}
        if b&0b0010 != 0 {return Some('3');}
        if b&0b0100 != 0 {return Some('6');}
        if b&0b1000 != 0 {return Some('9');}

        col_out(0b0111);
        arduino_hal::delay_us(5);
        let b = row_in();
        if b&0b0001 != 0 {return Some('p');}
        if b&0b0010 != 0 {return Some('m');}
        if b&0b0100 != 0 {return Some('x');}
        if b&0b1000 != 0 {return Some('d');}

        None
    };

    let mut get_key = || {
        loop {
            let b = scan_key();
            if let Some(x) = b {
                while scan_key().is_some() {}
                return x;
            }
        }
    };

    write!(&mut w, "!").unwrap();
    let mut num:i64 = 0;
    loop {
        let c = get_key();
        if c.is_numeric() {
            if let Some(n) = c.to_digit(10) {
                num *= 10;
                num += i64::from(n);
            }
        }
        if c == 'e' {
            num = 0;
        }
        //to stringが使えない
        clear_lcd();
        ufmt::uwrite!(&mut w, "{}",num).unwrap();
    }
}

fn clear_lcd() {
    unsafe {
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0xAE])
            .unwrap();
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0x21, 0, 127])
            .unwrap();
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0x22, 0, 4])
            .unwrap();
        for _ in 0..(128 * 5 / 16) {
            let mut data = [0x00u8; 17];
            data[0] = 0b01000000;
            I2C_GLOBAL.as_mut().unwrap().write(0x3C, &data).unwrap();
        }
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0xAF])
            .unwrap();
    }
}

fn reset_pos_lcd() {
    unsafe {
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0x21, 0, 127])
            .unwrap();
        I2C_GLOBAL
            .as_mut()
            .unwrap()
            .write(0x3C, &[0b00000000, 0x22, 0, 4])
            .unwrap();
    }
}

struct MyWrite;
impl Write for MyWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            let c = c.to_ascii_uppercase();
            let b = c as u8;
            match b {
                0x30..=0x39 => put_font(usize::from(b) - 0x30),
                0x41..=0x5A => put_font(usize::from(b) + 10 - 0x41),
                0x21 => put_font(36),
                0x3F => put_font(37),
                0x2E => put_font(38),
                0x2A => put_font(39),
                0x23 => put_font(40),
                0x20 => put_font(255),
                _ => put_font(254),
            }
        }
        Ok(())
    }
}
impl ufmt::uWrite for MyWrite {
    fn write_str(&mut self, s: &str) ->  Result<(), Self::Error>{
        for c in s.chars() {
            let c = c.to_ascii_uppercase();
            let b = c as u8;
            match b {
                0x30..=0x39 => put_font(usize::from(b) - 0x30),
                0x41..=0x5A => put_font(usize::from(b) + 10 - 0x41),
                0x21 => put_font(36),
                0x3F => put_font(37),
                0x2E => put_font(38),
                0x2A => put_font(39),
                0x23 => put_font(40),
                0x20 => put_font(255),
                _ => put_font(254),
            }
        }
        Ok(())
    }

    type Error = ();
}

fn put_font(c: usize) {
    let mut data = [0x00u8; 9];
    data[0] = 0b01000000;
    unsafe {
        if c == 0xFF {
            for i in 0..5 {
                data[2 + i] = 0x00;
            }
        } else if c > FONT_GLOBAL.len() {
            for i in 0..5 {
                data[2 + i] = 0xFF;
            }
        } else {
            for i in 0..5 {
                data[2 + i] = FONT_GLOBAL[c][i];
            }
        }
        I2C_GLOBAL.as_mut().unwrap().write(0x3C, &data).unwrap();
    }
}

fn make_font() -> [[u8; 5]; 41] {
    let mut font = [[0u8; 5]; 41];

    font[0][0] = 0b00111110;
    font[0][1] = 0b01110001;
    font[0][2] = 0b01001001;
    font[0][3] = 0b01000111;
    font[0][4] = 0b00111110;

    font[1][0] = 0b01000100;
    font[1][1] = 0b01000010;
    font[1][2] = 0b01111111;
    font[1][3] = 0b01000000;
    font[1][4] = 0b01000000;

    font[2][0] = 0b01000010;
    font[2][1] = 0b01100001;
    font[2][2] = 0b01010001;
    font[2][3] = 0b01001001;
    font[2][4] = 0b01000110;

    font[3][0] = 0b00100010;
    font[3][1] = 0b01000001;
    font[3][2] = 0b01001001;
    font[3][3] = 0b01001001;
    font[3][4] = 0b00110110;

    font[4][0] = 0b00011000;
    font[4][1] = 0b00010100;
    font[4][2] = 0b00010010;
    font[4][3] = 0b01111111;
    font[4][4] = 0b00010000;

    font[5][0] = 0b01001111;
    font[5][1] = 0b01001001;
    font[5][2] = 0b01001001;
    font[5][3] = 0b01001001;
    font[5][4] = 0b00110001;

    font[6][0] = 0b00111110;
    font[6][1] = 0b01001001;
    font[6][2] = 0b01001001;
    font[6][3] = 0b01001001;
    font[6][4] = 0b00110000;

    font[7][0] = 0b00000111;
    font[7][1] = 0b00000001;
    font[7][2] = 0b01100001;
    font[7][3] = 0b00011001;
    font[7][4] = 0b00000111;

    font[8][0] = 0b00110110;
    font[8][1] = 0b01001001;
    font[8][2] = 0b01001001;
    font[8][3] = 0b01001001;
    font[8][4] = 0b00110110;

    font[9][0] = 0b00000110;
    font[9][1] = 0b01001001;
    font[9][2] = 0b01001001;
    font[9][3] = 0b01001001;
    font[9][4] = 0b00110110;

    font[10][0] = 0b01111110;
    font[10][1] = 0b00001001;
    font[10][2] = 0b00001001;
    font[10][3] = 0b00001001;
    font[10][4] = 0b01111110;

    font[11][0] = 0b01111111;
    font[11][1] = 0b01001001;
    font[11][2] = 0b01001001;
    font[11][3] = 0b01001001;
    font[11][4] = 0b00110110;

    font[12][0] = 0b00111110;
    font[12][1] = 0b01000001;
    font[12][2] = 0b01000001;
    font[12][3] = 0b01000001;
    font[12][4] = 0b00100010;

    font[13][0] = 0b01111111;
    font[13][1] = 0b01000001;
    font[13][2] = 0b01000001;
    font[13][3] = 0b01000001;
    font[13][4] = 0b00111110;

    font[14][0] = 0b01111111;
    font[14][1] = 0b01001001;
    font[14][2] = 0b01001001;
    font[14][3] = 0b01001001;
    font[14][4] = 0b01001001;

    font[15][0] = 0b01111111;
    font[15][1] = 0b00001001;
    font[15][2] = 0b00001001;
    font[15][3] = 0b00001001;
    font[15][4] = 0b00001001;

    font[16][0] = 0b00111110;
    font[16][1] = 0b01000001;
    font[16][2] = 0b01001001;
    font[16][3] = 0b01001001;
    font[16][4] = 0b00111010;

    font[17][0] = 0b01111111;
    font[17][1] = 0b00001000;
    font[17][2] = 0b00001000;
    font[17][3] = 0b00001000;
    font[17][4] = 0b01111111;

    font[18][0] = 0b00000000;
    font[18][1] = 0b01000001;
    font[18][2] = 0b01111111;
    font[18][3] = 0b01000001;
    font[18][4] = 0b00000000;

    font[19][0] = 0b00100000;
    font[19][1] = 0b01000001;
    font[19][2] = 0b01000001;
    font[19][3] = 0b00111111;
    font[19][4] = 0b00000001;

    font[20][0] = 0b01111111;
    font[20][1] = 0b00001000;
    font[20][2] = 0b00010100;
    font[20][3] = 0b00100010;
    font[20][4] = 0b01000001;

    font[21][0] = 0b01111111;
    font[21][1] = 0b01000000;
    font[21][2] = 0b01000000;
    font[21][3] = 0b01000000;
    font[21][4] = 0b01000000;

    font[22][0] = 0b01111111;
    font[22][1] = 0b00000010;
    font[22][2] = 0b00001100;
    font[22][3] = 0b00000010;
    font[22][4] = 0b01111111;

    font[23][0] = 0b01111111;
    font[23][1] = 0b00000011;
    font[23][2] = 0b00001100;
    font[23][3] = 0b00110000;
    font[23][4] = 0b01111111;

    font[24][0] = 0b00111110;
    font[24][1] = 0b01000001;
    font[24][2] = 0b01000001;
    font[24][3] = 0b01000001;
    font[24][4] = 0b00111110;

    font[25][0] = 0b01111111;
    font[25][1] = 0b00001001;
    font[25][2] = 0b00001001;
    font[25][3] = 0b00001001;
    font[25][4] = 0b00000110;

    font[26][0] = 0b00111110;
    font[26][1] = 0b01000001;
    font[26][2] = 0b01010001;
    font[26][3] = 0b00100001;
    font[26][4] = 0b01011110;

    font[27][0] = 0b01111111;
    font[27][1] = 0b00001001;
    font[27][2] = 0b00011001;
    font[27][3] = 0b00101001;
    font[27][4] = 0b01000110;

    font[28][0] = 0b00000110;
    font[28][1] = 0b01001001;
    font[28][2] = 0b01001001;
    font[28][3] = 0b01001001;
    font[28][4] = 0b00110000;

    font[29][0] = 0b00000001;
    font[29][1] = 0b00000001;
    font[29][2] = 0b01111111;
    font[29][3] = 0b00000001;
    font[29][4] = 0b00000001;

    font[30][0] = 0b00111111;
    font[30][1] = 0b01000000;
    font[30][2] = 0b01000000;
    font[30][3] = 0b01000000;
    font[30][4] = 0b00111111;

    font[31][0] = 0b00000111;
    font[31][1] = 0b00111000;
    font[31][2] = 0b01000000;
    font[31][3] = 0b00111000;
    font[31][4] = 0b00000111;

    font[32][0] = 0b00011111;
    font[32][1] = 0b01100000;
    font[32][2] = 0b00011000;
    font[32][3] = 0b01100000;
    font[32][4] = 0b00011111;

    font[33][0] = 0b01100011;
    font[33][1] = 0b00011100;
    font[33][2] = 0b00001000;
    font[33][3] = 0b00011100;
    font[33][4] = 0b01100011;

    font[34][0] = 0b00000011;
    font[34][1] = 0b00001100;
    font[34][2] = 0b01110000;
    font[34][3] = 0b00001100;
    font[34][4] = 0b00000011;

    font[35][0] = 0b01100001;
    font[35][1] = 0b01010001;
    font[35][2] = 0b01001001;
    font[35][3] = 0b01000101;
    font[35][4] = 0b01000011;

    font[36][0] = 0b00000000;
    font[36][1] = 0b00000000;
    font[36][2] = 0b01011111;
    font[36][3] = 0b00000000;
    font[36][4] = 0b00000000;

    font[37][0] = 0b00000110;
    font[37][1] = 0b00000001;
    font[37][2] = 0b01010001;
    font[37][3] = 0b00001001;
    font[37][4] = 0b00000110;

    font[38][0] = 0b00000000;
    font[38][1] = 0b00000000;
    font[38][2] = 0b01000000;
    font[38][3] = 0b00000000;
    font[38][4] = 0b00000000;

    font[39][0] = 0b00100010;
    font[39][1] = 0b00010100;
    font[39][2] = 0b01111111;
    font[39][3] = 0b00010100;
    font[39][4] = 0b00100010;

    font[40][0] = 0b00010100;
    font[40][1] = 0b01111111;
    font[40][2] = 0b00010100;
    font[40][3] = 0b01111111;
    font[40][4] = 0b00010100;

    font
}
