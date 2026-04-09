# Syntax highlighting for [`micro`](https://micro-editor.github.io/)

Drop the YAML file into your micro syntax folder:

```sh
mkdir -p ~/.config/micro/syntax
cp rpu.yaml ~/.config/micro/syntax/
```

Open any `.rpu` file in `micro` and you should see:

| Token       | Group              | Example                                            |
| ----------- | ------------------ | -------------------------------------------------- |
| Instruction | `statement`        | `put`, `copy`, `jump`, `add`, `halt`               |
| Register    | `type`             | `gp0`, `ans`, `dvc`, `out`, `pc`, `sp`, `zero`     |
| Number      | `constant.number`  | `7`, `100`, `258`                                  |
| Label       | `special`          | `.LOOP`, `.DONE`                                   |
| Comment     | `comment`          | `# this is a comment` (must start in column 0)     |

The exact colors come from your active micro colorscheme — try
`> set colorscheme monokai` (or `solarized`, `gruvbox-tc`, etc.) if
the defaults are bland.

## Comment syntax is strict

The RPU assembler only accepts `#` or `;` comments at the very start
of a line. **No inline comments**, **no leading whitespace** before
the `#`. The highlighter mirrors this — if a comment isn't lit up,
it's a sign the assembler will silently treat it as code (and
probably fail to parse).
