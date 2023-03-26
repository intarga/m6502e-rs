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
    Acc,   //Accumulator
}

// -- Helper functions --

fn get_byte_at_addr(sys: &mut SystemState, addr: u16) -> u8 {
    sys.memory[addr as usize]
}

fn set_byte_at_addr(sys: &mut SystemState, addr: u16, byte: u8) {
    sys.memory[addr as usize] = byte;
}

fn cat_bytes(b1: u8, b2: u8) -> u16 {
    (u16::from(b1) << 8) | u16::from(b2)
}

fn get_immediate_byte(sys: &mut SystemState, offset: u16) -> u8 {
    let addr = cat_bytes(sys.cpu_state.pch, sys.cpu_state.pcl) + offset;
    get_byte_at_addr(sys, addr)
}

fn get_absolute_addr(sys: &mut SystemState) -> u16 {
    let addr_lo = get_immediate_byte(sys, 1);
    let addr_hi = get_immediate_byte(sys, 2);
    cat_bytes(addr_hi, addr_lo)
}

fn get_absolute_byte(sys: &mut SystemState) -> u8 {
    let addr = get_absolute_addr(sys);
    get_byte_at_addr(sys, addr)
}

fn set_absolute_byte(sys: &mut SystemState, byte: u8) {
    let addr = get_absolute_addr(sys);
    set_byte_at_addr(sys, addr, byte)
}

fn get_absolute_addr_indexed(sys: &mut SystemState, index: u8) -> (u16, bool) {
    let mut addr_lo = get_immediate_byte(sys, 1);
    let mut addr_hi = get_immediate_byte(sys, 2);

    let carry: bool;
    (addr_lo, carry) = addr_lo.overflowing_add(index);

    if carry {
        addr_hi = addr_hi
            .checked_add(1)
            .expect("Overflow indexing memory address")
    }

    (cat_bytes(addr_hi, addr_lo), carry)
}

fn get_absolute_byte_indexed(sys: &mut SystemState, index: u8) -> (u8, bool) {
    let (addr, boundary_cross) = get_absolute_addr_indexed(sys, index);
    (get_byte_at_addr(sys, addr), boundary_cross)
}

fn set_absolute_byte_indexed(sys: &mut SystemState, index: u8, byte: u8) {
    let (addr, _) = get_absolute_addr_indexed(sys, index);
    set_byte_at_addr(sys, addr, byte)
}

fn get_zero_page_byte(sys: &mut SystemState) -> u8 {
    let addr = get_immediate_byte(sys, 1) as u16;
    get_byte_at_addr(sys, addr)
}

fn set_zero_page_byte(sys: &mut SystemState, byte: u8) {
    let addr = get_immediate_byte(sys, 1) as u16;
    set_byte_at_addr(sys, addr, byte)
}

fn get_zero_page_byte_indexed(sys: &mut SystemState, index: u8) -> u8 {
    let addr = get_immediate_byte(sys, 1).wrapping_add(index) as u16;
    get_byte_at_addr(sys, addr)
}

fn set_zero_page_byte_indexed(sys: &mut SystemState, index: u8, byte: u8) {
    let addr = get_immediate_byte(sys, 1).wrapping_add(index) as u16;
    set_byte_at_addr(sys, addr, byte)
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

fn increment_pc(sys: &mut SystemState, num: u8) -> bool {
    let carry: bool;
    (sys.cpu_state.pcl, carry) = sys.cpu_state.pcl.overflowing_add(num);

    if carry {
        sys.cpu_state.pch = sys
            .cpu_state
            .pch
            .checked_add(1)
            .expect("Overflow of program counter");
    }

    carry
}

fn decrement_pc(sys: &mut SystemState, num: u8) -> bool {
    let carry: bool;
    (sys.cpu_state.pcl, carry) = sys.cpu_state.pcl.overflowing_sub(num);

    if carry {
        sys.cpu_state.pch = sys
            .cpu_state
            .pch
            .checked_sub(1)
            .expect("Overflow of program counter");
    }

    carry
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

fn branch(sys: &mut SystemState, predicate: bool) -> (u8, u8) {
    if predicate {
        let displacement = get_immediate_byte(sys, 1);
        let displacement_mag = displacement & 0x7f;

        let page_cross = if (displacement & 0x80) != 0 {
            decrement_pc(sys, displacement_mag)
        } else {
            increment_pc(sys, displacement_mag)
        };
        (0, 3 + page_cross as u8) // TODO: verify this byte offset is correct
    } else {
        (2, 2)
    }
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
        }
        _ => panic!("unsupported mode {:?} on instruction ADC", mode),
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
        }
        _ => panic!("unsupported mode {:?} on instruction AND", mode),
    };

    sys.cpu_state.a &= operand;

    set_n_z(sys, sys.cpu_state.a);

    (length, cycles)
}

fn asl(sys: &mut SystemState, mode: AddressingMode) -> (u8, u8) {
    let (operand, length, cycles) = match mode {
        AddressingMode::Acc => (sys.cpu_state.a, 1, 2),
        AddressingMode::A => (get_absolute_byte(sys), 3, 6),
        AddressingMode::Zp => (get_zero_page_byte(sys), 2, 5),
        AddressingMode::Aix => (get_absolute_byte_indexed(sys, sys.cpu_state.x).0, 3, 7),
        AddressingMode::Zpix => (get_zero_page_byte_indexed(sys, sys.cpu_state.x), 2, 6),
        _ => panic!("unsupported mode {:?} on instruction ASL", mode),
    };

    let result = operand << 1;

    match mode {
        AddressingMode::Acc => sys.cpu_state.a = result,
        AddressingMode::A => set_absolute_byte(sys, result),
        AddressingMode::Zp => set_zero_page_byte(sys, result),
        AddressingMode::Aix => set_absolute_byte_indexed(sys, sys.cpu_state.x, result),
        AddressingMode::Zpix => set_zero_page_byte_indexed(sys, sys.cpu_state.x, result),
        _ => panic!("unsupported mode {:?} on instruction ASL", mode),
    }

    (length, cycles)
}

fn bcc(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, !sys.cpu_state.carry)
}

fn bcs(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, sys.cpu_state.carry)
}

fn beq(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, sys.cpu_state.zero)
}

fn bit(sys: &mut SystemState, mode: AddressingMode) -> (u8, u8) {
    let (operand, length, cycles) = match mode {
        AddressingMode::A => (get_absolute_byte(sys), 3, 4),
        AddressingMode::Zp => (get_zero_page_byte(sys), 2, 3),
        _ => panic!("unsupported mode {:?} on instruction BIT", mode),
    };

    sys.cpu_state.negative = negative_u8(operand);
    sys.cpu_state.signed_overflow = (operand & 0x40) != 0x00;
    sys.cpu_state.zero = (operand & sys.cpu_state.a) == 0x00;

    (length, cycles)
}

fn bmi(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, sys.cpu_state.negative)
}

fn bne(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, !sys.cpu_state.zero)
}

fn bpl(sys: &mut SystemState) -> (u8, u8) {
    branch(sys, !sys.cpu_state.negative)
}

// -- Emulation zone --

pub fn emulate_op(sys: &mut SystemState) -> u8 {
    let opcode = get_immediate_byte(sys, 0);

    let (length, cyc) = match opcode {
        0x06 => asl(sys, AddressingMode::Zp),
        0x0a => asl(sys, AddressingMode::Acc),
        0x0e => asl(sys, AddressingMode::A),

        0x10 => bpl(sys),

        0x1e => asl(sys, AddressingMode::Aix),
        0x16 => asl(sys, AddressingMode::Zpix),

        0x21 => and(sys, AddressingMode::Zpiix),

        0x24 => bit(sys, AddressingMode::Zp),

        0x25 => and(sys, AddressingMode::Zp),
        0x29 => and(sys, AddressingMode::I),

        0x2c => bit(sys, AddressingMode::A),

        0x2d => and(sys, AddressingMode::A),

        0x30 => bmi(sys),

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

        0x90 => bcc(sys),

        0xb0 => bcs(sys),

        0xd0 => bne(sys),

        0xf0 => beq(sys),

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
