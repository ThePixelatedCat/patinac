use crate::instructions::Instr;
use std::{os::raw, sync::mpsc::Sender};

pub fn decode(instructions: Vec<u32>, tx: Sender<Instr>) {
    for raw_instr in instructions {
        let op_code = (raw_instr & 0b111111) as u8;

        let instr = match op_code {
            0b_001001 => decode_add(raw_instr),
            _ => unreachable!(),
        };

        let _ = tx.send(instr);
    }
}

fn decode_add(raw_instr: u32) -> Instr {
    let instr_bytes = raw_instr.to_be_bytes();

    let rhs = instr_bytes[0] & 0b_11111;
    let lhs = instr_bytes[1] & 0b_11111;
    let dst = instr_bytes[2] & 0b_11111;

    Instr::Add { dst, lhs, rhs }
}
