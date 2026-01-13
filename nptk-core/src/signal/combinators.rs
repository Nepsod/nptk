use crate::signal::zip::ZipSignal;
use crate::signal::{BoxedSignal, Signal};

/// Combine two signals into a tuple signal.
///
/// The resulting signal emits a new value whenever either of the source signals changes.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_core::signal::{Signal, combinators::zip, state::StateSignal};
///
/// let a = StateSignal::new(1);
/// let b = StateSignal::new(2);
/// let combined = zip(a.dyn_clone(), b.dyn_clone()); // Signal<(i32, i32)>
/// ```
pub fn zip<A: 'static + Clone, B: 'static + Clone>(
    signal_a: BoxedSignal<A>,
    signal_b: BoxedSignal<B>,
) -> ZipSignal<A, B> {
    ZipSignal::new(signal_a, signal_b)
}
