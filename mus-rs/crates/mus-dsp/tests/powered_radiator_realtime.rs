use mus_dsp::powered_radiator::{PoweredRadiator, PoweredRadiatorControls};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    // Allocation activity in another concurrently executing test must not be
    // charged to the simulated audio thread.
    static COUNTING_THIS_THREAD: Cell<bool> = const { Cell::new(false) };
}

fn counting_this_thread() -> bool {
    COUNTING_THIS_THREAD.try_with(Cell::get).unwrap_or(false)
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if counting_this_thread() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if counting_this_thread() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if counting_this_thread() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn audio_path_allocates_nothing_after_prepare() {
    let mut body = PoweredRadiator::new(48_000.0).unwrap();
    let controls = PoweredRadiatorControls::default();

    // Warm any lazy runtime machinery before opening the measured interval.
    for index in 0..1024 {
        let sample = (index as f32 * 0.017).sin() * 0.2;
        let _ = body.process_sample(sample, -sample, controls);
    }

    ALLOCATIONS.store(0, Ordering::SeqCst);
    COUNTING_THIS_THREAD.with(|counting| counting.set(true));
    for index in 0..480_000 {
        let sample = (index as f32 * 0.013).sin() * 0.25;
        let _ = body.process_sample(sample, sample * 0.9, controls);
    }
    COUNTING_THIS_THREAD.with(|counting| counting.set(false));

    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
    assert_eq!(body.receipt().non_finite_resets, 0);
}

#[test]
fn arbitrary_block_partitioning_is_sample_identical() {
    let controls = PoweredRadiatorControls::default();
    let mut sample_by_sample = PoweredRadiator::new(48_000.0).unwrap();
    let mut blocked = PoweredRadiator::new(48_000.0).unwrap();
    let mut source_left = vec![0.0_f32; 8192];
    let mut source_right = vec![0.0_f32; 8192];
    for index in 0..source_left.len() {
        source_left[index] = (index as f32 * 0.011).sin() * 0.3;
        source_right[index] = (index as f32 * 0.017).cos() * 0.2;
    }

    let expected: Vec<_> = source_left
        .iter()
        .zip(&source_right)
        .map(|(&left, &right)| sample_by_sample.process_sample(left, right, controls))
        .collect();

    let mut actual_left = source_left;
    let mut actual_right = source_right;
    let mut start = 0;
    for block_size in [1, 7, 31, 64, 127, 512, 1024, 2048] {
        if start == actual_left.len() {
            break;
        }
        let end = (start + block_size).min(actual_left.len());
        blocked.process_block(
            &mut actual_left[start..end],
            &mut actual_right[start..end],
            controls,
        );
        start = end;
    }
    if start < actual_left.len() {
        blocked.process_block(
            &mut actual_left[start..],
            &mut actual_right[start..],
            controls,
        );
    }

    for (index, frame) in expected.into_iter().enumerate() {
        assert_eq!(actual_left[index].to_bits(), frame.left.to_bits());
        assert_eq!(actual_right[index].to_bits(), frame.right.to_bits());
    }
}
