use kernel::gdt;
use spin::Lazy;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

pub static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.stack_segment_fault.set_handler_fn(stack_segment_fault);

    idt
});

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint(stack_frame: InterruptStackFrame) {
    log::error!("EXCEPTION: BREAKPOINT\n{stack_frame:#?}");
}

extern "x86-interrupt" fn double_fault(stack_frame: InterruptStackFrame, _e: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{stack_frame:#?}");
}

extern "x86-interrupt" fn stack_segment_fault(stack_frame: InterruptStackFrame, e: u64) {
    log::error!("EXCEPTION: STACK SEGMENT FAULT\n{stack_frame:#?}");
    log::error!("ERROR CODE: {e}");
}
