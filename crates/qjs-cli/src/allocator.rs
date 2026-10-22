//! Thread-local, page-structured size-class global allocator for the CLI.
//!
//! The CLI is a short-lived host process, so this allocator trades some
//! retention for speed on the allocation/free path that real programs spend a
//! large share of their time in. Memory is organised the way mimalloc does it:
//! fixed-size pages, one class per page, a per-page free list and live count,
//! and page-level recycling instead of permanently retaining every freed block.
//!
//! What the module owns:
//!
//! - 32 size classes: 16-byte steps through 128 bytes, then four equal classes
//!   per power of two through 8 KiB. A block is always aligned to at least 16
//!   bytes and holds at least the requested size.
//! - Pages: one 64-KiB, 64-KiB-aligned System allocation per page. The page
//!   header sits at the page base, so `block & !(PAGE_SIZE - 1)` recovers it.
//!   Each page serves exactly one class, keeps its own free list, bump area and
//!   live count, and is linked into its owning thread's class list.
//! - Per-thread state: the class lists, the current page per class, a queue of
//!   pages with free blocks (most recently freed first), and a bounded pool of
//!   at most `EMPTY_PAGES` empty pages reusable by any class. Pages beyond that
//!   bound are returned to System. Allocation reuses a queued free block before
//!   extending: it bumps the current page only when no page has a free block,
//!   scans the remaining remote lists and bump space next, and takes a new or
//!   empty page last.
//!
//! Invariants that keep it sound:
//!
//! - A page's ordinary fields (`class`, `live`, `free`, `bump`, `bump_end`,
//!   `next`, `free_next`, `free_prev`, `queued`) are touched only by the thread
//!   that owns the page. Remote frees use only `owner` and the atomic `remote`
//!   stack.
//! - `live` counts blocks that are allocated plus blocks sitting in `remote`.
//!   A remote free pushes the block and never touches `live`; the owner
//!   subtracts the drained count when it takes the remote list. Therefore
//!   `live == 0` proves that no block is allocated, no remote push is
//!   undrained, and no future remote push is possible (a push requires an
//!   allocated block). That is the only condition under which a page is
//!   recycled.
//! - `owner` is written when a page becomes that thread's page and is reset
//!   only after `live == 0`. A block can only be freed while it is live, so a
//!   remote freer always observes the owning thread's id; the Release store at
//!   page initialisation and the Acquire load at free establish the ordering.
//! - A recycled page is unlinked and either kept in the owner's empty pool or
//!   returned to System with the same page layout it was allocated with. An
//!   empty page has no live blocks and no undrained remote entries, so no
//!   pointer into it can be freed.
//! - `thread_local!` state is const-initialised and registers no destructor
//!   (`Cell<*mut ...>`/`Cell<usize>` have no drop glue), so no thread ever
//!   observes a destroyed pool. Pages and lists retained by a thread that
//!   exits are leaked; nothing can reference them afterwards, which is sound.
//! - The class table and page size are static, so a block pointer always masks
//!   to its page header. `realloc` never changes alignment (the Rust trait
//!   fixes it): same-class growth keeps the block, System-to-System forwards to
//!   `System::realloc`, and everything else allocates, copies `min(old, new)`
//!   and frees the old block.
//!
//! No dependency, no FFI, no global mutable state beyond the
//! `#[global_allocator]` static and the thread-id counter. Library crates keep
//! their allocator so embedders choose their own.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    ptr,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

/// Diagnostic counters for the repair round. Compiled only with the
/// `alloc-counters` feature, which is never part of a timed build.
mod stats {
    #[cfg(feature = "alloc-counters")]
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(feature = "alloc-counters")]
    static SYSTEM_PAGE_ALLOCS: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static SYSTEM_PAGE_FREES: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static EMPTY_POOL_HITS: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static PAGE_RECYCLES: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static BLOCK_ALLOCS: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static BLOCK_FREES: AtomicU64 = AtomicU64::new(0);
    #[cfg(feature = "alloc-counters")]
    static SCAN_VISITS: AtomicU64 = AtomicU64::new(0);

    #[inline]
    pub(super) fn system_page_alloc() {
        #[cfg(feature = "alloc-counters")]
        SYSTEM_PAGE_ALLOCS.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn system_page_free() {
        #[cfg(feature = "alloc-counters")]
        SYSTEM_PAGE_FREES.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn empty_pool_hit() {
        #[cfg(feature = "alloc-counters")]
        EMPTY_POOL_HITS.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn page_recycle() {
        #[cfg(feature = "alloc-counters")]
        PAGE_RECYCLES.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn block_alloc() {
        #[cfg(feature = "alloc-counters")]
        BLOCK_ALLOCS.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn block_free() {
        #[cfg(feature = "alloc-counters")]
        BLOCK_FREES.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub(super) fn scan_visit() {
        #[cfg(feature = "alloc-counters")]
        SCAN_VISITS.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn report() {
        #[cfg(feature = "alloc-counters")]
        eprintln!(
            "QJS_ALLOC_COUNTERS system_page_allocs={} system_page_frees={} empty_pool_hits={} page_recycles={} block_allocs={} block_frees={} scan_visits={}",
            SYSTEM_PAGE_ALLOCS.load(Ordering::Relaxed),
            SYSTEM_PAGE_FREES.load(Ordering::Relaxed),
            EMPTY_POOL_HITS.load(Ordering::Relaxed),
            PAGE_RECYCLES.load(Ordering::Relaxed),
            BLOCK_ALLOCS.load(Ordering::Relaxed),
            BLOCK_FREES.load(Ordering::Relaxed),
            SCAN_VISITS.load(Ordering::Relaxed),
        );
    }
}

/// Largest size served by an explicit class with 16-byte steps.
const SMALL_LIMIT: usize = 128;
/// Largest size served by any class; larger layouts are System-managed.
const MAX_CLASS: usize = 8192;
/// Number of classes; `class_index` and the pool arrays share this bound.
const CLASS_COUNT: usize = 32;
/// Uniform page size; pages are aligned to it so masking finds the header.
const PAGE_SIZE: usize = 64 * 1024;
const PAGE_MASK: usize = PAGE_SIZE - 1;
/// Space reserved for the page header at the page base (16-byte multiple).
const HEADER_SIZE: usize = 96;
/// Empty pages a thread keeps for reuse; the rest go back to System.
const EMPTY_PAGES: usize = 8;

/// Upper bound of each class, strictly increasing from 16 through 8192.
const CLASS_SIZES: [usize; CLASS_COUNT] = [
    16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 640, 768, 896, 1024,
    1280, 1536, 1792, 2048, 2560, 3072, 3584, 4096, 5120, 6144, 7168, 8192,
];

const _: () = assert!(CLASS_SIZES[0] == 16 && CLASS_SIZES[CLASS_COUNT - 1] == MAX_CLASS);
const _: () = assert!(HEADER_SIZE % 16 == 0 && HEADER_SIZE < PAGE_SIZE);

/// Header at the base of every page. Only the owning thread touches all fields
/// except `owner` and `remote`; remote frees touch exactly those two.
#[repr(C)]
struct PageHeader {
    /// Class served by this page; owner-only.
    class: usize,
    /// Owning thread id, or 0 while the page sits in an empty pool.
    owner: AtomicUsize,
    /// Allocated blocks plus blocks queued on `remote`; owner-only.
    live: usize,
    /// Blocks freed by other threads (Treiber stack through the first word).
    remote: AtomicPtr<u8>,
    /// Next page in the owning thread's class list; owner-only.
    next: *mut PageHeader,
    /// Free blocks; owner-only.
    free: *mut u8,
    /// Next unallocated block and the end of the block area; owner-only.
    bump: *mut u8,
    bump_end: *mut u8,
    /// Thread-local queue of pages with free blocks; owner-only.
    free_next: *mut PageHeader,
    free_prev: *mut PageHeader,
    /// Whether `free_next`/`free_prev` currently link this page; owner-only.
    queued: bool,
}

const _: () = assert!(std::mem::size_of::<PageHeader>() <= HEADER_SIZE);

/// One class's pages owned by this thread, and the preferred page.
struct ClassState {
    pages: Cell<*mut PageHeader>,
    current: Cell<*mut PageHeader>,
    /// Pages with at least one free block, most recently freed first.
    free_pages: Cell<*mut PageHeader>,
}

impl ClassState {
    const fn new() -> Self {
        Self {
            pages: const { Cell::new(ptr::null_mut()) },
            current: const { Cell::new(ptr::null_mut()) },
            free_pages: const { Cell::new(ptr::null_mut()) },
        }
    }
}

struct Pool {
    /// Stable non-zero id for this thread; 0 means "not assigned yet".
    id: Cell<usize>,
    classes: [ClassState; CLASS_COUNT],
    empty: [Cell<*mut PageHeader>; EMPTY_PAGES],
    empty_len: Cell<usize>,
}

impl Pool {
    const fn new() -> Self {
        Self {
            id: const { Cell::new(0) },
            classes: [const { ClassState::new() }; CLASS_COUNT],
            empty: [const { Cell::new(ptr::null_mut()) }; EMPTY_PAGES],
            empty_len: const { Cell::new(0) },
        }
    }

    fn thread_id(&self) -> usize {
        let id = self.id.get();
        if id != 0 {
            return id;
        }
        let id = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
        self.id.set(id);
        id
    }
}

/// Unique non-zero thread ids; never reused, so a stale id cannot match.
static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    /// Per-thread pool; no destructor, so it stays valid for the whole thread.
    static POOL: Pool = const { Pool::new() };
}

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

#[inline]
fn page_layout() -> Layout {
    // SAFETY: PAGE_SIZE is a non-zero power of two, so size and alignment are
    // valid and the size is a multiple of the alignment.
    unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) }
}

#[inline]
const fn page_blocks(block_size: usize) -> usize {
    (PAGE_SIZE - HEADER_SIZE) / block_size
}

/// Page that owns `block`. The page base is aligned to `PAGE_SIZE`, so masking
/// recovers the header.
#[inline]
fn page_of(block: *mut u8) -> *mut PageHeader {
    (block as usize & !PAGE_MASK) as *mut PageHeader
}

/// Link a page into its class's free-block queue. Owner-only.
#[inline]
unsafe fn enqueue_free(pool: &Pool, index: usize, page: *mut PageHeader) {
    // SAFETY: the page is live and owned by this thread.
    unsafe {
        if (*page).queued {
            return;
        }
        let state = &pool.classes[index];
        (*page).queued = true;
        (*page).free_prev = ptr::null_mut();
        (*page).free_next = state.free_pages.get();
        if !(*page).free_next.is_null() {
            (*(*page).free_next).free_prev = page;
        }
        state.free_pages.set(page);
    }
}

/// Unlink a page from its class's free-block queue. Owner-only.
#[inline]
unsafe fn dequeue_free(pool: &Pool, index: usize, page: *mut PageHeader) {
    // SAFETY: the page is owned by this thread.
    unsafe {
        if !(*page).queued {
            return;
        }
        let state = &pool.classes[index];
        if (*page).free_prev.is_null() {
            state.free_pages.set((*page).free_next);
        } else {
            (*(*page).free_prev).free_next = (*page).free_next;
        }
        if !(*page).free_next.is_null() {
            (*(*page).free_next).free_prev = (*page).free_prev;
        }
        (*page).free_next = ptr::null_mut();
        (*page).free_prev = ptr::null_mut();
        (*page).queued = false;
    }
}

/// Move a page's remotely freed blocks onto its local free list. Owner-only.
#[inline]
unsafe fn drain_remote(page: *mut PageHeader) {
    // SAFETY: the page is owned by this thread; the remote list is atomic and
    // is exchanged so each queued block is drained exactly once.
    unsafe {
        let mut cursor = (*page).remote.swap(ptr::null_mut(), Ordering::AcqRel);
        if cursor.is_null() {
            return;
        }
        let mut drained = 0usize;
        while !cursor.is_null() {
            // Remote entries are blocks of this page and were at least 16
            // bytes, so each stores the next entry in its first word.
            let next = cursor.cast::<*mut u8>().read();
            cursor.cast::<*mut u8>().write((*page).free);
            (*page).free = cursor;
            cursor = next;
            drained += 1;
        }
        // Queued blocks were counted in `live`; they are free now.
        (*page).live -= drained;
    }
}

#[inline]
unsafe fn has_bump(page: *mut PageHeader) -> bool {
    // SAFETY: the page is owned by this thread.
    unsafe {
        let bump = (*page).bump;
        !bump.is_null() && bump < (*page).bump_end
    }
}

/// Carve the next block from a page's bump area. The caller checked `has_bump`.
#[inline]
unsafe fn bump_block(page: *mut PageHeader) -> *mut u8 {
    // SAFETY: the bump area was sized to whole blocks, so `bump` has
    // `CLASS_SIZES[class]` bytes before `bump_end`.
    unsafe {
        let bump = (*page).bump;
        (*page).bump = bump.add(CLASS_SIZES[(*page).class]);
        (*page).live += 1;
        stats::block_alloc();
        bump
    }
}

/// Take one block from `page`, which is the free-queue head. Returns `None` when
/// the entry is stale (no local or remote block left).
#[inline]
unsafe fn take_queued(pool: &Pool, index: usize, page: *mut PageHeader) -> Option<*mut u8> {
    // SAFETY: the page is live and owned by this thread, and it is the queue
    // head, so no other thread touches its owner-only fields.
    unsafe {
        if (*page).live == 0 {
            // Defensive: a fully free page should have been recycled already.
            dequeue_free(pool, index, page);
            recycle_page(pool, index, page);
            return None;
        }
        if (*page).free.is_null() {
            drain_remote(page);
        }
        let block = (*page).free;
        if block.is_null() {
            dequeue_free(pool, index, page);
            return None;
        }
        (*page).free = block.cast::<*mut u8>().read();
        (*page).live += 1;
        stats::block_alloc();
        pool.classes[index].current.set(page);
        if (*page).free.is_null() && (*page).remote.load(Ordering::Relaxed).is_null() {
            dequeue_free(pool, index, page);
        }
        Some(block)
    }
}

/// Take a fresh page for `index`: reuse an empty page when possible, otherwise
/// ask System for one. Links the page into the class list and makes it current.
#[inline]
unsafe fn new_page(pool: &Pool, index: usize) -> *mut PageHeader {
    // SAFETY: System receives a valid page layout, and every field written here
    // belongs to this thread's page (its owner id is stored before any block is
    // handed out).
    unsafe {
        let len = pool.empty_len.get();
        let page = if len == 0 {
            stats::system_page_alloc();
            let raw = System.alloc(page_layout());
            if raw.is_null() {
                return ptr::null_mut();
            }
            raw.cast::<PageHeader>()
        } else {
            stats::empty_pool_hit();
            let page = pool.empty[len - 1].get();
            pool.empty_len.set(len - 1);
            page
        };
        let base = page as *mut u8;
        let block_size = CLASS_SIZES[index];
        (*page).class = index;
        (*page).owner.store(pool.thread_id(), Ordering::Release);
        (*page).live = 0;
        (*page).remote.store(ptr::null_mut(), Ordering::Relaxed);
        (*page).free = ptr::null_mut();
        // The header occupies the first HEADER_SIZE bytes of a PAGE_SIZE page,
        // and the block area holds `page_blocks` whole blocks.
        (*page).bump = base.add(HEADER_SIZE);
        (*page).bump_end = base.add(HEADER_SIZE + page_blocks(block_size) * block_size);
        (*page).free_next = ptr::null_mut();
        (*page).free_prev = ptr::null_mut();
        (*page).queued = false;
        let state = &pool.classes[index];
        (*page).next = state.pages.get();
        state.pages.set(page);
        state.current.set(page);
        page
    }
}

/// Unlink a fully free page from the class list and either park it in the empty
/// pool or return it to System.
#[inline]
unsafe fn recycle_page(pool: &Pool, index: usize, page: *mut PageHeader) {
    // SAFETY: the caller proved `live == 0`, so no block of this page can be
    // freed again and the page is owned by this thread. Unlinking and the
    // System deallocation use the page's own bookkeeping.
    unsafe {
        stats::page_recycle();
        dequeue_free(pool, index, page);
        let state = &pool.classes[index];
        if state.current.get() == page {
            state.current.set(ptr::null_mut());
        }
        let mut cursor = state.pages.get();
        if cursor == page {
            state.pages.set((*page).next);
        } else {
            while !cursor.is_null() {
                if (*cursor).next == page {
                    (*cursor).next = (*page).next;
                    break;
                }
                cursor = (*cursor).next;
            }
        }
        (*page).next = ptr::null_mut();
        (*page).owner.store(0, Ordering::Release);
        let len = pool.empty_len.get();
        if len < EMPTY_PAGES {
            pool.empty[len].set(page);
            pool.empty_len.set(len + 1);
        } else {
            // The page came from System with exactly this layout.
            stats::system_page_free();
            System.dealloc(page.cast::<u8>(), page_layout());
        }
    }
}

/// Reuse a free block before extending: the class's free-page queue first, then
/// the current page's bump area, then a scan that drains remote lists and looks
/// for stranded bump space, and only then a new page.
#[inline]
unsafe fn allocate(index: usize) -> *mut u8 {
    POOL.with(|pool| {
        let state = &pool.classes[index];
        loop {
            let queued = state.free_pages.get();
            if !queued.is_null() {
                // SAFETY: the queue holds live pages owned by this thread.
                if let Some(block) = unsafe { take_queued(pool, index, queued) } {
                    return block;
                }
                continue;
            }
            let current = state.current.get();
            if !current.is_null() {
                // SAFETY: `current` is a live page owned by this thread.
                if unsafe { has_bump(current) } {
                    return unsafe { bump_block(current) };
                }
            }
            // No known free block and no bump on the current page. Drain every
            // page's remote list and remember whether any page still has bump
            // space before taking a new page.
            let mut bump_page: *mut PageHeader = ptr::null_mut();
            let mut cursor = state.pages.get();
            while !cursor.is_null() {
                stats::scan_visit();
                // SAFETY: every linked page is live and owned by this thread.
                let next = unsafe { (*cursor).next };
                if !unsafe { (*cursor).remote.load(Ordering::Relaxed) }.is_null() {
                    unsafe { drain_remote(cursor) };
                    if !unsafe { (*cursor).free.is_null() } {
                        unsafe { enqueue_free(pool, index, cursor) };
                    }
                }
                if bump_page.is_null() && unsafe { has_bump(cursor) } {
                    bump_page = cursor;
                }
                cursor = next;
            }
            if !state.free_pages.get().is_null() {
                continue;
            }
            if !bump_page.is_null() {
                state.current.set(bump_page);
                // SAFETY: the page has bump space.
                return unsafe { bump_block(bump_page) };
            }
            // SAFETY: the pool belongs to this thread and `index` is a class.
            let page = unsafe { new_page(pool, index) };
            if page.is_null() {
                return page.cast::<u8>();
            }
            // SAFETY: a fresh page always has bump space.
            return unsafe { bump_block(page) };
        }
    })
}

/// Return a block to its page: locally when this thread owns the page, remotely
/// through the atomic list otherwise.
#[inline]
unsafe fn release(index: usize, block: *mut u8) {
    // SAFETY: `block` came from this allocator with `index`'s class, so masking
    // finds its live page header.
    let page = page_of(block);
    POOL.with(|pool| {
        // SAFETY: `owner` is atomic and stable while the block is live; a local
        // free touches only fields this thread owns, and a remote free uses the
        // atomic list head and a block whose first word is free.
        unsafe {
            if (*page).owner.load(Ordering::Acquire) == pool.thread_id() {
                block.cast::<*mut u8>().write((*page).free);
                (*page).free = block;
                stats::block_free();
                enqueue_free(pool, index, page);
                let live = (*page).live - 1;
                (*page).live = live;
                if live == 0 {
                    // No block is allocated or queued remotely, so the page
                    // cannot be reached again.
                    recycle_page(pool, index, page);
                }
                return;
            }
            // Remote free: publish the block and leave `live` for the owner to
            // subtract when it drains the list.
            let mut head = (*page).remote.load(Ordering::Relaxed);
            loop {
                block.cast::<*mut u8>().write(head);
                match (*page).remote.compare_exchange_weak(
                    head,
                    block,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => head = observed,
                }
            }
        }
    })
}

#[inline]
unsafe fn allocate_zeroed(index: usize) -> *mut u8 {
    // SAFETY: forwarding to this module's class allocator.
    let block = unsafe { allocate(index) };
    if !block.is_null() {
        // SAFETY: the block holds at least `CLASS_SIZES[index]` usable bytes;
        // clearing the whole class also clears the requested prefix and is
        // required because a recycled block holds a stale free-list pointer.
        unsafe { ptr::write_bytes(block, 0, CLASS_SIZES[index]) };
    }
    block
}

/// Print allocator counters when built with `alloc-counters`; otherwise no-op.
pub(crate) fn report_counters() {
    stats::report();
}

pub(crate) struct SizeClassAllocator;

// SAFETY: `SizeClassAllocator` satisfies the `GlobalAlloc` contract: every
// returned block is non-null or null on failure, aligned to at least the
// requested alignment, usable for the requested size, and each `dealloc`/
// `realloc` receives the layout the block was allocated with. Class blocks live
// in 64-KiB pages aligned to 64 KiB, start at a 16-byte-multiple offset, and
// are class-size multiples apart, so they are 16-aligned; System-managed
// layouts keep their exact align and layout. `release` either updates the
// owning thread's page or publishes the block on that page's atomic remote
// list, and a page is only recycled once its live count proves no block can be
// freed again.
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
            // More blocks than one page holds for the larger classes, so the
            // page list, the bump area and page replacement are all exercised.
            let count = (page_blocks(size) + 2).min(300);
            let mut blocks = Vec::with_capacity(count);
            for round in 0..count {
                let byte = (index as u8) ^ (round as u8);
                // SAFETY: test-only use of the allocator with a valid layout.
                let block = unsafe { ALLOC.alloc(block_layout) };
                assert!(!block.is_null(), "class {size} returned null");
                assert_eq!(block as usize % 16, 0, "class {size} misaligned");
                fill(block, size, byte);
                check_fill(block, size, byte);
                blocks.push((block, byte));
            }
            for (block, byte) in blocks {
                check_fill(block, size, byte);
                // SAFETY: `block` came from this allocator with this layout.
                unsafe { ALLOC.dealloc(block, block_layout) };
            }
            // Recycled pages and the empty pool serve the next request too.
            // SAFETY: test-only allocation with a valid layout.
            let block = unsafe { ALLOC.alloc(block_layout) };
            assert!(!block.is_null());
            assert_eq!(block as usize % 16, 0);
            // SAFETY: `block` came from this allocator with this layout.
            unsafe { ALLOC.dealloc(block, block_layout) };
        }
    }

    #[test]
    fn live_count_tracks_local_allocations_and_frees() {
        let block_layout = layout(64, 8);
        // SAFETY: test-only allocation with a valid layout.
        let first = unsafe { ALLOC.alloc(block_layout) };
        let page = page_of(first);
        // The test thread's pool already served harness allocations, so work
        // from the page's current count instead of assuming a fresh page.
        // SAFETY: `first` is live and belongs to `page`, owned by this thread.
        let base = unsafe { (*page).live };
        // SAFETY: test-only allocation with a valid layout.
        let second = unsafe { ALLOC.alloc(block_layout) };
        // SAFETY: both blocks are live on the same page.
        assert_eq!(unsafe { (*page).live }, base + 1);
        // SAFETY: `first` came from this allocator with this layout.
        unsafe { ALLOC.dealloc(first, block_layout) };
        // SAFETY: `page` still has live blocks, so it is not recycled.
        assert_eq!(unsafe { (*page).live }, base);
        // SAFETY: `second` came from this allocator with this layout. If it was
        // the page's last live block the page is recycled, so do not inspect it.
        unsafe { ALLOC.dealloc(second, block_layout) };
    }

    #[test]
    fn remote_free_is_drained_and_reused_by_the_owner() {
        use std::sync::{Arc, Barrier, atomic::AtomicUsize};
        const PATTERN: u8 = 0x6d;
        let block_layout = layout(48, 8);
        let barrier = Arc::new(Barrier::new(2));
        let address = Arc::new(AtomicUsize::new(0));
        let owner = {
            let barrier = Arc::clone(&barrier);
            let address = Arc::clone(&address);
            std::thread::spawn(move || {
                // SAFETY: test-only allocation with a valid layout.
                let block = unsafe { ALLOC.alloc(block_layout) };
                assert!(!block.is_null());
                // The first word becomes the remote list link, so pattern the
                // rest and verify it survives the round trip.
                // SAFETY: the block is live for 48 bytes.
                unsafe { ptr::write_bytes(block.add(8), PATTERN, 40) };
                let page = page_of(block);
                // SAFETY: the page is owned by this thread.
                let live = unsafe { (*page).live };
                address.store(block as usize, Ordering::Release);
                barrier.wait();
                barrier.wait();
                // SAFETY: the other thread's free completed before this wait;
                // it queued the block remotely and left live unchanged.
                assert_eq!(unsafe { (*page).live }, live);
                // Reallocate until the queue is drained and the block returns.
                let mut found = false;
                for _ in 0..(page_blocks(48) * 2) {
                    // SAFETY: test-only allocation with a valid layout.
                    let reused = unsafe { ALLOC.alloc(block_layout) };
                    assert!(!reused.is_null());
                    if reused == block {
                        // SAFETY: the reused block is live for 48 bytes.
                        let tail = unsafe { std::slice::from_raw_parts(reused.add(8), 40) };
                        assert!(tail.iter().all(|&b| b == PATTERN), "content lost");
                        found = true;
                        break;
                    }
                }
                assert!(found, "remote block was never drained");
            })
        };
        barrier.wait();
        // SAFETY: the owner allocated this block and published it before the
        // barrier; it is freed exactly once here.
        unsafe { ALLOC.dealloc(address.load(Ordering::Acquire) as *mut u8, block_layout) };
        barrier.wait();
        owner.join().expect("owner panicked");
    }

    #[test]
    fn empty_pages_are_reused_by_another_class() {
        // Fill several pages of a large class, free every block, then exercise
        // a different class so recycled pages come back through the empty pool.
        let large = layout(8192, 16);
        let count = page_blocks(8192) * 2 + 1;
        let mut blocks = Vec::with_capacity(count);
        for _ in 0..count {
            // SAFETY: test-only allocation with a valid layout.
            let block = unsafe { ALLOC.alloc(large) };
            assert!(!block.is_null());
            fill(block, 8192, 0x3c);
            blocks.push(block);
        }
        for block in blocks {
            check_fill(block, 8192, 0x3c);
            // SAFETY: every block came from this allocator with `large`.
            unsafe { ALLOC.dealloc(block, large) };
        }
        let small = layout(24, 8);
        let mut reused = Vec::new();
        for _ in 0..1000 {
            // SAFETY: test-only allocation with a valid layout.
            let block = unsafe { ALLOC.alloc(small) };
            assert!(!block.is_null());
            assert_eq!(block as usize % 16, 0);
            fill(block, 24, 0x7e);
            reused.push(block);
        }
        for block in reused {
            check_fill(block, 24, 0x7e);
            // SAFETY: every block came from this allocator with `small`.
            unsafe { ALLOC.dealloc(block, small) };
        }
    }

    #[test]
    fn non_current_free_blocks_are_reused_before_bump() {
        let block_layout = layout(8192, 16);
        let blocks_per_page = page_blocks(8192);
        let mut blocks = Vec::new();
        // Fill the first page and start a second one, so the first page is no
        // longer the current page.
        for _ in 0..(blocks_per_page + 1) {
            // SAFETY: test-only allocation with a valid layout.
            blocks.push(unsafe { ALLOC.alloc(block_layout) });
        }
        let first = blocks[0];
        let second_page = blocks[blocks_per_page];
        assert_ne!(page_of(first), page_of(second_page));
        // SAFETY: `first` came from this allocator with this layout.
        unsafe { ALLOC.dealloc(first, block_layout) };
        // Reuse before extend: the freed block on the non-current page must come
        // back before the current page's bump space is touched.
        // SAFETY: test-only allocation with a valid layout.
        let reused = unsafe { ALLOC.alloc(block_layout) };
        assert_eq!(
            reused, first,
            "freed block on a non-current page was skipped"
        );
        // SAFETY: `reused` came from this allocator with this layout.
        unsafe { ALLOC.dealloc(reused, block_layout) };
        for block in blocks.into_iter().skip(1) {
            // SAFETY: every remaining block is live and freed once here.
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
        // This thread's pages still hold remote blocks; allocating drains them.
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
