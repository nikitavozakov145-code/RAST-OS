#![no_std]
#![no_main]
#![feature(alloc_error_handler)] // для кастомного обработчика ошибок аллокации

mod vga_buffer;
mod interrupts;
mod memory;

use core::panic::PanicInfo;
use bootloader::BootInfo;

// Глобальный аллокатор
#[global_allocator]
static ALLOCATOR: memory::BumpAllocator = memory::BumpAllocator::new();

// Точка входа, вызывается загрузчиком
#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    // Инициализация VGA (очистка, приветствие)
    vga_buffer::clear_screen();
    println!("Welcome to rast os!");
    println!("Initializing...");

    // Инициализация аллокатора памяти
    memory::init_heap(boot_info);

    // Инициализация GDT и IDT
    interrupts::init_idt();
    interrupts::init_gdt();

    // Инициализация контроллера прерываний и настройка PIT
    interrupts::init_pic();
    interrupts::init_pit();

    // Разрешаем прерывания
    x86_64::instructions::interrupts::enable();

    println!("rast os is running!");

    // Основной цикл ядра
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}
