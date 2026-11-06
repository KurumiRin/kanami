//! Stops the main run loop on SIGINT / SIGTERM / SIGHUP so [`run`](crate::run) takes its normal
//! cleanup path.
//!
//! It uses a libdispatch signal source on the main queue to avoid unsafe calls inside a signal handler.

use std::ffi::{c_int, c_void};
use std::sync::Once;

use crate::ffi::*;

const SIGHUP: c_int = 1;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SIG_IGN: usize = 1;

extern "C" fn stop(_context: *mut c_void) {
    crate::nsapp::stop();
}

pub fn stop_on_signals() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        for signum in [SIGHUP, SIGINT, SIGTERM] {
            // Ignore the default action (exiting immediately) so the dispatch source receives the signal instead
            signal(signum, SIG_IGN);
            let source = dispatch_source_create(
                &raw const _dispatch_source_type_signal,
                signum as usize,
                0,
                &raw const _dispatch_main_q,
            );
            dispatch_source_set_event_handler_f(source, stop);
            dispatch_resume(source);
        }
    });
}
