//! §8.25/§20.7: the array query functions over a CLASS type parameter
//! answer from the type it binds, default or explicit: `$left(T)` of
//! `C#(2, logic [1:4])` is 1. They answered as if `T` were a 1-bit scalar
//! (`$bits(T)` was right). Expected values from the reference simulator,
//! except where noted.
use xezim::simulate;

fn messages(src: &str) -> Vec<String> {
    let sim = simulate(src, 1_000).expect("simulate failed");
    sim.output.iter().map(|o| o.message.clone()).collect()
}

fn assert_line(msgs: &[String], want: &str) {
    assert!(
        msgs.iter().any(|m| m == want),
        "missing {want:?} in {msgs:?}"
    );
}

const SRC: &str = r#"
typedef logic [0:3] a_t;
typedef logic [3:0] ua_t [0:2];
class C #(int W = 4, type T = logic [W+1:2]);
  localparam int CL = $left(T);
  static function string q();
    return $sformatf("%0d %0d %0d %0d %0d %0d %0d %0d", $left(T), $right(T), $high(T), $low(T),
                     $size(T), $increment(T), $dimensions(T), $unpacked_dimensions(T));
  endfunction
  function string m(); return $sformatf("m %0d %0d %0d", $left(T), $right(T), CL); endfunction
endclass
class D #(type U = logic [9:5]) extends C #(6, U);
  function string dm(); return $sformatf("dm %0d %0d", $left(U), $right(U)); endfunction
endclass
class E #(type T = byte);
  static function string e(); return C#(8, T)::q(); endfunction
endclass
module top;
  initial begin
    C #() c0 = new; C #(2, logic [1:4]) c1 = new; D #() d0 = new; D #(bit [0:2]) d1 = new;
    $display("def %s", C#()::q());
    $display("W8 %s", C#(8)::q());
    $display("asc %s", C#(4, logic [1:4])::q());
    $display("td %s", C#(4, a_t)::q());
    $display("p2 %s", C#(4, bit [3:0][7:0])::q());
    $display("sgn %s", C#(4, logic signed [15:8])::q());
    $display("int %s", C#(4, int)::q());
    $display("ua %s", C#(4, ua_t)::q());
    $display("%s / %s", c0.m(), c1.m());
    $display("d0 %s %s", d0.dm(), d0.m());
    $display("d1 %s %s", d1.dm(), d1.m());
    $display("nest %s", E#(logic [2:7])::e());
  end
endmodule
"#;

/// §8.25: the queries over a CLASS type parameter, default or bound, in a
/// static and an instance method, through a subclass's `extends` argument
/// and a nested specialization. They read 0 (`$bits(T)` was right).
/// `W8`: a default type is sized by the specialization's `W` (§8.25,
/// `logic [9:2]`); Verilator 5.052 sizes it by `W`'s default and reads
/// `5 2 5 2 4`. Every other line is Verilator's.
#[test]
fn array_queries_over_a_class_type_parameter() {
    let msgs = messages(SRC);
    for want in [
        "def 5 2 5 2 4 1 1 0",
        "W8 9 2 9 2 8 1 1 0",
        "asc 1 4 4 1 4 -1 1 0",
        "td 0 3 3 0 4 -1 1 0",
        "p2 3 0 3 0 4 1 2 0",
        "sgn 15 8 15 8 8 1 1 0",
        "int 31 0 31 0 32 1 1 0",
        "ua 0 2 2 0 3 -1 2 1",
        "m 5 2 5 / m 1 4 1",
        "d0 dm 9 5 m 9 5 9",
        "d1 dm 0 2 m 0 2 0",
        "nest 2 7 7 2 6 -1 1 0",
    ] {
        assert_line(&msgs, want);
    }
}
