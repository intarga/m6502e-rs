#[derive(Default)]
pub struct CpuState {
    // registers
    a: u8,
    x: u8,
    y: u8,
    pch: u8,
    pcl: u8,
    // s: u8,

    // flags
    negative: bool,
    signed_overflow: bool,
    // brk_interrupt: bool,
    decimal_mode: bool,
    // irq_interrupt_disable: bool,
    zero: bool,
    carry: bool,
}

pub struct SystemState {
    cpu_state: CpuState,
    memory: [u8; 0x10000],
}

impl Default for SystemState {
    fn default() -> Self {
        SystemState {
            cpu_state: CpuState::default(),
            memory: [0; 0x10000],
        }
    }
}

#[derive(Debug)]
enum AddressingMode {
    I,     // Immediate
    A,     // Absolute
    Zp,    // Zero Page
    Aix,   // Absolute Indexed X
    Aiy,   // Absolute Indexed Y
    Zpix,  // Zero Page Indexed X
    Zpiix, // Zero Page Indexed Indirect X
    Zpiiy, // Zero Page Indirect Indexed Y
}

// -- Helper functions --

fn get_byte_at_addr(sys: &mut SystemState, addr: u16) -> u8 {
    sys.memory[addr as usize]
}

fn cat_bytes(b1: u8, b2: u8) -> u16 {
    (u16::from(b1) << 8) | u16::from(b2)
}

fn get_immediate_byte(sys: &mut SystemState, offset: u16) -> u8 {
    let addr = cat_bytes(sys.cpu_state.pch, sys.cpu_state.pcl) + offset;
    get_byte_at_addr(sys, addr)
}

fn get_absolute_byte(sys: &mut SystemState) -> u8 {
    let addr_lo = get_immediate_byte(sys, 1);
    let addr_hi = get_immediate_byte(sys, 2);
    let addr = cat_bytes(addr_hi, addr_lo);

    get_byte_at_addr(sys, addr)
}

fn get_absolute_byte_indexed(sys: &mut SystemState, index: u8) -> (u8, bool) {
    let mut addr_lo = get_immediate_byte(sys, 1);
    let mut addr_hi = get_immediate_byte(sys, 2);

    let carry: bool;
    (addr_lo, carry) = addr_lo.overflowing_add(index);

    if carry {
        addr_hi = addr_hi
            .checked_add(1)
            .expect("Overflow indexing memory address")
    }

    let addr = cat_bytes(addr_hi, addr_lo);

    (get_byte_at_addr(sys, addr), carry)
}

fn get_zero_page_byte(sys: &mut SystemState) -> u8 {
    let addr = get_immediate_byte(sys, 1) as u16;
    get_byte_at_addr(sys, addr)
}

fn get_zero_page_byte_indexed(sys: &mut SystemState, index: u8) -> u8 {
    let addr = get_immediate_byte(sys, 1).wrapping_add(index) as u16;
    get_byte_at_addr(sys, addr)
}

fn get_zero_page_byte_indexed_indirect(sys: &mut SystemState, index: u8) -> u8 {
    let addr1 = get_immediate_byte(sys, 1).wrapping_add(index) as u16;

    let addr2_lo = get_byte_at_addr(sys, addr1);
    let addr2_hi = get_byte_at_addr(sys, (addr1 + 1) & 0xff);
    let addr2 = cat_bytes(addr2_hi, addr2_lo);

    get_byte_at_addr(sys, addr2)
}

fn get_zero_page_byte_indirect_indexed(sys: &mut SystemState, index: u8) -> (u8, bool) {
    //
    let addr1 = get_immediate_byte(sys, 1) as u16;

    let mut addr2_lo = get_byte_at_addr(sys, addr1);
    let mut addr2_hi = get_byte_at_addr(sys, (addr1 + 1) & 0xff);

    let carry: bool;
    (addr2_lo, carry) = addr2_lo.overflowing_add(index);

    if carry {
        addr2_hi = addr2_hi
            .checked_add(1)
            .expect("Overflow indexing memory address")
    }

    let addr2 = cat_bytes(addr2_hi, addr2_lo);

    (get_byte_at_addr(sys, addr2), carry)
}

fn increment_pc(sys: &mut SystemState, num: u8) {
    let carry: bool;
    (sys.cpu_state.pcl, carry) = sys.cpu_state.pcl.overflowing_add(num);

    if carry {
        sys.cpu_state.pch = sys
            .cpu_state
            .pch
            .checked_add(1)
            .expect("Overflow of program counter");
    }
}

fn negative_u8(num: u8) -> bool {
    (num >> 7) != 0
}

fn set_n_z(sys: &mut SystemState, result: u8) {
    sys.cpu_state.negative = negative_u8(result);
    sys.cpu_state.zero = result == 0;
}

fn bcd_add_digit(a: u8, b: u8, carry: bool) -> (u8, bool) {
    let sum = a + b + carry as u8;
    if sum > 9 {
        (sum - 10, true)
    } else {
        (sum, false)
    }
}

fn bcd_add(a: u8, b: u8) -> (u8, bool) {
    let (res_lo, carry_lo) = bcd_add_digit(a & 0x0f, b & 0x0f, false);
    let (res_hi, carry) = bcd_add_digit(a >> 4, b >> 4, carry_lo);

    ((res_hi << 4) | res_lo, carry)
}

// -- Instructions --

fn adc(sys: &mut SystemState, mode: AddressingMode) -> (u8, u8) {
    let (operand, length, cycles) = match mode {
        AddressingMode::I => (get_immediate_byte(sys, 1), 2, 2),
        AddressingMode::A => (get_absolute_byte(sys), 3, 4),
        AddressingMode::Zp => (get_zero_page_byte(sys), 2, 3),
        AddressingMode::Aix => {
            let (byte, page_cross) = get_absolute_byte_indexed(sys, sys.cpu_state.x);
            (byte, 3, 4 + page_cross as u8)
        }
        AddressingMode::Aiy => {
            let (byte, page_cross) = get_absolute_byte_indexed(sys, sys.cpu_state.y);
            (byte, 3, 4 + page_cross as u8)
        }
        AddressingMode::Zpix => (get_zero_page_byte_indexed(sys, sys.cpu_state.x), 2, 4),
        AddressingMode::Zpiix => (
            get_zero_page_byte_indexed_indirect(sys, sys.cpu_state.x),
            2,
            6,
        ),
        AddressingMode::Zpiiy => {
            let (byte, page_cross) = get_zero_page_byte_indirect_indexed(sys, sys.cpu_state.y);
            (byte, 2, 5 + page_cross as u8)
        } // _ => panic!("unsupported mode {:?} on instruction ADC", mode),
    };
    let negative_before = negative_u8(sys.cpu_state.a);
    let (carry1, carry2): (bool, bool);

    if sys.cpu_state.decimal_mode {
        // TODO: check that the inputs are valid decimal numbers?
        // not sure how the 6502 handles invalid inputs here
        (sys.cpu_state.a, carry1) = bcd_add(sys.cpu_state.a, operand);
        (sys.cpu_state.a, carry2) = bcd_add(sys.cpu_state.a, sys.cpu_state.carry as u8);
    } else {
        (sys.cpu_state.a, carry1) = sys.cpu_state.a.overflowing_add(operand);
        (sys.cpu_state.a, carry2) = sys.cpu_state.a.overflowing_add(sys.cpu_state.carry as u8);
    }

    sys.cpu_state.carry = carry1 || carry2;
    set_n_z(sys, sys.cpu_state.a);
    sys.cpu_state.signed_overflow = !negative_before && negative_u8(sys.cpu_state.a);

    (length, cycles)
}

fn and(sys: &mut SystemState, mode: AddressingMode) -> (u8, u8) {
    let (operand, length, cycles) = match mode {
        AddressingMode::I => (get_immediate_byte(sys, 1), 2, 2),
        AddressingMode::A => (get_absolute_byte(sys), 3, 4),
        AddressingMode::Zp => (get_zero_page_byte(sys), 2, 3),
        AddressingMode::Aix => {
            let (byte, page_cross) = get_absolute_byte_indexed(sys, sys.cpu_state.x);
            (byte, 3, 4 + page_cross as u8)
        }
        AddressingMode::Aiy => {
            let (byte, page_cross) = get_absolute_byte_indexed(sys, sys.cpu_state.y);
            (byte, 3, 4 + page_cross as u8)
        }
        AddressingMode::Zpix => (get_zero_page_byte_indexed(sys, sys.cpu_state.x), 2, 4),
        AddressingMode::Zpiix => (
            get_zero_page_byte_indexed_indirect(sys, sys.cpu_state.x),
            2,
            6,
        ),
        AddressingMode::Zpiiy => {
            let (byte, page_cross) = get_zero_page_byte_indirect_indexed(sys, sys.cpu_state.y);
            (byte, 2, 5 + page_cross as u8)
        } // _ => panic!("unsupported mode {:?} on instruction AND", mode),
    };

    sys.cpu_state.a &= operand;

    set_n_z(sys, sys.cpu_state.a);

    (length, cycles)
}

// -- Emulation zone --

pub fn emulate_op(sys: &mut SystemState) -> u8 {
    let opcode = get_immediate_byte(sys, 0);

    let (length, cyc) = match opcode {
        0x21 => and(sys, AddressingMode::Zpiix),
        0x25 => and(sys, AddressingMode::Zp),
        0x29 => and(sys, AddressingMode::I),
        0x2d => and(sys, AddressingMode::A),
        0x31 => and(sys, AddressingMode::Zpiiy),
        0x35 => and(sys, AddressingMode::Zpix),
        0x39 => and(sys, AddressingMode::Aiy),
        0x3d => and(sys, AddressingMode::Aix),

        0x61 => adc(sys, AddressingMode::Zpiix),
        0x65 => adc(sys, AddressingMode::Zp),
        0x69 => adc(sys, AddressingMode::I),
        0x6d => adc(sys, AddressingMode::A),
        0x71 => adc(sys, AddressingMode::Zpiiy),
        0x75 => adc(sys, AddressingMode::Zpix),
        0x79 => adc(sys, AddressingMode::Aiy),
        0x7d => adc(sys, AddressingMode::Aix),

        _ => panic!("unimplemented instruction {}", opcode),
    };

    increment_pc(sys, length);

    cyc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_add() {
        assert_eq!((0x98, true), bcd_add(0x99, 0x99));
    }
}
