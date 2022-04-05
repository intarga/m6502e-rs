mod cpu;

fn main() {
    let mut state = cpu::CpuState::default();

    cpu::emulate_op(0xff, &mut state);
}
