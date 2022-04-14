#[derive(Default)]
pub struct CpuState {
    a: u8,
    x: u8,
    y: u8,
    pch: u8,
    pcl: u8,
    s: u8,
    p: u8,
}

pub struct SystemState {
    cpu_state: CpuState,
    memory: [u8; 0x10000],
}

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

pub fn emulate_op(sys: &mut SystemState) {
    let opcode = get_byte_at_addr(sys, 0x0000);

    match opcode {
        _ => println!("unimplemented instruction {}", opcode),
    }
}
