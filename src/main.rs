mod cpu;

fn main() {
    let mut sys = cpu::SystemState {
        cpu_state: cpu::CpuState::default(),
        memory: [0; 0x10000],
    };

    cpu::emulate_op(&mut sys);
}
