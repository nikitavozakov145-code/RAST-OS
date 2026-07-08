use crate::{println, print};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::instructions::port::Port;
use pic8259::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        (gdt, Selectors {
            code_selector,
            data_selector,
            user_code_selector,
            user_data_selector,
        })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
}

pub fn init_gdt() {
    GDT.0.load();
    unsafe {
        x86_64::instructions::segmentation::set_cs(GDT.1.code_selector);
        x86_64::instructions::segmentation::load_ds(GDT.1.data_selector);
        x86_64::instructions::segmentation::load_es(GDT.1.data_selector);
        x86_64::instructions::segmentation::load_ss(GDT.1.data_selector);
        x86_64::instructions::segmentation::load_fs(GDT.1.data_selector);
        x86_64::instructions::segmentation::load_gs(GDT.1.data_selector);
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // Исключения
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
        // IRQ
        idt[PIC_1_OFFSET as usize].set_handler_fn(timer_interrupt_handler);
        idt[PIC_1_OFFSET as usize + 1].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_pic() {
    unsafe {
        PICS.lock().initialize();
    }
}

pub fn init_pit() {
    let mut cmd_port = Port::new(0x43);
    let mut data_port = Port::new(0x40);
    let divisor: u16 = 1193;  // ~1000 Гц для примера; ограничение 16 бит
    let divisor_lo = (divisor & 0xFF) as u8;
    let divisor_hi = ((divisor >> 8) & 0xFF) as u8;
    unsafe {
        cmd_port.write(0b00_11_010_0u8); // канал 0, режим 2, двоичный счёт
        data_port.write(divisor_lo);
        data_port.write(divisor_hi);
    }
}

// --------- Обработчики исключений ---------
macro_rules! exception_handler {
    ($name:ident, $msg:expr) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            println!("{}", $msg);
            println!("{:#?}", stack_frame);
            loop { x86_64::instructions::hlt(); }
        }
    };
}

exception_handler!(divide_error_handler, "EXCEPTION: DIVIDE BY ZERO");
exception_handler!(debug_handler, "EXCEPTION: DEBUG");
exception_handler!(nmi_handler, "EXCEPTION: NMI");
exception_handler!(breakpoint_handler, "EXCEPTION: BREAKPOINT");
exception_handler!(overflow_handler, "EXCEPTION: OVERFLOW");
exception_handler!(bound_range_exceeded_handler, "EXCEPTION: BOUND RANGE EXCEEDED");
exception_handler!(invalid_opcode_handler, "EXCEPTION: INVALID OPCODE");
exception_handler!(device_not_available_handler, "EXCEPTION: DEVICE NOT AVAILABLE");
exception_handler!(invalid_tss_handler, "EXCEPTION: INVALID TSS");
exception_handler!(segment_not_present_handler, "EXCEPTION: SEGMENT NOT PRESENT");
exception_handler!(stack_segment_fault_handler, "EXCEPTION: STACK SEGMENT FAULT");
exception_handler!(x87_floating_point_handler, "EXCEPTION: x87 FLOATING POINT");
exception_handler!(alignment_check_handler, "EXCEPTION: ALIGNMENT CHECK");
exception_handler!(machine_check_handler, "EXCEPTION: MACHINE CHECK");
exception_handler!(simd_floating_point_handler, "EXCEPTION: SIMD FLOATING POINT");
exception_handler!(virtualization_handler, "EXCEPTION: VIRTUALIZATION");

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    println!("EXCEPTION: DOUBLE FAULT (error_code = {:#x})", error_code);
    println!("{:#?}", stack_frame);
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT (error_code = {:#x})", error_code);
    println!("{:#?}", stack_frame);
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    loop { x86_64::instructions::hlt(); }
}

// --------- Прерывания устройств ---------
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Отправить EOI ведущему PIC
    unsafe {
        PICS.lock().notify_end_of_interrupt(32);
    }
    // Можно добавить счётчик тиков или переключение контекста
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    // Простейшая обработка: выводим сканкод
    println!("Key pressed: {:#x}", scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(33);
    }
}
