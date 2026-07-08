use bootloader::BootInfo;
use spin::Mutex;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

const HEAP_SIZE: usize = 64 * 1024; // 64 KiB

static HEAP: Mutex<Option<BumpAllocatorInner>> = Mutex::new(None);

struct BumpAllocatorInner {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocatorInner {
    unsafe fn new(heap_start: usize, heap_size: usize) -> Self {
        BumpAllocatorInner {
            heap_start,
            heap_end: heap_start + heap_size,
            next: heap_start,
        }
    }

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > self.heap_end {
            null_mut() // не хватает места
        } else {
            self.next = alloc_end;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        // Bump аллокатор не освобождает память отдельными фрагментами
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub struct BumpAllocator;

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap = HEAP.lock();
        if let Some(ref mut inner) = *heap {
            inner.alloc(layout)
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut heap = HEAP.lock();
        if let Some(ref mut inner) = *heap {
            inner.dealloc(ptr, layout);
        }
    }
}

pub fn init_heap(boot_info: &BootInfo) {
    // Используем неиспользуемую память из карты памяти загрузчика.
    // Для простоты берём фиксированную область (например, конец памяти, куда загрузчик не пишет).
    // В реальной ОС нужно анализировать memory_map. Здесь – пример.
    let heap_start = 0x200000; // 2 МиБ, предполагаем, что свободно
    let heap_size = HEAP_SIZE;
    let mut heap = HEAP.lock();
    *heap = Some(unsafe { BumpAllocatorInner::new(heap_start, heap_size) });
    println!("Heap initialized: {:#x} - {:#x}", heap_start, heap_start + heap_size);
}
