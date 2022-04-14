mod cpu;

fn main() {
    let mut sys = cpu::SystemState::default();

    cpu::emulate_op(&mut sys);
}
