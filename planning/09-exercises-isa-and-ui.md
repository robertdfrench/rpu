# Stage 9 — New exercises, ISA extensions, and UI improvements

This document covers three areas identified during a comprehensive
review of the codebase (April 2026):

1. New exercises (examples 12–25) that fill gaps in the existing
   curriculum.
2. Proposed ISA extensions that unlock more advanced exercises.
3. UI improvements (beyond what's already planned) that would make
   exercises more practical and enjoyable.

The new exercises have already been written to `examples/` — see
those files directly. The rest of this document is the design
rationale and the feature proposals.


## 1. New exercises (12–25)

### Why these exercises exist

The original 11 examples cover: basic output, arithmetic, errors,
jumping, noop/pc observation, loops, memory writes, fibonacci
(plain and stack-based), and function calls. The new exercises fill
several gaps:

| #   | File                     | First use of        | Core concept                    |
| --- | ------------------------ | ------------------- | ------------------------------- |
| 12  | `read_memory.rpu`        | `read`              | Memory read/write round-trip    |
| 13  | `multiply.rpu`           | `mul` (focused)     | Multiplication                  |
| 14  | `copy_chain.rpu`         | —                   | `copy` is non-destructive       |
| 15  | `swap.rpu`               | —                   | Arithmetic tricks               |
| 16  | `memory_array_sum.rpu`   | —                   | Array + loop + accumulator      |
| 17  | `memory_copy.rpu`        | —                   | Two-pointer memory traversal    |
| 18  | `string_to_tty.rpu`      | TTY device (dvc 2)  | ASCII codes                     |
| 19  | `power.rpu`              | —                   | Repeated multiplication (exp)   |
| 20  | `division.rpu`           | —                   | Repeated subtraction (div)      |
| 21  | `factorial.rpu`          | —                   | Accumulator pattern with mul    |
| 22  | `count_up.rpu`           | —                   | Counting-up loop                |
| 23  | `mult_table.rpu`         | —                   | Single-row table in memory      |
| 24  | `stack_swap.rpu`         | push/pop for swap   | Stack mechanics                 |
| 25  | `add_subroutine.rpu`     | —                   | Manual call/ret (cleaner 11)    |

### The comparison gap

The biggest thing these exercises *don't* cover: comparing two
unknown values. The current ISA has no safe way to ask "is A bigger
than B?" without risking an underflow crash from `sub`. This means
exercises like "find the maximum in an array", "sort", or "GCD"
are impractical.

This is the single strongest argument for adding new opcodes (see
below). With `jumpnz` alone, many comparison-heavy exercises become
feasible but still awkward. With `cmp` or bitwise ops, they become
natural.


## 2. Proposed ISA extensions

Opcodes are ordered by impact: the first three would unlock the
most new exercises and are recommended for implementation soon.
The rest can wait.

### Priority 1: `jumpnz addr cond` — Jump when nonzero

Already proposed as the existing stage 9 (see planning/README.md
line 38). Reiterated here because it's the single most impactful
addition.

**Current problem.** `jump addr cond` jumps when `cond == 0`.
Every "loop while nonzero" pattern requires two jumps:

```
jump .SKIP gp0    ; if gp0 == 0, skip the loop body
jump .LOOP zero   ; unconditional: go back to the top
```

Or the inverted form used in the examples:

```
jump .END gp0     ; if gp0 == 0, exit the loop
jump .LOOP zero   ; unconditional: keep looping
```

With `jumpnz`, the common case is one instruction:

```
jumpnz .LOOP gp0  ; keep looping while gp0 != 0
```

**Exercises this unlocks (or simplifies):**

- Every existing loop exercise gets a cleaner rewrite.
- Polling loops with `rdy` (stage 6) become
  `jumpnz WAIT rdy` — reads naturally as "keep waiting while not
  ready."
- `jumpnz` + `sub` gives a safe-ish comparison for the "is A
  larger than B?" pattern: subtract and check if the *remainder*
  is nonzero. Still can't recover from underflow, but combined
  with a guard check it works for more cases.

### Priority 2: `call label` / `ret` — Proper subroutine support

**Current problem.** Example 11 and 25 manually push a return
address and use `jump reg zero` to return. This works but is
verbose and error-prone (4–5 instructions per call site).

**Proposal.**

```
call reg     ; push PC+4, then set PC = reg
ret          ; pop address from stack, set PC = it
```

`call` takes a register (same shape as `jump`). It pushes the
next instruction's address onto the stack and then jumps. `ret`
pops the top of the stack into PC. Both are single instructions.

Encoding: `call` is a one-register instruction (like `push`).
`ret` is a zero-argument instruction (like `halt`).

**Exercises this unlocks:**

- Clean function calls without boilerplate. Students think about
  *what* the function does, not the calling convention.
- Recursive algorithms: recursive factorial, recursive fibonacci.
- Library patterns: `print_number` subroutine, `multiply_by_10`
  subroutine.
- More complex programs that decompose into subroutines naturally.

**Implementation notes.** `call` needs to push PC+4 (the address
of the instruction *after* the call). But the auto-advance in
`execute_single_instruction` will try to advance PC again after
the instruction completes. So `call` should set PC to `target - 4`
(following the same convention as `jump`), and the auto-advance
will correct it to `target`. Alternatively, skip the auto-advance
for `call` and `ret` the same way `jump` is handled.

Actually, looking at the code, `jump` sets PC directly and the
auto-advance still fires, adding 4. So `jump` backs up by 4 to
compensate. `call` and `ret` would need the same treatment: set
PC to `target - 4`, then the auto-advance brings it to `target`.

For `call`, the pushed return address should be the instruction
after the call: `current_pc + 4`. The caller's PC after return
will be this value minus 4 (due to auto-advance), so the
auto-advance puts it back to `current_pc + 4`. That's correct.

### Priority 3: Bitwise operations — `and`, `or`, `xor`

Same shape as `add`/`sub`/`mul`: two register arguments, result in
`ans`.

**Exercises these unlock:**

- **Masking**: extract specific bits from a value.
- **Even/odd test**: `and gp0 1` → ans is 0 for even, 1 for odd.
  (With a `put 1 gp1; and gp0 gp1; jump .ODD ans` pattern.)
- **Power-of-2 check**: `and x (x-1)` is zero iff x is a power of 2.
  (Currently impossible because `sub x 1` might underflow if x is 0,
  but for nonzero x it works.)
- **Bit extraction**: get the Nth bit of a value with `and` and a
  mask.
- **XOR swap**: `xor a b; xor b a; xor a b` — swap without a temp
  register and without arithmetic overflow concerns.
- **Simple checksums**: XOR all bytes together.

**Why three instructions, not one?** You could argue for just `and`
and derive the rest (NOT via `xor reg 0xFFFF`, OR via De Morgan's).
But each operation is one byte in the encoding and the ISA is
already tiny. Adding all three costs nothing and keeps programs
readable.

### Priority 4: `mod reg reg` — Modulo

`ans = x % y`. Currently requires a full division loop plus
remainder tracking — painful for something as fundamental as "is
this number divisible by 3?"

**Exercises this unlocks:**

- **FizzBuzz**: the classic interview exercise. Without `mod` it's
  impractical.
- **Prime checker**: "is N prime?" requires checking divisibility.
- **Clock arithmetic**: wrap-around counters, circular buffers.
- **Digit extraction**: get the ones digit of a number with `mod 10`.

### Priority 5: `div reg reg` — Integer division

`ans = x / y`. Combined with `mod`, students get both quotient and
remainder. Division is the most tedious operation to implement by
hand (repeated subtraction).

**Exercises this unlocks:**

- Anything involving splitting numbers into parts.
- Base conversion: extract digits with `div 10` and `mod 10`.
- Mean/average: sum then divide by count.

### Priority 6: `shr reg reg` / `shl reg reg` — Bit shifts

Shift the value in the first register right or left by the amount
in the second. Result in `ans`.

**Exercises these unlock:**

- **Efficient multiplication/division by powers of 2**.
- **Bit manipulation**: rotate bits, extract nibbles.
- **Binary representation**: shift-right and mask to print a number
  in binary on the screen device.

### Nice-to-have: `cmp reg reg` — Compare

Sets `ans` to 0 if equal, 1 if first is larger, or some flag
scheme. This would solve the comparison gap completely. However,
`jumpnz` + `sub` with careful guards covers most teaching cases,
and `cmp` introduces a slightly different mental model. Consider
only if the comparison gap is genuinely blocking exercises after
the other extensions ship.

### Nice-to-have: `inc reg` / `dec reg`

Increment or decrement a register in place. Currently requires
3 instructions (`put 1 gp1; add gp0 gp1; copy ans gp0`). The
verbosity is pedagogically clear, but for loops where the counter
is not the point, `inc`/`dec` would reduce clutter. Low priority
because the 3-instruction form is a good teaching moment.


## 3. UI improvements (not in existing planning docs)

These are ordered by how much they would improve the exercise
experience, independent of implementation difficulty.

### 1. Breakpoints (`b` to toggle, `R` to run-to-breakpoint)

**The single most impactful UI improvement.** Students can mark any
source line with a breakpoint, then press `R` to run until a
breakpoint is hit (or the program halts).

**Why it matters.** Currently, watching a loop iterate 99 times
(example 08) requires pressing `n` 99 times. With breakpoints,
students set a breakpoint after the loop and skip straight to the
result. This makes long-running programs practical to debug.

**Exercises it enables.** Every exercise with a loop of more than
~10 iterations becomes practical. The multiplication table
(exercise 23) becomes inspectable: set a breakpoint in the
verification phase and skip the build phase.

**Implementation sketch.** Add `breakpoints: HashSet<u16>` to
`App`. `b` toggles a breakpoint at the current PC. `R` loops
calling `step()` until a breakpoint is hit, the program halts, or
a step limit is reached (to prevent infinite-run hangs).

### 2. Variable-speed auto-run (`r` to toggle, `+`/`-` to adjust)

Press `r` to start auto-stepping at a configurable speed. `+` and
`-` speed up or slow down (e.g., 1, 5, 10, 20, 50 steps/sec).

**Why it matters.** For any program with a loop, watching it
execute in slow motion is far more instructive than pressing `n`
hundreds of times. Students can see registers and memory changing
in real time and spot the moment something goes wrong.

**Implementation sketch.** In the main loop, when auto-run is
active, call `step()` on a timer instead of waiting for keypress.
Use `crossterm::event::poll(timeout)` with a timeout derived from
the speed setting. Key events still fire during auto-run so `r`
can stop it.

### 3. Register change highlighting

When a step executes, briefly highlight (e.g., yellow background
for 0.3 seconds) any register whose value changed.

**Why it matters.** With 8 GP registers + 4 special registers,
it's hard to spot which one just changed. Highlighting makes data
flow visible at a glance, especially in complex exercises like
subroutine calls or multi-register loops.

**Implementation sketch.** Before each step, snapshot all register
values. After the step, diff against the snapshot. Store a
`HashMap<RegisterName, Instant>` of recently-changed registers.
Renderers check the map and apply a highlight style. Clear entries
older than 300ms.

### 4. LCD output history

Show the last N values written to each LCD, not just the most
recent one. The data already exists in `Lcd::history()` — it just
needs rendering.

**Why it matters.** Exercises like "count from 1 to 10 and display
each number" are impossible to verify when only the last value is
visible. With history, students can see the full output sequence.

**Implementation sketch.** Render the last 5–8 values from
`lcd.history()` in a scrollable list below the current 7-segment
display. The LCD pane might need to grow by a few rows.

### 5. Stack visualization pane

A dedicated pane showing the top ~4–8 values on the stack, rendered
as a growing/shrinking column with the stack pointer marked.

**Why it matters.** The stack is currently invisible unless you
scroll to the bottom of the memory pane. Stack-based exercises
(10, 11, 24, 25) would be dramatically easier to follow if students
could see push/pop happening visually.

**Implementation sketch.** Read the memory between `sp` and
`RAM - 2` (the stack region). Display the top N u16 values in a
vertical list. Highlight changes on push/pop. Slot the pane into
the layout — perhaps between the registers and memory panes.

### 6. Run-to-halt key (`H`)

Press `H` to run the program to completion (halt) and show the
final state. Essentially `Computer::run_to_halt()` exposed to the
TUI.

**Why it matters.** "Did my program produce the right answer?"
should not require pressing `n` 200 times. `H` is the verification
shortcut. Combined with reload (re-run from scratch), it enables
a fast edit-verify cycle.

### 7. Hex/decimal toggle (`x`)

Toggle register and memory displays between decimal and hexadecimal.

**Why it matters.** Exercise 18 (string_to_tty) deals with ASCII
codes, and any bit-manipulation exercise works more naturally in
hex. Currently all values are shown in decimal, which obscures bit
patterns.

### 8. Challenge/verification mode

A convention for specifying expected output in source comments:

```
; EXPECT: lcd0=42 lcd1=7
```

After running, the TUI shows a green checkmark or red X. This
turns every exercise into a self-checking problem.

**Why it matters.** Self-grading lets students work independently
without a teacher present. It also makes it easy to add "challenge"
versions of exercises where the student has to fill in missing
code to match the expected output.

### 9. Step counter

Display "Step: 47" somewhere on screen. After halt, show total
steps taken.

**Why it matters.** Enables exercises about algorithm efficiency:
"which version uses fewer steps?" A counting-up loop and a
counting-down loop might take the same number of steps, but a
repeated-subtraction division vs. a hypothetical `div` instruction
shows a dramatic difference.

### 10. Decoded instruction sidebar

Next to each source line in the code pane, show the decoded
instruction: `put 5 gp0` shows `[PUT 0x05 gp0]`. Helps students
understand that the CPU sees bytes, not text.

**Why it matters.** Teaches the fetch-decode-execute cycle
visually. Students can see what the assembler produced and how
labels resolve to addresses.

### 11. Undo / step-back (`u`)

Maintain a snapshot stack (save full state every N steps). Press
`u` to undo the last step and restore the previous state.

**Why it matters.** "Wait, what just happened?" is the most common
question when learning. Being able to step backward to re-examine
a state is incredibly valuable. Implementation is non-trivial
(snapshotting the full machine state is cheap but the UX needs
care), which is why this is lower priority.


## Summary: recommended implementation order

### ISA extensions (by impact)

1. `jumpnz` — already planned, unblocks cleaner loops
2. `call` / `ret` — unblocks recursive and multi-function programs
3. `and` / `or` / `xor` — unblocks bit manipulation and comparison
4. `mod` — unblocks FizzBuzz, prime checking
5. `div` — unblocks base conversion, efficient division
6. `shr` / `shl` — unblocks bit-shift exercises

### UI improvements (by impact)

1. Breakpoints
2. Variable-speed auto-run
3. Register change highlighting
4. LCD output history
5. Stack visualization pane
6. Run-to-halt key
7. Hex/decimal toggle
8. Challenge/verification mode
9. Step counter
10. Decoded instruction sidebar
11. Undo / step-back
