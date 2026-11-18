//! lens' engine (codename: NEAT - NES Execution abstraction)

#[derive(Clone, Copy, Debug)]
struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pc: u16,
    p: u8, // private to prevent mid-frame external writes WHICH WILL FUCKING RUIN EVERYTHING
}

enum regs {
    a,
    x,
    y,
    sp,
    pc,
    p
}



pub const MAX_TILES: u64 = 512;
pub const MAX_REWIND: u64 = 2048; // len of rewind buffer

struct Rom { // made everything public, fuck Deref
    pub prg_size: usize, // * 16kb
    pub chr_size: usize, // == 0 : chr ram | * 8kb 
    pub prg_ram: usize, // == 0 : =1 | * 8kb 
    pub qchr_pos: usize,
    pub qprg_pos: usize,
    pub flag6: u8,
    pub flag7: u8,
    pub tv: u8,
    pub unnof: u8,
    pub mem: Vec<u8>, 
    // tiles: Vec<u8>, DEPRECATED
}

impl Rom {
    pub fn fetch(self, pc: usize) -> u8 {
        self.mem[pc]
    }
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
    WIP
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
    pub fn pc(self) -> u16 {
        self.pc
    }
    pub fn ainc_pc(&mut self) -> u16 { // atomic increment and fetch
        self.pc+=1;
        self.pc-1
    }
    pub fn tax(&mut self) {
        self.x = self.a;
    }
    pub fn txa(&mut self) {
        self.a = self.x;
    }
    pub fn tay(&mut self) {
        self.y = self.a;
    }
    pub fn tya(&mut self) {
        self.a = self.y;
    }
    pub fn inx(&mut self) {
        self.x += 1;
    }
    pub fn iny(&mut self) {
        self.y += 1;
    }
    pub fn dex(&mut self) {
        self.x -= 1;
    } 
    pub fn dey(&mut self) {
        self.y -= 1;
    }
    pub fn lda(&mut self, val: u8) {
        self.a = val;
    }
    pub fn ldx(&mut self, val: u8) {
        self.x = val;
    }
    pub fn ldy(&mut self, val: u8) {
        self.y = val;
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
    pub fn fetch(&mut self) -> u8 {
        let pc = self.cpu.ainc_pc();
        self.mem.as_ref().expect("Tried to run without a rom!").mem[pc as usize]
    }
    pub fn load_rom(&mut self, data: Vec<u8>) -> Result<(), RomErr> {
        if data.len() < 16 || &data[0..4] != b"NES\x1A" {
            return Err(RomErr::EInvalid);
        }
        // read flags and some header data
        let prg_rom: u8 = data[4];
        if prg_rom == 0 {
            return Err(RomErr::EBadRom);
        }
        let chr_rom: u8 = data[5];
        let flag6:  u8 = data[6];
        let flag7: u8 = data[7];
        let mapper_ = (flag6 >> 4) | (flag7 & 0xF0);
        let prg_ram: u8 = data[8];
        let tv_stat: u8 = data[9];
        let unnof_m: u8 = data[10];
        let mapper_: u8 = (flag6 >> 4) | (flag7 & 0xF0);        
        if mapper_ != 0 {
            return Err(RomErr::ENoMapper);
        }
        let has_trainer = (flag6 & 0x04) != 0;
        let mut prg_pos = 16;
        if has_trainer {
            prg_pos += 512;
        }
        let mut chr_pos = 0;
        if chr_rom != 0 {
            chr_pos = prg_pos + (prg_rom as usize *1024*16);
        }
        self.mem = Some(Rom {
            prg_size: prg_rom as usize,
            chr_size: chr_rom as usize,
            prg_ram: prg_ram as usize,
            qchr_pos: chr_pos as usize,
            qprg_pos: prg_pos as usize,
            flag6: data[6],
            flag7: data[7],
            tv: tv_stat,
            unnof: unnof_m,
            mem: data,
            // tiles: Vec::new() DEPRECATED
        });
        Ok(())
    }
    pub fn step(&mut self, bus: &mut Bus) {
        // NEED FETCH
        // fetch done
        // NEED DEC
        // implement inplace
        let mut executed = false;
        while !executed {
            match self.fetch() {
                0xEA => (), // nop
                0xAA => self.cpu.tax(),// TAX transfer A to X
                0xA8 => self.cpu.tay(),
                0x8A => self.cpu.txa(),
                0x98 => self.cpu.tya(),
                0xE8 => self.cpu.inx(),
                0xC8 => self.cpu.iny(),
                0xCA => self.cpu.dex(),
                0x88 => self.cpu.dey(),
                0xA9 => {let val = self.fetch(); self.cpu.lda(val)}, // unsafe but improbable to happen
                0xA2 => {let val = self.fetch(); self.cpu.ldx(val)},
                0xA0 => {let val = self.fetch(); self.cpu.ldy(val)},
                _ => panic!("Unimplemented opcode: 0x{:02X}", self.cpu.pc),
            }
            executed = true;
        }
    }
}
