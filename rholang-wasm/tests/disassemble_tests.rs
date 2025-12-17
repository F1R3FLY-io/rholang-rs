use rholang_wasm::{disassemble, eval};

#[test]
fn disassemble_basic_program_returns_nonempty() {
    let input = "1 + 2";
    let disasm = disassemble(input);
    assert!(
        !disasm.trim().is_empty(),
        "disassembly should not be empty for arithmetic program"
    );
}

#[test]
fn disassemble_handles_empty_input_and_matches_eval_nil() {
    let input = "   \n\t";
    let disasm = disassemble(input);
    let output = eval(input);
    // For empty sources, the compiler emits an empty process; ensure we render it.
    assert!(
        disasm.contains("<empty>") || disasm.contains("Nil"),
        "disassembly should mention empty/Nil for empty input, got: {disasm}"
    );
    assert_eq!("Nil", output);
}

#[test]
fn disassemble_complex_structure_contains_instructions() {
    let input = "new x in { x!(42) | for (y <- x) { y } }";
    let disasm = disassemble(input);
    assert!(
        disasm.contains("SEND") || disasm.contains("RECV") || disasm.contains("TELL") || disasm.contains("ASK"),
        "expected disassembly to include send/receive instructions, got: {disasm}"
    );
}