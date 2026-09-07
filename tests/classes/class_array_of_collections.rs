//! IEEE 1800-2017 §7.4.5: a CLASS PROPERTY that is a fixed-size unpacked
//! array whose ELEMENT is a collection.
//!
//!   class sb;
//!     item expected [5][5][$];        // per (in, out) FIFO of expectations
//!   endclass
//!   expected[i][o].push_back(t);      // silently did nothing
//!   expected[i][o].size();            // 0, forever
//!
//! `elaborate_class` could not express the shape. `array_nd_properties`
//! wants constant bounds on every dimension and a queue dimension has none,
//! so it bailed and classification fell back to `effective_dims.first()`:
//! the member became a plain fixed array of SCALARS and the trailing `[$]`
//! was dropped on the floor.
//!
//! It then failed in the worst possible way — quietly, and only half. The
//! OUTER shape was right, so `$size(q)` answered 3 and `foreach (q[i])`
//! iterated three times; only the element collections had no storage at all.
//! A UVM scoreboard built on `expected[in][out][$]` reported every delivered
//! transaction as "nothing pending" and its end-of-test "nothing left in
//! flight" check silently passed on an empty store.
//!
//! Not specific to queues, to 2-D, or to a class-handle element type: an
//! array of dynamic arrays and an array of associative arrays failed the
//! same way, so did 1-D, and so did a queue of `int`. A MODULE-scope array
//! of queues and a plain (non-array) queue property both always worked,
//! which is what isolated it to the per-instance element store.

use xezim::simulate;

const SRC: &str = r#"
module tb;
  class C;
    int q1 [3][$];        // 1-D array of queues
    int q2 [2][2][$];     // 2-D array of queues
    int dy [3][];         // array of dynamic arrays
    int as [3][int];      // array of associative arrays
    int bare [$];         // plain queue property (the control)
    int flat [3];         // plain fixed array   (the control)

    function void go();
      int n = 0;
      q1[1].push_back(7);
      q1[1].push_back(8);
      q1_size = q1[1].size();
      q1_e0   = q1[1][0];
      q1_e1   = q1[1][1];
      q1_popped         = q1[1].pop_front();
      q1_size_after_pop = q1[1].size();
      q1[1][0]   = 55;                 // element WRITE must reach the same store
      q1_written = q1[1][0];

      q2[1][0].push_back(9);
      q2_size = q2[1][0].size();
      q2_e0   = q2[1][0][0];

      dy[1][0] = 7;                    // auto-grow, like any queue element
      dy_e0    = dy[1][0];

      as[1][5] = 7;
      as_num   = as[1].num();
      as_e5    = as[1][5];

      bare.push_back(4);
      bare_size = bare.size();
      flat[2]   = 6;
      flat_v    = flat[2];

      // The OUTER shape must still resolve — it did even while broken, and
      // regressing it is the obvious way to "fix" this wrong.
      shape_size = $size(q1);
      foreach (q1[i]) n++;
      shape_iters = n;
    endfunction
  endclass

  // Results are copied out to module scope: a class PROPERTY is not a
  // signal the harness can read by name.
  int q1_size, q1_e0, q1_e1, q2_size, q2_e0;
  int dy_e0, as_num, as_e5, bare_size, flat_v;
  int q1_size_after_pop, q1_popped, q1_written;
  int shape_size, shape_iters;

  // A module-scope array of queues; always worked, must keep working.
  int MQ [3][$];
  int mq_size;

  C c;
  initial begin
    c = new();
    c.go();
    MQ[1].push_back(3);
    mq_size = MQ[1].size();
  end
endmodule
"#;

fn i(sim: &xezim::compiler::Simulator, n: &str) -> u64 {
    sim.get_signal(n)
        .unwrap_or_else(|| panic!("signal not found: {}", n))
        .to_u64()
        .unwrap_or_else(|| panic!("{} not u64-able", n))
        & 0xFFFF_FFFF
}

fn p(sim: &xezim::compiler::Simulator, name: &str) -> u64 {
    i(sim, name)
}

#[test]
fn push_back_onto_an_array_of_queues_element_lands() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "q1_size"), 2);
    assert_eq!(p(&sim, "q1_e0"), 7);
    assert_eq!(p(&sim, "q1_e1"), 8);
}

#[test]
fn pop_front_and_element_write_reach_the_same_store() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "q1_popped"), 7);
    assert_eq!(p(&sim, "q1_size_after_pop"), 1);
    // The write must be visible to the read — the two used different stores.
    assert_eq!(p(&sim, "q1_written"), 55);
}

#[test]
fn two_dimensional_array_of_queues_resolves_per_element() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "q2_size"), 1);
    assert_eq!(p(&sim, "q2_e0"), 9);
}

#[test]
fn arrays_of_dynamic_and_associative_arrays_resolve_too() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "dy_e0"), 7);
    assert_eq!(p(&sim, "as_num"), 1);
    assert_eq!(p(&sim, "as_e5"), 7);
}

#[test]
fn the_outer_fixed_shape_still_resolves() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "shape_size"), 3);
    assert_eq!(p(&sim, "shape_iters"), 3);
}

#[test]
fn the_forms_that_already_worked_are_unchanged() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(p(&sim, "bare_size"), 1);
    assert_eq!(p(&sim, "flat_v"), 6);
    assert_eq!(i(&sim, "mq_size"), 1);
}
