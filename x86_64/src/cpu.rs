// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::time;

/// Hardware hint in tight loops for hyperthreads to
/// get access to compute.
pub fn relax() {
    unsafe {
        core::arch::x86_64::_mm_pause();
    }
}

/// Returns the clock frequency of the current CPU in Hertz.
pub fn frequency() -> u128 {
    const DEFAULT_HZ: u128 = 2_000_000_000;
    DEFAULT_HZ
}

fn rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn pause(duration: time::Duration) {
    const NANOS_PER_SEC: u128 = 1_000_000_000;
    let ns = duration.as_nanos();
    let cycles = ns * frequency() / NANOS_PER_SEC;
    let start = u128::from(rdtsc());
    let end = u64::try_from(start.checked_add(cycles).unwrap()).unwrap();
    while rdtsc() < end {
        relax();
    }
}
