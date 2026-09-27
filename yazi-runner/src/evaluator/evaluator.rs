use compact_str::CompactString;
use tokio::task;
use yazi_binding::Scope;
use yazi_shared::data::Data;

use crate::{Runner, evaluator::{EvaluateHandle, EvaluateJob}};

impl Runner {
	pub fn evaluate(
		&'static self,
		name: CompactString,
		scope: Scope,
		bytes: Vec<u8>,
		arg: Data,
	) -> EvaluateHandle {
		let scope = scope.child();
		let job = EvaluateJob { runner: self, scope: scope.clone(), name, bytes, arg };

		EvaluateHandle::new(scope, task::spawn_blocking(move || job.eval()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn scope_child_cancel_propagates() {
		let parent = Scope::new();
		let child = parent.child();
		assert!(!child.is_cancelled());
		parent.cancel();
		assert!(child.is_cancelled());
	}

	#[tokio::test]
	async fn evaluate_cancelled_scope_terminates_promptly() {
		// Real `Runner::evaluate → EvaluateJob` path: the job installs an
		// mlua instruction hook (every 2000 instrs) that errors once the scope
		// is cancelled, plus a `select!` on `scope.cancelled()`. This test
		// proves a RUNNING Lua body is interrupted: it waits for the Lua body
		// to enter execution (ready marker), cancels the PARENT scope, then
		// awaits the REAL evaluation JoinHandle under a bounded timeout and
		// asserts the body never reached its natural end.
		//
		// Failsafe: an unbounded `while true do end` would hang `cargo test`
		// forever if the hook ever regressed (aborting a `spawn_blocking`
		// JoinHandle cannot stop an already-running closure). The Lua loop
		// below therefore ALSO polls a stop marker and exits naturally when
		// it appears, writing a natural-finish marker first. The test fails
		// if that marker exists: success requires the hook to win the race.
		crate::init_tests();

		static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
		let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let tag = format!("yazi-task016-cancel-{}-{seq}", std::process::id());
		let mut ready = std::env::temp_dir();
		ready.push(format!("{tag}.ready"));
		let mut stop = std::env::temp_dir();
		stop.push(format!("{tag}.stop"));
		let mut natural = std::env::temp_dir();
		natural.push(format!("{tag}.natural"));
		for p in [&ready, &stop, &natural] {
			let _ = std::fs::remove_file(p);
		}

		fn lua_literal(s: &str) -> String {
			let mut out = String::with_capacity(s.len() + 2);
			out.push('"');
			for c in s.chars() {
				match c {
					'\\' => out.push_str("\\\\"),
					'"' => out.push_str("\\\""),
					'\n' => out.push_str("\\n"),
					'\r' => out.push_str("\\r"),
					c => out.push(c),
				}
			}
			out.push('"');
			out
		}

		// Signal readiness with standard Lua `io` before the loop, so
		// cancellation provably lands after execution started (a pre-cancelled
		// scope could otherwise take the `select!` branch without ever
		// demonstrating the instruction hook). The loop body performs ample
		// instructions per iteration for the hook, and always stays killable
		// via the stop marker even if cancellation regresses.
		let bytes = format!(
			"local f = assert(io.open({}, \"w\")); f:write(\"ready\"); f:close(); \
			 while true do \
			 local stop = io.open({}, \"r\"); \
			 if stop then stop:close(); break; end; \
			 end; \
			 local done = assert(io.open({}, \"w\")); done:write(\"natural\"); done:close()",
			lua_literal(&ready.to_string_lossy()),
			lua_literal(&stop.to_string_lossy()),
			lua_literal(&natural.to_string_lossy())
		)
		.into_bytes();

		let runner: &'static Runner = &crate::RUNNER;
		let scope = Scope::new();
		let mut handle =
			runner.evaluate("task016-cancel".into(), scope.clone(), bytes, Data::Nil);

		// Deterministic readiness: poll for the marker instead of sleeping an
		// arbitrary duration.
		let start = std::time::Instant::now();
		loop {
			if ready.exists() {
				break;
			}
			if start.elapsed() > std::time::Duration::from_secs(10) {
				panic!("Lua body never started; marker missing: {}", ready.display());
			}
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}

		// Cancel the PARENT scope passed to `Runner::evaluate`; it must
		// propagate to the child Scope created inside `evaluate`, and the
		// instruction hook must abort the loop before the failsafe below can
		// release it naturally.
		scope.cancel();

		// Failsafe only: release the loop naturally if the hook regresses, so
		// a broken cancellation mechanism fails this test instead of hanging
		// the whole test process. Aborted on the success path.
		let stop_ = stop.clone();
		let failsafe = tokio::spawn(async move {
			tokio::time::sleep(std::time::Duration::from_secs(2)).await;
			let _ = std::fs::write(&stop_, b"stop");
		});

		// Await the REAL evaluation JoinHandle (test-only `join` seam borrows
		// `&mut self` and awaits `&mut self.handle`). `yield_now` alone would
		// not prove job completion; timing alone is only a deadlock guard.
		let outcome =
			tokio::time::timeout(std::time::Duration::from_secs(10), handle.join()).await;
		failsafe.abort();
		outcome
			.expect("cancelled evaluate must terminate the real job")
			.expect("evaluation JoinHandle must not report a JoinError");
		assert!(scope.is_cancelled());
		assert!(ready.exists(), "Lua body must have started before cancellation");
		assert!(
			!natural.exists(),
			"Lua loop exited via the failsafe stop marker: the instruction-hook cancellation did not interrupt it"
		);
		for p in [&ready, &stop, &natural] {
			let _ = std::fs::remove_file(p);
		}
	}
}
