//! Thread-local size-class global allocator for the CLI process.
//!
//! The CLI is a short-lived host process, so this allocator trades returning
//! memory to the system for speed on the allocation/free path that real
//! programs spend a large share of their time in. It owns three things and
//! nothing else:
//!
//! - 32 size classes: 16-byte steps through 128 bytes, then four equal classes
//!   per power of two through 8 KiB. A class block is always aligned to at
//!   least 16 bytes and holds at least the requested size.
//! - Per-thread free lists and bump slabs. A slab is one System allocation of
//!   `blocks * class_size` bytes taken with alignment 16. A class starts with a
//!   4 KiB slab and each later slab doubles, up to 64 KiB, so a lightly used
//!   class retains little memory while a hot class still amortizes mallocs.
//! - A `GlobalAlloc` that forwards anything outside the class contract (size
//!   above 8 KiB, alignment above 16, or zero size) to `System`.
//!
//! Invariants that keep it sound:
//!
//! - A block is owned by exactly one thread at a time. `dealloc` pushes it onto
//!   the *freeing* thread's list, which transfers ownership; no other thread
//!   ever reads that thread's pool. A block allocated on thread A and freed on
//!   thread B is therefore reused only by B, and A's list never sees it again.
//! - Slabs are never deallocated, so every block's backing storage lives for
//!   the whole process. A thread that exits leaks its free lists and slabs;
//!   nothing can reference them afterwards, which is sound.
//! - `thread_local!` is const-initialized and the pool registers no destructor
//!   (`Cell<*mut u8>` has no drop glue), so no thread can observe a destroyed
//!   or partially initialized pool.
//! - The class table is static; class sizes are multiples of 16, so a block at
//!   `slab + k * class_size` is 16-aligned whenever the slab is.
//! - `realloc` changes only the size in the frozen Rust trait signature, so it
//!   never changes alignment. A same-class realloc returns the same block; a
//!   System-to-System realloc forwards to `System::realloc`; every other case
//!   allocates, copies `min(old, new)` bytes and frees the old block.
//!
//! No dependency, no FFI, no global mutable state beyond the
//! `#[global_allocator]` static itself. Library crates keep their allocator so
//! embedders choose their own.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    ptr,
};

/// Largest size served by an explicit class with 16-byte steps.
const SMALL_LIMIT: usize = 128;
/// Largest size served by any class; larger layouts are System-managed.
const MAX_CLASS: usize = 8192;
/// Number of classes; `class_index` and the pool arrays share this bound.
const CLASS_COUNT: usize = 32;
/// Bytes in the first slab of a class; later slabs double up to `SLAB_TARGET`.
const FIRST_SLAB: usize = 4 * 1024;
/// Upper bound for one class's slab; growth stops there.
const SLAB_TARGET: usize = 64 * 1024;

/// Upper bound of each class, strictly increasing from 16 through 8192.
const CLASS_SIZES: [usize; CLASS_COUNT] = [
    16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 640, 768, 896, 1024,
    1280, 1536, 1792, 2048, 2560, 3072, 3584, 4096, 5120, 6144, 7168, 8192,
];

const _: () = assert!(CLASS_SIZES[0] == 16 && CLASS_SIZES[CLASS_COUNT - 1] == MAX_CLASS);

/// Class index for a non-zero size at or below `MAX_CLASS`.
///
/// Sizes through `SMALL_LIMIT` step by 16 bytes. Above it, each octave
/// `(2^k, 2^(k+1)]` is split into four equal classes, so the worst internal
/// fragmentation is one quarter-step (at most 25% above the lower bound).
#[inline]
fn class_index(size: usize) -> Option<usize> {
    if size == 0 || size > MAX_CLASS {
        return None;
    }
    if size <= SMALL_LIMIT {
        return Some((size - 1) / 16);
    }
    let upper = size.next_power_of_two();
    let step = upper >> 3;
    let within = (size - (upper >> 1)).div_ceil(step);
    let octave = upper.trailing_zeros() as usize - 8;
    Some(8 + octave * 4 + within - 1)
}

/// Class index for a layout, or `None` when System must serve it.
#[inline]
fn class_index_for(layout: Layout) -> Option<usize> {
    if layout.align() > 16 {
        return None;
    }
    class_index(layout.size())
}

struct Pool {
    /// Singly linked free lists; a freed block stores the next pointer in its
    /// first word. Every class block is at least 16 bytes, so that word fits.
    free: [Cell<*mut u8>; CLASS_COUNT],
    /// Next unserved block in the current slab, and its one-past-the-end.
    next: [Cell<*mut u8>; CLASS_COUNT],
    end: [Cell<*mut u8>; CLASS_COUNT],
    /// Bytes in the last slab of each class, which sets the next slab's size.
    slab_bytes: [Cell<usize>; CLASS_COUNT],
}

impl Pool {
    const fn new() -> Self {
        Self {
            free: [const { Cell::new(ptr::null_mut()) }; CLASS_COUNT],
            next: [const { Cell::new(ptr::null_mut()) }; CLASS_COUNT],
            end: [const { Cell::new(ptr::null_mut()) }; CLASS_COUNT],
            slab_bytes: [const { Cell::new(0) }; CLASS_COUNT],
        }
    }
}

thread_local! {
    /// Per-thread pool; no destructor, so it stays valid for the whole thread.
    static POOL: Pool = const { Pool::new() };
}

/// Pop a recycled block or carve one from the current slab, allocating a new
/// slab when the current one is exhausted.
#[inline]
unsafe fn allocate(index: usize) -> *mut u8 {
    POOL.with(|pool| {
        let free = pool.free[index].get();
        if !free.is_null() {
            // SAFETY: a freed block was at least 16 bytes and stores the next
            // free block in its first word; it is no longer live.
            pool.free[index].set(unsafe { free.cast::<*mut u8>().read() });
            return free;
        }
        let block_size = CLASS_SIZES[index];
        let mut block = pool.next[index].get();
        if block.is_null() || block == pool.end[index].get() {
            // Start small and double: a class the program barely uses keeps one
            // small slab, while a hot class reaches the 64 KiB amortized size.
            let previous = pool.slab_bytes[index].get();
            let target = if previous == 0 {
                FIRST_SLAB
            } else {
                previous.saturating_mul(2).min(SLAB_TARGET)
            };
            let blocks = (target / block_size).max(1);
            let bytes = block_size * blocks;
            // SAFETY: `bytes` is a non-zero multiple of 16 bytes, so alignment
            // 16 is valid, and the size does not exceed `isize::MAX`.
            let slab = unsafe { System.alloc(Layout::from_size_align_unchecked(bytes, 16)) };
            if slab.is_null() {
                return slab;
            }
            block = slab;
            pool.slab_bytes[index].set(bytes);
            // SAFETY: the slab owns exactly `bytes` bytes.
            pool.end[index].set(unsafe { slab.add(bytes) });
        }
        // SAFETY: `block` is inside the slab and at least `block_size` bytes
        // remain before `end`.
        pool.next[index].set(unsafe { block.add(block_size) });
        block
    })
}

/// Push a block onto the current thread's free list.
#[inline]
unsafe fn release(index: usize, block: *mut u8) {
    POOL.with(|pool| {
        // SAFETY: `block` is no longer live, came from this class, and has room
        // for one pointer (class blocks are at least 16 bytes).
        unsafe { block.cast::<*mut u8>().write(pool.free[index].get()) };
        pool.free[index].set(block);
    })
}

#[inline]
unsafe fn allocate_zeroed(index: usize) -> *mut u8 {
    let block = unsafe { allocate(index) };
    if !block.is_null() {
        // SAFETY: the block holds at least `CLASS_SIZES[index]` usable bytes;
        // clearing the whole class also clears the requested prefix and is
        // required because a recycled block holds a stale free-list pointer.
        unsafe { ptr::write_bytes(block, 0, CLASS_SIZES[index]) };
    }
    block
}

pub(crate) struct SizeClassAllocator;

// SAFETY: `SizeClassAllocator` satisfies the `GlobalAlloc` contract: every
// returned block is non-null or null on failure, aligned to at least the
// requested alignment, usable for the requested size, and each `dealloc`/
// `realloc` receives the layout the block was allocated with. Class blocks
// originate from a 16-aligned System slab and are multiples of 16 bytes apart;
// System-managed layouts keep their exact align and layout. Thread ownership is
// transferred by `release`, and slabs outlive every block.
unsafe impl GlobalAlloc for SizeClassAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match class_index_for(layout) {
            // SAFETY: the class block is at least `layout.size()` bytes and
            // aligned to 16, which is at least `layout.align()`.
            Some(index) => unsafe { allocate(index) },
            // SAFETY: forwarding the caller's valid layout to System.
            None => unsafe { System.alloc(layout) },
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        match class_index_for(layout) {
            // SAFETY: same block contract as `alloc`; the whole class is cleared.
            Some(index) => unsafe { allocate_zeroed(index) },
            // SAFETY: forwarding the caller's valid layout to System.
            None => unsafe { System.alloc_zeroed(layout) },
        }
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        match class_index_for(layout) {
            // SAFETY: the caller passes the layout `block` was allocated with,
            // so it belongs to this class and is no longer live.
            Some(index) => unsafe { release(index, block) },
            // SAFETY: System-managed blocks are freed with their own layout.
            None => unsafe { System.dealloc(block, layout) },
        }
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        debug_assert!(new_size > 0);
        // The Rust trait fixes the alignment, so only the size can change.
        // SAFETY: `new_size` is positive and the caller guarantees the layout
        // describes a live block; alignment is unchanged.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        match (class_index_for(layout), class_index_for(new_layout)) {
            // Same class: the block already fits and keeps its alignment.
            (Some(old), Some(new)) if old == new => block,
            // Both System-managed with the same alignment (fixed by the trait).
            // SAFETY: forwarding the caller's block and layouts to System.
            (None, None) => unsafe { System.realloc(block, layout, new_size) },
            (old, new) => {
                let target = match new {
                    // SAFETY: the new class block fits `new_size` and its align.
                    Some(index) => unsafe { allocate(index) },
                    // SAFETY: the new System layout is valid and non-zero.
                    None => unsafe { System.alloc(new_layout) },
                };
                if target.is_null() {
                    return target;
                }
                // SAFETY: both blocks are live and valid for at least the
                // copied prefix; the regions cannot overlap because `target`
                // was just returned as a distinct live allocation.
                unsafe {
                    ptr::copy_nonoverlapping(block, target, layout.size().min(new_size));
                }
                match old {
                    // SAFETY: `block` still belongs to the old class.
                    Some(index) => unsafe { release(index, block) },
                    // SAFETY: `block` is still described by the old layout.
                    None => unsafe { System.dealloc(block, layout) },
                }
                target
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOC: SizeClassAllocator = SizeClassAllocator;

    fn layout(size: usize, align: usize) -> Layout {
        Layout::from_size_align(size, align).expect("valid test layout")
    }

    fn fill(block: *mut u8, size: usize, byte: u8) {
        // SAFETY: the caller guarantees `block` is live for `size` bytes.
        unsafe { ptr::write_bytes(block, byte, size) };
    }

    fn check_fill(block: *mut u8, size: usize, byte: u8) {
        // SAFETY: the caller guarantees `block` is live for `size` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(block, size) };
        assert!(bytes.iter().all(|&b| b == byte), "pattern corrupted");
    }

    #[test]
    fn class_table_is_dense_monotonic_and_bounded() {
        assert_eq!(CLASS_SIZES.len(), CLASS_COUNT);
        assert_eq!(CLASS_SIZES[0], 16);
        assert_eq!(CLASS_SIZES[CLASS_COUNT - 1], MAX_CLASS);
        for pair in CLASS_SIZES.windows(2) {
            assert!(pair[0] < pair[1]);
            assert_eq!(pair[0] % 16, 0);
        }
        for size in 1..=MAX_CLASS {
            let index = class_index(size).expect("size has a class");
            let class = CLASS_SIZES[index];
            assert!(class >= size, "class {class} smaller than {size}");
            assert_eq!(class_index(class), Some(index), "class boundary maps back");
            // Internal fragmentation is one 16-byte step or one quarter-step.
            assert!(
                class - size < 16.max(class / 4),
                "size {size} wastes {} bytes in class {class}",
                class - size
            );
        }
        assert_eq!(class_index(0), None);
        assert_eq!(class_index(MAX_CLASS + 1), None);
    }

    #[test]
    fn over_aligned_and_large_layouts_are_system_managed() {
        assert_eq!(class_index_for(layout(8, 32)), None);
        assert_eq!(class_index_for(layout(64, 64)), None);
        assert_eq!(class_index_for(layout(MAX_CLASS + 1, 8)), None);
        assert_eq!(class_index_for(layout(16, 16)), Some(0));
    }

    #[test]
    fn every_class_returns_aligned_usable_blocks() {
        for (index, &size) in CLASS_SIZES.iter().enumerate() {
            assert_eq!(class_index(size), Some(index));
            let block_layout = layout(size, 16);
            for round in 0..8 {
                let byte = (index as u8) ^ (round as u8);
                // SAFETY: test-only use of the allocator with a valid layout.
                let block = unsafe { ALLOC.alloc(block_layout) };
                assert!(!block.is_null(), "class {size} returned null");
                assert_eq!(block as usize % 16, 0, "class {size} misaligned");
                fill(block, size, byte);
                check_fill(block, size, byte);
                // SAFETY: `block` came from this allocator with this layout.
                unsafe { ALLOC.dealloc(block, block_layout) };
            }
            // Recycled blocks are handed out again with the same contract.
            // SAFETY: test-only allocation with a valid layout.
            let block = unsafe { ALLOC.alloc(block_layout) };
            assert!(!block.is_null());
            assert_eq!(block as usize % 16, 0);
            // SAFETY: `block` came from this allocator with this layout.
            unsafe { ALLOC.dealloc(block, block_layout) };
        }
    }

    #[test]
    fn realloc_same_class_keeps_block_and_bytes() {
        let old = layout(17, 8);
        // SAFETY: test-only allocation with a valid layout.
        let block = unsafe { ALLOC.alloc(old) };
        assert!(!block.is_null());
        fill(block, 17, 0x5a);
        // 17..32 all map to class 32, so growth stays in place.
        // SAFETY: `block` is live and described by `old`; 32 is positive.
        let grown = unsafe { ALLOC.realloc(block, old, 32) };
        assert_eq!(grown, block, "same-class realloc moved the block");
        check_fill(grown, 17, 0x5a);
        // SAFETY: `grown` was allocated with size 32 and alignment 8.
        unsafe { ALLOC.dealloc(grown, layout(32, 8)) };
    }

    #[test]
    fn realloc_across_classes_preserves_prefix() {
        // small -> larger class
        let old = layout(40, 8);
        // SAFETY: test-only allocation with a valid layout.
        let block = unsafe { ALLOC.alloc(old) };
        fill(block, 40, 0x11);
        // SAFETY: `block` is live and described by `old`.
        let bigger = unsafe { ALLOC.realloc(block, old, 700) };
        assert!(!bigger.is_null());
        check_fill(bigger, 40, 0x11);
        // SAFETY: `bigger` was allocated with size 700 and alignment 8.
        unsafe { ALLOC.dealloc(bigger, layout(700, 8)) };

        // small -> System-managed
        let old = layout(64, 8);
        // SAFETY: test-only allocation with a valid layout.
        let block = unsafe { ALLOC.alloc(old) };
        fill(block, 64, 0x22);
        // SAFETY: `block` is live and described by `old`.
        let large = unsafe { ALLOC.realloc(block, old, 40_000) };
        assert!(!large.is_null());
        check_fill(large, 64, 0x22);

        // System-managed -> System-managed (forwarded)
        // SAFETY: `large` is live and described by size 40_000, align 8.
        let larger = unsafe { ALLOC.realloc(large, layout(40_000, 8), 80_000) };
        assert!(!larger.is_null());
        check_fill(larger, 64, 0x22);

        // System-managed -> small class
        // SAFETY: `larger` is live and described by size 80_000, align 8.
        let small = unsafe { ALLOC.realloc(larger, layout(80_000, 8), 24) };
        assert!(!small.is_null());
        check_fill(small, 24, 0x22);
        // SAFETY: `small` was allocated with size 24 and alignment 8.
        unsafe { ALLOC.dealloc(small, layout(24, 8)) };
    }

    #[test]
    fn realloc_over_aligned_blocks_stays_system_managed() {
        let old = layout(8, 64);
        // SAFETY: test-only allocation with a valid layout.
        let block = unsafe { ALLOC.alloc(old) };
        assert!(!block.is_null());
        assert_eq!(block as usize % 64, 0);
        fill(block, 8, 0x33);
        // SAFETY: `block` is live and described by `old`; alignment is fixed.
        let grown = unsafe { ALLOC.realloc(block, old, 4096) };
        assert!(!grown.is_null());
        assert_eq!(grown as usize % 64, 0);
        check_fill(grown, 8, 0x33);
        // SAFETY: `grown` was allocated with size 4096 and alignment 64.
        unsafe { ALLOC.dealloc(grown, layout(4096, 64)) };
    }

    #[test]
    fn alloc_zeroed_clears_recycled_blocks() {
        for size in [24, 128, 5 * 1024, 20_000] {
            let block_layout = layout(size, 8);
            // SAFETY: test-only allocation with a valid layout.
            let dirty = unsafe { ALLOC.alloc(block_layout) };
            fill(dirty, size, 0xff);
            // SAFETY: `dirty` came from this allocator with this layout.
            unsafe { ALLOC.dealloc(dirty, block_layout) };
            // SAFETY: test-only zeroed allocation with a valid layout.
            let clean = unsafe { ALLOC.alloc_zeroed(block_layout) };
            assert!(!clean.is_null());
            check_fill(clean, size, 0);
            // SAFETY: `clean` came from this allocator with this layout.
            unsafe { ALLOC.dealloc(clean, block_layout) };
        }
    }

    #[test]
    fn cross_thread_frees_transfer_ownership() {
        let block_layout = layout(48, 8);
        // Allocate on this thread, free on worker threads.
        // SAFETY: test-only allocations with a valid layout.
        let blocks: Vec<usize> = (0..4_000)
            .map(|_| unsafe { ALLOC.alloc(block_layout) } as usize)
            .collect();
        let workers: Vec<_> = blocks
            .chunks(1_000)
            .map(|chunk| {
                let chunk = chunk.to_vec();
                std::thread::spawn(move || {
                    for address in chunk {
                        // SAFETY: each address was allocated once above and is
                        // freed exactly once here, with the same layout.
                        unsafe { ALLOC.dealloc(address as *mut u8, block_layout) };
                    }
                    // Workers also recycle their own blocks.
                    // SAFETY: test-only allocations with a valid layout.
                    let local = unsafe { ALLOC.alloc(block_layout) };
                    assert!(!local.is_null());
                    assert_eq!(local as usize % 16, 0);
                    // SAFETY: `local` came from this allocator.
                    unsafe { ALLOC.dealloc(local, block_layout) };
                })
            })
            .collect();
        for worker in workers {
            worker.join().expect("worker panicked");
        }
        // This thread's blocks were freed elsewhere; its pool is still usable.
        // SAFETY: test-only allocation with a valid layout.
        let block = unsafe { ALLOC.alloc(block_layout) };
        assert!(!block.is_null());
        assert_eq!(block as usize % 16, 0);
        fill(block, 48, 0x77);
        check_fill(block, 48, 0x77);
        // SAFETY: `block` came from this allocator with this layout.
        unsafe { ALLOC.dealloc(block, block_layout) };
    }

    #[test]
    fn mixed_stress_preserves_contents_and_alignment() {
        struct Live {
            block: *mut u8,
            layout: Layout,
            byte: u8,
        }

        let sizes = [
            1usize, 8, 16, 17, 33, 64, 100, 128, 129, 256, 777, 1024, 4096, 8192, 8193, 20_000,
        ];
        let mut state = 0x9e37_79b9_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut live: Vec<Option<Live>> = (0..256).map(|_| None).collect();
        for step in 0..20_000usize {
            let slot = (next() as usize) % live.len();
            match live[slot].take() {
                None => {
                    let size = sizes[(next() as usize) % sizes.len()];
                    let align = if next() % 16 == 0 { 32 } else { 8 };
                    let l = layout(size, align);
                    // SAFETY: test-only allocation with a valid layout.
                    let block = unsafe { ALLOC.alloc(l) };
                    assert!(!block.is_null(), "step {step} allocation failed");
                    assert_eq!(block as usize % align, 0);
                    let byte = (next() & 0xff) as u8;
                    fill(block, size, byte);
                    live[slot] = Some(Live {
                        block,
                        layout: l,
                        byte,
                    });
                }
                Some(entry) => {
                    if next() % 3 == 0 {
                        check_fill(entry.block, entry.layout.size(), entry.byte);
                        // SAFETY: `entry.block` came from this allocator with
                        // `entry.layout` and is freed once.
                        unsafe { ALLOC.dealloc(entry.block, entry.layout) };
                        continue;
                    }
                    let size = sizes[(next() as usize) % sizes.len()];
                    let copied = entry.layout.size().min(size);
                    // SAFETY: `entry.block` is live and described by its layout;
                    // `size` is positive.
                    let block = unsafe { ALLOC.realloc(entry.block, entry.layout, size) };
                    assert!(!block.is_null(), "step {step} realloc failed");
                    check_fill(block, copied, entry.byte);
                    let byte = (next() & 0xff) as u8;
                    fill(block, size, byte);
                    let l = layout(size, entry.layout.align());
                    live[slot] = Some(Live {
                        block,
                        layout: l,
                        byte,
                    });
                }
            }
        }
        for entry in live.into_iter().flatten() {
            check_fill(entry.block, entry.layout.size(), entry.byte);
            // SAFETY: each remaining block is live and freed once here.
            unsafe { ALLOC.dealloc(entry.block, entry.layout) };
        }
    }
}
