//! lens' engine (codename: NEAT - NES Execution abstraction)

#[derive(Clone, Copy, Debug)]
struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pc: u16,
    p: u8, // private to prevent mid-frame external writes WHICH WILL FUCKING RUIN EVERYTHING
    haltC: u8,
    haltF: bool,
}

enum regs {
    a,
    x,
    y,
    sp,
    pc,
    p
}

enum flags {
    carry = 1,
    zero = 2,
    int_dis = 4,
    decimal = 8, // IGNORED
    b = 16,
    none = 32,
    overflow = 64,
    negative = 128,
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
#[derive(Debug)]
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

pub struct App { // same
    cpu: Cpu, // Cpu state
    mem: Option<Rom>, // current ROM
    rew: Vec<State>, // automatic, in between frames
    snap: Vec<State>, // snapshots, user controlled
    bus: Bus,
}


impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            sp: 0, 
            pc: 0x8000, // most common entry point
            p: 0,
            haltC: 0,
            haltF: false,
        }
    }
    #[inline(always)]
    pub fn set_zn(&mut self, val: u8) {
        if val & 0x80 != 0 {
            self.p |= flags::negative as u8;
        } else {
            self.p &= !(flags::negative as u8);
        }
        if val == 0 {
            self.p |= flags::zero as u8;
        } else {
            self.p &= !(flags::zero as u8);
        }
    }
    #[inline(always)]
    pub fn set_zcvn(&mut self, val: u8, carry: bool, overflow: bool) {
        self.p &= !(flags::zero as u8 | flags::carry as u8 | flags::overflow as u8| flags::negative as u8);

        if val == 0 {
            self.p |= flags::zero as u8;
        }

        if carry {
            self.p |= flags::carry as u8;
        }

        if val & 0x80 != 0 {
            self.p |= flags::negative as u8;
        }

        if overflow {
            self.p |= flags::overflow as u8;
        }
    }
    #[inline(always)]
    pub fn pc(self) -> u16 {
        self.pc
    }
    #[inline(always)]
    pub fn ainc_pc(&mut self) -> u16 { // atomic increment and fetch
        self.pc+=1;

        self.pc-1
    }
    #[inline(always)]
    pub fn set_pc(&mut self, val: u16) {
        self.pc=val;
    }
    #[inline(always)]
    pub fn tax(&mut self) {
        self.x = self.a;
        self.set_zn(self.x);
    }
    #[inline(always)]
    pub fn txa(&mut self) {
        self.a = self.x;
        self.set_zn(self.a);
    }
    #[inline(always)]
    pub fn tay(&mut self) {
        self.y = self.a;
        self.set_zn(self.y);
    }
    #[inline(always)]
    pub fn tya(&mut self) {
        self.a = self.y;
        self.set_zn(self.a);
    }
    #[inline(always)]
    pub fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.set_zn(self.x);
    }
    #[inline(always)]
    pub fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.set_zn(self.y);
    }
    #[inline(always)]
    pub fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.set_zn(self.x);
    } 
    #[inline(always)]
    pub fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.set_zn(self.y);
    }
    #[inline(always)]
    pub fn lda(&mut self, val: u8) {
        self.a = val;
        self.set_zn(self.a);
    }
    #[inline(always)]
    pub fn ldx(&mut self, val: u8) {
        self.x = val;
        self.set_zn(self.x);
    }
    #[inline(always)]
    pub fn ldy(&mut self, val: u8) {
        self.y = val;
        self.set_zn(self.y);
    }
    #[inline(always)]
    pub fn txs(&mut self) {
        self.sp = self.x;
    }
    #[inline(always)]
    pub fn sei(&mut self) {
        self.p |= flags::int_dis as u8;
    }
    #[inline(always)]
    pub fn cld(&mut self) {
        self.p &= !(flags::decimal as u8);
    }
    #[inline(always)]
    pub fn clc(&mut self) {
        self.p &= !(flags::carry as u8);
    }

}

impl App {
    pub fn new() -> App {
        App {
            cpu: Cpu::new(),
            mem: None,
            rew: Vec::new(),
            snap: Vec::new(),
            bus: Bus::new()
        }
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.bus.ram[(addr & 0x07FF) as usize] = val, // wrap 2kb
            0x2000..=0x2007 => {println!("Write to PPU: val 0x{:02X} at 0x{:04X}", val, addr);}
            _ => {panic!("Attempted write to unmapped address 0x{:04X}", addr);} // EXTERNAL (NOT-RAM) : ignored ! todo : not ignore;
        }
    }
    pub fn push_stack(&mut self, val: u8) {
        self.write(0x0100 | self.cpu.sp as u16, val);
        self.cpu.sp = self.cpu.sp.wrapping_sub(1);
    }  
    pub fn pop_stack(&mut self) -> u8 {
        self.cpu.sp = self.cpu.sp.wrapping_add(1);
        self.read(0x0100 | self.cpu.sp as u16)
    }
    pub fn read(&mut self, addr: u16) -> u8 {
        return match addr {
            0x0000..=0x1FFF => self.bus.ram[(addr&0x07FF) as usize],
            0x2002 => 0x80, // PPU status stub ! todo: implement actual regs (mirrored each 8 bytes)
                            // from 0x2002 to 0x3FFF & not suck
            _ => 0 // EXTERNAL (NOT-RAM) : ignored ! todo : not ignore; (yup)
        };
    }
    pub fn start_addr(&self) -> Result<u16, u16> {
        if self.mem.is_none() {return Err(0xBEEF);}
        let rom = self.mem.as_ref().unwrap();
        let offset: u16 = (rom.qprg_pos + (if rom.prg_size == 1 { 0x3FFC } else { 0x7FFC })).try_into().unwrap();

        Ok(u16::from_le_bytes([
                rom.mem[offset as usize],
                rom.mem[offset as usize + 1],
        ]))
    }
    pub fn fetch(&mut self) -> u8 {
        let pc = self.cpu.ainc_pc();


        match pc {
            0x0000..=0x1FFF => {
                self.bus.ram[(pc & 0x07FF) as usize]
            }

            0x8000..=0xFFFF => {
                let rom = self.mem.as_ref()
                    .expect("CPU fetch attempted without loaded ROM");

                let prg_size = rom.mem[4] as usize * 0x4000;
                let mut index = (pc - 0x8000) as usize;

                if prg_size == 0x4000 {
                    index &= 0x3FFF;
                }

                rom.mem[16 + index]
            }

            _ => {
                panic!("Unmapped CPU address: {pc:#06X}");
            }
        }
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
        let _mapper_ = (flag6 >> 4) | (flag7 & 0xF0);
        let prg_ram: u8 = data[8];
        let tv_stat: u8 = data[9];
        let unnof_m: u8 = data[10];
        let mapper_: u8 = (flag6 >> 4) | (flag7 & 0xF0);        
        if mapper_ != 0 {
            println!("No mapper: {}", mapper_);
            return Err(RomErr::ENoMapper);
        }
        let has_trainer = (flag6 & 0x04) != 0;
        let mut prg_pos = 16;
        if has_trainer {
            prg_pos += 512;
        }
        println!("Has trainer: {}", has_trainer);
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
    pub fn init(&mut self) {
        self.set_pc(self.start_addr().unwrap());
        println!("start address: {}", self.start_addr().unwrap());
    }
    pub fn set_zn(&mut self, val: u8) {
        if val & 0x80 != 0 {
            self.cpu.p |= flags::negative as u8;
        } else {
            self.cpu.p &= !(flags::negative as u8);
        }
        if val == 0 {
            self.cpu.p |= flags::zero as u8;
        } else {
            self.cpu.p &= !(flags::zero as u8);
        }
    }
    pub fn step(&mut self) {
        // NEED FETCH
        // fetch done
        // NEED DEC
        // implement inplace
        if self.cpu.haltC != 0 {
            self.cpu.haltC -= 1;
            return;
        }
        if self.cpu.haltF {
            return;
        }
        let mut executed = false;
        let op = self.fetch();

        while !executed {
            match op {
                0xEA => (), // nop
                0xAA => self.cpu.tax(),// TAX transfer A to X
                0xA8 => self.cpu.tay(),
                0x8A => self.cpu.txa(),
                0x98 => self.cpu.tya(),
                0xE8 => self.cpu.inx(),
                0xC8 => self.cpu.iny(),
                0xCA => self.cpu.dex(),
                0x88 => self.cpu.dey(),
                0x9A => self.cpu.txs(),
                0x78 => self.cpu.sei(), // SEI
                0xD8 => self.cpu.cld(), // CLD
                0x18 => self.cpu.clc(),
                0xA9 => {let val = self.fetch(); self.cpu.lda(val)}, // unsafe but improbable to happen
                0xA2 => {let val = self.fetch(); self.cpu.ldx(val)},
                0xA0 => {let val = self.fetch(); self.cpu.ldy(val)},
                0x84 => { // sty (store y)
                    let addr = self.fetch() as u16;

                    self.write(addr, self.cpu.y);
                },
                0x86 => {
                    let addr = self.fetch() as u16;
                    self.write(addr, self.cpu.x);
                },
                0xAD => { // LDA
                    let le = self.fetch() as u16;
                    let he = self.fetch() as u16;
                    let val = self.read((he << 8) | le);
                    self.cpu.a = val;
                    self.set_zn(val);
                },
                0xBD => { // LDA abs x
                    let le = self.fetch() as u16;
                    let he = self.fetch() as u16;
                    let addr = (((he << 8) | le) as u32 + self.cpu.x as u32) as u16;
                    let val = self.read(addr);
                    self.cpu.a = val;
                    self.set_zn(val);
                },
                0x85 => { // STA zp
                    let addr = self.fetch() as u16;
                    self.write(addr, self.cpu.a);

                },
                0x8D => { // STA
                    let le = self.fetch() as u16;
                    let he = self.fetch() as u16;
                    let addr = (he << 8) | le;
                    self.write(addr, self.cpu.a);
                },
                0x91 => { // STA IND Y
                    let ptr = self.fetch();
                    let le = self.read(ptr as u16) as u16;
                    let he = self.read(ptr.wrapping_add(1) as u16) as u16;

                    let base = (he << 8) | le;
                    let addr = base.wrapping_add(self.cpu.y as u16);

                    self.write(addr, self.cpu.a);
                }
                // compare
                0xC9 => { // cmp immediate against a
                    self.cpu.p &= !(flags::zero as u8 | flags::carry as u8 | flags::negative as u8);
                    let val = self.fetch();
                    let result = self.cpu.a.wrapping_sub(val);
                    if result == 0 {
                        self.cpu.p |= flags::zero as u8;
                    }
                    if self.cpu.a >= val {
                        self.cpu.p |= flags::carry as u8;
                    }
                    self.cpu.p |= result & 0x80; 
                },
                0xE0 => { // cmp x immediate
                    self.cpu.p &= !(flags::zero as u8 | flags::carry as u8 | flags::negative as u8);
                    let value = self.fetch();
                    let result = self.cpu.x.wrapping_sub(value);
                    self.cpu.p |= if result == 0 { flags::zero as u8 } else { 0 };
                    self.cpu.p |= if result & 0x80 != 0 { flags::negative as u8 } else { 0 };
                    self.cpu.p |= if self.cpu.x >= value { flags::carry as u8 } else { 0 };
                },
                0xC0 => { // cmp y immediate
                    self.cpu.p &= !(flags::zero as u8 | flags::carry as u8 | flags::negative as u8);
                    let value = self.fetch();
                    let result = self.cpu.y.wrapping_sub(value);
                    self.cpu.p |= if result == 0 { flags::zero as u8 } else { 0 };
                    self.cpu.p |= if result & 0x80 != 0 { flags::negative as u8 } else { 0 };
                    self.cpu.p |= if self.cpu.y >= value { flags::carry as u8 } else { 0 };
                },

                // unnoficials 
                0xFB => { // lol facebook | isc
                    let le = self.fetch() as u16;
                    let he = self.fetch() as u16;
                    let base = (he << 8) | le;
                    let addr = base.wrapping_add(self.cpu.y as u16);
                    let x = self.read(addr).wrapping_add(1);
                    self.write(addr, x);
                    let old = self.cpu.a;
                    let carry = (self.cpu.p & flags::carry as u8) != 0;
                    let result = (old as u16)
                        .wrapping_sub(x as u16)
                        .wrapping_sub((!carry) as u16);
                    let result_8b = result as u8;
                    let no_borrow = (old as u16) >= (x as u16 + (!carry as u16));
                    let overflow = ((old ^ result_8b) & (old ^ x) & 0x80) != 0;
                    self.cpu.a = result_8b;
                    self.cpu.set_zcvn(result_8b, no_borrow, overflow);
                },
                // strangest of em all, immediate nop
                0x04 | 0x82 => {
                    let _ = self.fetch();
                },
                // another one
                0x3A => {},
                // branches
                0x10 => { // bpl
                          // branch if positive
                    if self.cpu.p & flags::negative as u8 == 0 { // not negative
                        let x = self.fetch() as i8;
                        self.cpu.pc = self.cpu.pc.wrapping_add_signed(x as i16); 
                    } else {
                        self.fetch();
                    }
                },
                0xB0 => { // branch if carry set
                    if self.cpu.p & flags::carry as u8 != 0 {
                        let x = self.fetch() as i8;
                        self.cpu.pc = self.cpu.pc.wrapping_add_signed(x as i16);
                    } else {
                        self.fetch();
                    }
                }
                0xD0 => { // branch if not equal
                    if self.cpu.p & flags::zero as u8 == 0 {
                        let x = self.fetch() as i8;
                        self.cpu.pc = self.cpu.pc.wrapping_add_signed(x as i16);
                    } else {
                        self.fetch();
                    }
                }
                // jumps
                0x20 => {
                    let le = self.fetch() as u16;
                    let he = self.fetch() as u16;
                    let addr = (he << 8) | le; // high byte first, aka high endian
                    let ret_addr = self.cpu.pc.wrapping_sub(1); // RTS will add 1 back, 6502 quirk

                    self.push_stack((ret_addr >> 8) as u8);
                    self.push_stack((ret_addr & 0x00FF) as u8);

                    self.cpu.pc = addr;
                }
                0x60 => {
                    let le = self.pop_stack() as u16;
                    let he = self.pop_stack() as u16;

                    self.cpu.pc = ((he << 8) | le).wrapping_add(1);
                }
                _ => panic!("Unimplemented opcode: 0x{:02X} at 0x{:04X}", op, self.cpu.pc -1),
            }
            executed = true;
        }
    }
    pub fn set_pc(&mut self, val: u16) {
        self.cpu.set_pc(val);
    }
}
