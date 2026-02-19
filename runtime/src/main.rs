use anyhow::anyhow;
use std::{
    env, fs,
    sync::mpsc::{self, Receiver},
    thread,
};

use crate::instructions::Instr;

mod decode;
mod instructions;

#[derive(Default)]
struct Env {
    pc: u32,
    regs: [u64; 32],
    ret_addr: u32,
    stack_ptr: usize,
}

struct InstrBuf {
    rx: Receiver<Instr>,
    inner: Vec<Instr>,
}

impl InstrBuf {
    pub fn new(rx: Receiver<Instr>, cap: usize) -> Self {
        Self {
            rx,
            inner: Vec::with_capacity(cap),
        }
    }

    pub fn get(&mut self, idx: u32) -> Option<&Instr> {
        let idx = idx as usize;

        while self.inner.get(idx).is_none() {
            self.inner.push(self.rx.recv().ok()?)
        }

        self.inner.get(idx)
    }
}

fn main() -> anyhow::Result<()> {
    let Some(bin_path) = env::args().nth(1) else {
        return Err(anyhow!("missing binary path argument"));
    };

    let bin_src: Vec<u32> = fs::read(bin_path)?
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let (tx, rx) = mpsc::channel();
    let mut instructions = InstrBuf::new(rx, bin_src.len());

    let decoder = thread::spawn(move || decode::decode(bin_src, tx));

    Ok(())
}
