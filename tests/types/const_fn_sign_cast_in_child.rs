//! IEEE 1800-2017 §6.24.1 + §13.4.3: a SIGN CAST inside a constant function.
//!
//!   function automatic integer unsigned idx_width (input integer unsigned n);
//!     return (n > 32'd1) ? unsigned'($clog2(n)) : 32'd1;
//!   endfunction
//!   localparam int W = p::idx_width(5);   // 3
//!
//! `unsigned'(e)` / `signed'(e)` are lowered by the parser onto `$unsigned` /
//! `$signed`, and `const_fn_expr_supported` — the strict allow-list that
//! decides whether a constant function may be evaluated at elaboration —
//! admitted only `$clog2` and `$bits`. One sign cast anywhere in the body
//! bailed the WHOLE evaluation.
//!
//! At top level the parameter then fell back to sim-time resolution and came
//! out right, so the defect only showed in an instantiated CHILD, where the
//! fallback yields 0. That asymmetry is what made it look like a scoping bug.
//! It is not: the cast is the only factor, and the same function without one
//! was always correct in both scopes.
//!
//! `$unsigned`/`$signed` only reinterpret signedness — no width is resolved
//! and no type table is consulted — so `eval_const_expr_val` computes them
//! from `params` alone, which is the bar the allow-list guards.
//!
//! The real-world bite: `cf_math_pkg::idx_width` is written exactly like the
//! function above, so every `typedef logic [cf_math_pkg::idx_width(N)-1:0]`
//! inside a submodule became `[-1:0]` — one bit instead of three. In a router
//! arbiter that silently truncated the round-robin winner index, so a grant to
//! input 4 became a grant to input 0 and nothing was ever forwarded.

use xezim::simulate;

const SRC: &str = r#"
package p;
  function automatic integer unsigned viacall (input integer unsigned n);
    return unsigned'(n);
  endfunction
  function automatic integer unsigned viasigned (input integer unsigned n);
    return signed'(n);
  endfunction
  // The cf_math_pkg::idx_width shape: a sign cast in one ternary arm.
  function automatic integer unsigned idx_width (input integer unsigned n);
    return (n > 32'd1) ? unsigned'($clog2(n)) : 32'd1;
  endfunction
  // Same function WITHOUT a cast — the control that always worked.
  function automatic integer unsigned nocast (input integer unsigned n);
    return $clog2(n);
  endfunction
endpackage

module child ();
  localparam int C_VIACALL   = p::viacall(7);
  localparam int C_VIASIGNED = p::viasigned(7);
  localparam int C_IDXW      = p::idx_width(5);
  localparam int C_NOCAST    = p::nocast(5);
  localparam int C_DIRECT    = unsigned'(32'd7);
  // The width the bug actually corrupted: a range dimension driven by the
  // constant function, one level down.
  typedef logic [p::idx_width(5)-1:0] idx_t;
  idx_t wide;
  int c_viacall, c_viasigned, c_idxw, c_nocast, c_direct, c_bits;
  initial begin
    c_viacall   = C_VIACALL;
    c_viasigned = C_VIASIGNED;
    c_idxw      = C_IDXW;
    c_nocast    = C_NOCAST;
    c_direct    = C_DIRECT;
    c_bits      = $bits(idx_t);
    wide        = 3'd4;
  end
endmodule

module tb;
  localparam int T_VIACALL = p::viacall(7);
  localparam int T_IDXW    = p::idx_width(5);
  int t_viacall, t_idxw;
  child u ();
  initial begin
    t_viacall = T_VIACALL;
    t_idxw    = T_IDXW;
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

#[test]
fn sign_cast_in_a_const_function_resolves_in_a_child_module() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(i(&sim, "u.c_viacall"), 7);
    assert_eq!(i(&sim, "u.c_viasigned"), 7);
}

#[test]
fn idx_width_shape_resolves_in_a_child_module() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(i(&sim, "u.c_idxw"), 3);
    // The control: same body, no cast.
    assert_eq!(i(&sim, "u.c_nocast"), 3);
}

#[test]
fn a_range_dimension_from_that_function_is_three_bits_wide() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(i(&sim, "u.c_bits"), 3);
    // A 1-bit `idx_t` would have truncated 4 to 0.
    assert_eq!(i(&sim, "u.wide"), 4);
}

#[test]
fn the_top_level_scope_still_agrees() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(i(&sim, "t_viacall"), 7);
    assert_eq!(i(&sim, "t_idxw"), 3);
    assert_eq!(i(&sim, "u.c_direct"), 7);
}
