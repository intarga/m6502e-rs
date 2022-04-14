#[derive(Default)]
pub struct CpuState {
    // registers
    a: u8,
    x: u8,
    y: u8,
    pch: u8,
    pcl: u8,
    s: u8,

    // flags
    negative: bool,
    signed_overflow: bool,
    brk_interrupt: bool,
    decimal_mode: bool,
    irq_interrupt_disable: bool,
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
    DP,    // Direct Page
    AIX,   // Absolute Indexed X
    AIY,   // Absolute Indexed Y
    DPIX,  // Direct Page Indexed X
    DPIIX, // Direct Page Indexed Indirect X
    DPIIY, // Direct Page Indexed Indirect Y
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

// -- Instructions --

fn adc(sys: &mut SystemState, mode: AddressingMode) -> (u8, u8) {
    let (operand, length, cycles) = match mode {
        AddressingMode::I => (get_immediate_byte(sys, 1), 2, 2),
        _ => panic!("unsupported mode {:?} on instruction ADC", mode),
    };

    let negative_before = negative_u8(sys.cpu_state.a);

    let (carry1, carry2): (bool, bool);
    (sys.cpu_state.a, carry1) = sys.cpu_state.a.overflowing_add(operand);
    (sys.cpu_state.a, carry2) = sys.cpu_state.a.overflowing_add(sys.cpu_state.carry as u8);

    sys.cpu_state.carry = carry1 || carry2;
    set_n_z(sys, sys.cpu_state.a);
    sys.cpu_state.signed_overflow = !negative_before && negative_u8(sys.cpu_state.a);

    (length, cycles)
}

pub fn emulate_op(sys: &mut SystemState) -> u8 {
    let opcode = get_immediate_byte(sys, 0);

    let (length, cyc) = match opcode {
        0x69 => adc(sys, AddressingMode::I),
        _ => panic!("unimplemented instruction {}", opcode),
    };

    increment_pc(sys, length);

    cyc
}
