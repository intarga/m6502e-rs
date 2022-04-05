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

pub fn emulate_op(opcode: u8, state: &mut CpuState) {
    match opcode {
        0x00 => state.a = 0xff,
        _ => println!("unimplemented instruction {}", opcode),
    }

    println!("{}", state.a);
}
