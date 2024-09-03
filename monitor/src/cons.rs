// Copyright 2024  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use uart::arch::Uart;

pub(crate) type Error = ();
pub(crate) type Result<T> = core::result::Result<T, Error>;

fn readline<'a>(uart: &mut Uart, prompt: &str, line: &'a mut [u8]) -> Result<&'a [u8]> {
    const BS: u8 = 8;
    const TAB: u8 = 9;
    const NL: u8 = 10;
    const CR: u8 = 13;
    const CTLU: u8 = 21;
    const CTLW: u8 = 23;
    const DEL: u8 = 127;

    fn find_prev_col(line: &[u8], start: usize) -> usize {
        line.iter().fold(start, |v, &b| v + if b == TAB { 8 - (v & 0b111) } else { 1 })
    }

    fn backspace(uart: &mut Uart, line: &[u8], start: usize, col: usize) -> (usize, usize) {
        if line.is_empty() {
            return (start, 0);
        }
        let (pcol, overstrike) = match line.last() {
            Some(&b' ') => (col - 1, false),
            Some(&b'\t') => (find_prev_col(&line[..line.len() - 1], start), false),
            _ => (col - 1, true),
        };
        for _ in pcol..col {
            uart.putb(BS);
            if overstrike {
                uart.putb(b' ');
                uart.putb(BS);
            }
        }
        (pcol, line.len() - 1)
    }

    fn isword(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    if line.is_empty() {
        return Ok(line);
    }

    let start = prompt.len();
    let mut k = 0;
    let mut col = start;

    uart.puts(prompt);
    while k < line.len() {
        match uart.getb() {
            CR | NL => {
                uart.putb(CR);
                uart.putb(NL);
                break;
            }
            BS | DEL => {
                if k > 0 {
                    (col, k) = backspace(uart, &line[..k], start, col);
                }
            }
            CTLU => {
                while k > 0 {
                    (col, k) = backspace(uart, &line[..k], start, col);
                }
            }
            CTLW => {
                while k > 0 && line[k - 1].is_ascii_whitespace() {
                    (col, k) = backspace(uart, &line[..k], start, col);
                }
                if k > 0 {
                    let cond = isword(line[k - 1]);
                    while k > 0 && !line[k - 1].is_ascii_whitespace() && isword(line[k - 1]) == cond
                    {
                        (col, k) = backspace(uart, &line[..k], start, col);
                    }
                }
            }
            TAB => {
                line[k] = TAB;
                k += 1;
                let ncol = (8 + col) & !0b111;
                for _ in col..ncol {
                    uart.putb(b' ');
                }
                col = ncol;
            }
            b => {
                line[k] = b;
                k += 1;
                uart.putb(b);
                col += 1;
            }
        }
    }

    Ok(&line[..k])
}

pub(crate) fn repl() {
    let mut uart = Uart::new(uart::arch::Port::Eia0);
    let mut buf = [0u8; 1024];
    loop {
        if let Ok(line) = readline(&mut uart, "@", &mut buf) {
            if line.is_empty() {
                break;
            }
            for &b in line.iter() {
                uart.putb(b);
            }
            uart.putb(b'\r');
            uart.putb(b'\n');
        }
    }
}
