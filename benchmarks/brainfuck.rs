// An interpreter loop: run a Brainfuck program that prints squares.
fn run(program: &[u8], input: &[u8]) -> Vec<u8> {
    let mut jumps = vec![0usize; program.len()];
    let mut stack = Vec::new();
    for (i, &op) in program.iter().enumerate() {
        match op { b'[' => stack.push(i), b']' => { let open = stack.pop().unwrap(); jumps[open] = i; jumps[i] = open; } _ => {} }
    }
    let (mut tape, mut ptr, mut pc, mut out, mut inp) = (vec![0u8; 30000], 0usize, 0usize, Vec::new(), 0usize);
    while pc < program.len() {
        match program[pc] {
            b'>' => ptr += 1,
            b'<' => ptr -= 1,
            b'+' => tape[ptr] = tape[ptr].wrapping_add(1),
            b'-' => tape[ptr] = tape[ptr].wrapping_sub(1),
            b'.' => out.push(tape[ptr]),
            b',' => { tape[ptr] = input.get(inp).copied().unwrap_or(0); inp += 1; }
            b'[' => if tape[ptr] == 0 { pc = jumps[pc]; },
            b']' => if tape[ptr] != 0 { pc = jumps[pc]; },
            _ => {}
        }
        pc += 1;
    }
    out
}
fn main() {
    // Prints the squares from 0 to 10000 (Daniel B. Cristofani).
    let squares = b"++++[>+++++<-]>[<+++++>-]+<+[>[>+>+<<-]++>>[<<+>>-]>>>[-]++>[-]+>>>+[[-]++++++>>>]<<<[[<++++++++<++>>-]+<.<[>----<-]<]<<[>>>>>[>>>[-]+++++++++<[>-<-]+++++++++>[-[<->-]+[<<<]]<[>+<-]>]<<-]<<-]";
    let mut total = 0usize;
    let mut last = Vec::new();
    for _ in 0..100 { last = run(squares, b""); total += last.len(); }
    println!("{} bytes, last line {:?}", total, String::from_utf8_lossy(&last[last.len() - 6..]).trim());
}
