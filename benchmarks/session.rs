// SPDX-License-Identifier: AGPL-3.0-only
//! Session workflow measurement, not TSF/UI latency or the A13 target benchmark.
use nextime_core::{Engine, Key, Session};
use std::{hint::black_box, time::Instant};

fn main() {
    let mut session = Session::new(Engine::bundled(), 8).unwrap();
    let iterations = 2000_usize;
    println!(
        "session; OS={}; arch={}; dictionary=bundled62; iterations={iterations}; warmup=100; workflow=5_text+convert+next+enter",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let mut workflow = || {
        for text in ["h", "a", "s", "h", "i"] {
            black_box(session.event(Key::Text(text)).unwrap());
        }
        black_box(session.event(Key::Convert).unwrap());
        black_box(session.event(Key::Next).unwrap());
        let result = session.event(Key::Enter).unwrap();
        assert_eq!(result.committed.as_deref(), Some("箸"));
        black_box(result);
    };
    for _ in 0..100 {
        workflow();
    }
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        workflow();
        times.push(start.elapsed().as_nanos());
    }
    times.sort_unstable();
    let p = |percent: usize| times[(iterations * percent).div_ceil(100) - 1] as f64 / 1_000_000.0;
    println!(
        "eight_event_workflow: p50_ms={:.6}; p95_ms={:.6}; p99_ms={:.6}; max_ms={:.6}",
        p(50),
        p(95),
        p(99),
        *times.last().unwrap() as f64 / 1_000_000.0
    );
}
