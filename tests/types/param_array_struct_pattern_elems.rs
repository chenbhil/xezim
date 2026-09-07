//! IEEE 1800-2017 §6.20.2 + §10.9.2: an unpacked-array parameter whose
//! ELEMENTS are themselves assignment patterns.
//!
//!   localparam cfg_t A [3] = '{ '{4,2'd2}, '{5,2'd1}, '{6,2'd3} };
//!
//! Every element read back 0. `register_array_param` evaluated each element
//! with `eval_init_for_width`, which bottoms out in `eval_const_expr_val` —
//! and a bare `'{...}` there has no type context to lay itself out against.
//! The SCALAR form (`localparam cfg_t C = '{4,2'd2}`) was always correct
//! because it goes through `pack_struct_const_value`, which is handed the
//! declared type; the array-element path never called it.
//!
//! The failure was element-type-driven, not storage-driven: the same
//! declaration with integral elements (`'{4,5,6}`) worked, and so did a
//! struct array whose elements were written as integer literals
//! (`'{34'h1, 34'h2}`) — only a nested PATTERN element was lost. Both packed
//! and unpacked struct element types are affected, and so is `parameter` as
//! well as `localparam`.

use xezim::simulate;

const SRC: &str = r#"
module tb;
  typedef struct packed { bit [31:0] s; bit [1:0] x; } p_t;
  typedef struct        { int unsigned s; bit [1:0] x; } u_t;

  localparam p_t LP [3] = '{ '{4,2'd2}, '{5,2'd1}, '{6,2'd3} };
  localparam u_t LU [3] = '{ '{4,2'd2}, '{5,2'd1}, '{6,2'd3} };
  parameter  p_t PP [3] = '{ '{4,2'd2}, '{5,2'd1}, '{6,2'd3} };
  // Named members, and a `default:` inside the ELEMENT pattern.
  localparam p_t LN [2] = '{ '{s:9, x:2'd1}, '{default:0} };
  // Regression guards for the forms that already worked.
  localparam p_t LI [2] = '{ 34'h13, 34'h15 };   // integer-literal elements
  localparam int LC [3] = '{4, 5, 6};            // integral element type

  int lp0, lp1, lp2, lpx2;
  int lu0, lu1, lu2;
  int pp0, pp2;
  int ln0, ln0x, ln1;
  int li0, li1, lc0, lc2;
  initial begin
    lp0 = LP[0].s; lp1 = LP[1].s; lp2 = LP[2].s; lpx2 = LP[2].x;
    lu0 = LU[0].s; lu1 = LU[1].s; lu2 = LU[2].s;
    pp0 = PP[0].s; pp2 = PP[2].s;
    ln0 = LN[0].s; ln0x = LN[0].x; ln1 = LN[1].s;
    li0 = LI[0].s; li1 = LI[1].s;
    lc0 = LC[0];   lc2 = LC[2];
  end
endmodule
"#;

fn u(sim: &xezim::compiler::Simulator, n: &str) -> u64 {
    sim.get_signal(n)
        .unwrap_or_else(|| panic!("signal not found: {}", n))
        .to_u64()
        .unwrap_or_else(|| panic!("{} not u64-able", n))
        & 0xFFFF_FFFF
}

#[test]
fn packed_struct_pattern_elements_reach_every_index() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(u(&sim, "lp0"), 4);
    assert_eq!(u(&sim, "lp1"), 5);
    assert_eq!(u(&sim, "lp2"), 6);
    // The trailing member must land too, not just the first.
    assert_eq!(u(&sim, "lpx2"), 3);
}

#[test]
fn unpacked_struct_pattern_elements_reach_every_index() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(u(&sim, "lu0"), 4);
    assert_eq!(u(&sim, "lu1"), 5);
    assert_eq!(u(&sim, "lu2"), 6);
}

#[test]
fn parameter_keyword_behaves_like_localparam() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(u(&sim, "pp0"), 4);
    assert_eq!(u(&sim, "pp2"), 6);
}

#[test]
fn named_and_default_element_patterns_resolve() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    assert_eq!(u(&sim, "ln0"), 9);
    assert_eq!(u(&sim, "ln0x"), 1);
    assert_eq!(u(&sim, "ln1"), 0);
}

#[test]
fn integral_and_literal_element_forms_are_unchanged() {
    let sim = simulate(SRC, 100).expect("simulate failed");
    // 34'h13 = s:4 x:3, 34'h15 = s:5 x:1
    assert_eq!(u(&sim, "li0"), 4);
    assert_eq!(u(&sim, "li1"), 5);
    assert_eq!(u(&sim, "lc0"), 4);
    assert_eq!(u(&sim, "lc2"), 6);
}
