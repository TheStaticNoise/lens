//! lens' engine (codename: NEAT - NES Execution abstraction)

#[derive(Clone, Copy, Debug)]
struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pc: u16,
    p: u8, // private to prevent mid-frame writes WHICH WILL FUCKING RUIN EVERYTHING
}

pub const MAX_TILES: u64 = 512;
pub const MAX_REWIND: u64 = 2048; // len of rewind buffer

struct Rom {
    prg_size: usize, // * 16kb
    chr_rom: usize, // == 0 : chr ram | * 8kb  
    mem: Vec<u8>,
    tiles: Vec<u8>, 
}

#[derive(Clone, Copy, Debug)]
pub struct Bus {
    pub ram: [u8; 2048],
    // todo: add ppu, apu, mapper
}
#[derive(Clone, Copy, Debug)]
struct State {
    cpu: Cpu,
    bus: Bus
}

pub struct App { // same
    cpu: Cpu, // Cpu state
    mem: Option<Rom>, // current ROM
    rew: Vec<State>, // automatic, in between frames
    snap: Vec<State>, // snapshots, user controlled
}

pub enum RomErr {
    EInvalid,
    EBadRom,
    ENoMapper,
}

impl Bus {
    pub fn new() -> Bus {
        Bus {
            ram: [0; 2048]
        }
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            sp: 0, 
            pc: 0,
            p: 0,
        }
    }
}

impl App {
    pub fn new() -> App {
        App {
            cpu: Cpu::new(),
            mem: None,
            rew: Vec::new(),
            snap: Vec::new(),
        }
    }
    pub fn fetch(self) {
        
    }
    pub fn load_rom(mem: vec<u8>) {
        if mem.len() < 16 || &mem[0..4] != b"NES\x1A" {
            return Err(RomErr::EInvalid);
        }
    }

    }
    pub fn step(&mut self, bus: &mut Bus)
    }
}
