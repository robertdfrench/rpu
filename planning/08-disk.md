# Stage 8 — Disk device (stretch)

## Why this is here (last)

The goal: a program can persist data to a real file on the host
filesystem and read it back later. This is the "stretch" item from
the original brainstorm — nothing else depends on it, and it
introduces the only piece of host-environment state in the whole
project (a real file handle), so it's worth deferring until the
rest is solid.

Depends on stage 4 (device table). Otherwise standalone.

## Design decisions

### Stream, not disk

A real disk has seeks, sectors, and a notion of "where the head
is". That's a lot of mechanism for an educational tool, and the
first version doesn't need any of it. Instead, model the device as
a **stream**:

- Writes append bytes to the end of the file.
- Reads consume bytes from the front of the file.
- The "head" is just the current read offset, persisted in memory
  for the duration of the run (not on disk — fresh runs start
  reading from the front).

This is closer to a tape than a disk, but it's the smallest model
that lets students do persistence and round-tripping. The name
"disk" is fine — it's the metaphor students will care about, and
we can add seek later if a real example needs it.

### CLI flag

Specify the file path on the command line:

```sh
rpu examples/16.save_counter.s --disk save.bin
```

If `--disk` is not provided, the device is still present but reads
return `None` (so `rdy` will be 0) and writes return an error.
That makes "disk not attached" a runtime-observable thing instead
of a load-time crash, which feels right.

`clap` is already a dep so this is a one-liner.

### Reading

Reads return one byte at a time, wrapped in `u16`. When the read
offset reaches the end of the file, further reads return `None`
(setting `rdy = 0`, same convention as `LineInput` from stage 6).

The file is read lazily — open it on first access, not at startup.
That way "no file yet" and "file exists but empty" both Just Work.

### Writing

Writes go to a buffered `BufWriter<File>` opened in append mode.
**Flush on `halt` and on `Drop`.** Forgetting this is the most
common bug in this kind of code, so write a test for it (see
below).

### Bytes, not u16s

The `Device` trait deals in `u16`. For the disk, we'll only use
the low 8 bits and ignore the high 8 on write; on read we'll
always return `Some(byte as u16)`. This matches what `LineInput`
does and keeps the on-disk format honest (one byte per write).

If a future use case needs 16-bit values, that's a protocol
decision the *program* makes (write the high byte then the low
byte), not the device.

## What to change

### 1. New `Disk` device

```rust
// src/devices/disk.rs (or in src/devices.rs)
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use super::{Device, Error};

pub struct Disk {
    path: Option<PathBuf>,
    reader: Option<BufReader<File>>,
    writer: Option<BufWriter<File>>,
    read_offset: u64,
}

impl Disk {
    pub fn detached() -> Self {
        Self { path: None, reader: None, writer: None, read_offset: 0 }
    }

    pub fn attach(path: PathBuf) -> Self {
        Self { path: Some(path), reader: None, writer: None, read_offset: 0 }
    }

    fn ensure_writer(&mut self) -> Result<&mut BufWriter<File>, Error> {
        if self.writer.is_none() {
            let path = self.path.as_ref()
                .ok_or_else(|| Error::Write("disk: not attached".into()))?;
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| Error::Write(format!("disk open: {e}")))?;
            self.writer = Some(BufWriter::new(f));
        }
        Ok(self.writer.as_mut().unwrap())
    }

    fn ensure_reader(&mut self) -> Result<&mut BufReader<File>, Error> {
        if self.reader.is_none() {
            let path = self.path.as_ref()
                .ok_or_else(|| Error::Read("disk: not attached".into()))?;
            let mut f = File::open(path)
                .map_err(|e| Error::Read(format!("disk open: {e}")))?;
            f.seek(SeekFrom::Start(self.read_offset))
                .map_err(|e| Error::Read(format!("disk seek: {e}")))?;
            self.reader = Some(BufReader::new(f));
        }
        Ok(self.reader.as_mut().unwrap())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        if let Some(w) = self.writer.as_mut() {
            w.flush().map_err(|e| Error::Write(format!("disk flush: {e}")))?;
        }
        Ok(())
    }
}

impl Device for Disk {
    fn write(&mut self, value: u16) -> Result<(), Error> {
        let byte = (value & 0xff) as u8;
        let w = self.ensure_writer()?;
        w.write_all(&[byte])
            .map_err(|e| Error::Write(format!("disk write: {e}")))?;
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u16>, Error> {
        if self.path.is_none() {
            return Ok(None);
        }
        let r = self.ensure_reader()?;
        let mut buf = [0u8; 1];
        match r.read(&mut buf) {
            Ok(0) => Ok(None),                       // EOF
            Ok(_) => {
                self.read_offset += 1;
                Ok(Some(buf[0] as u16))
            }
            Err(e) => Err(Error::Read(format!("disk read: {e}"))),
        }
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
```

A few subtleties:

- **Reader and writer are separate** because they're in different
  modes (read vs append). They share the path but not the handle.
- **`read_offset` is tracked manually** so that if the writer
  appends and we re-open the reader later in the same run, we
  resume from where we left off (not from EOF). For the simple
  cases this stage targets, that doesn't actually come up — but
  it's the kind of thing that's cheap to do right and expensive
  to fix later.
- **`Drop` flushes** but ignores errors. The explicit `halt` flush
  (below) is the one that should surface errors.

### 2. CLI

Add to the `clap` args:

```rust
#[derive(clap::Parser)]
struct Args {
    /// Source file to load.
    source: PathBuf,

    /// Optional file to use as the disk device.
    #[arg(long)]
    disk: Option<PathBuf>,
}
```

In `main`:

```rust
let args = Args::parse();
let disk = match args.disk {
    Some(path) => Disk::attach(path),
    None       => Disk::detached(),
};
let mut computer = Computer::new_with_disk(disk);
```

### 3. Device table

`Devices` (from stages 3, 5, 6) gets the disk at index 5:

```rust
pub struct Devices {
    pub lcd0: Lcd,
    pub lcd1: Lcd,
    pub tty: Tty,
    pub line_input: LineInput,
    pub screen: Screen,
    pub disk: Disk,
}

impl Devices {
    pub fn as_slice(&mut self) -> [&mut dyn Device; 6] {
        [
            &mut self.lcd0,        // 0
            &mut self.lcd1,        // 1
            &mut self.tty,         // 2
            &mut self.line_input,  // 3
            &mut self.screen,      // 4
            &mut self.disk,        // 5
        ]
    }
}
```

### 4. Flush on halt

In `core.rs::halt`, or — better — in whatever wrapper layer
manages the device table on `halt` (since `core` shouldn't know
about `Disk` specifically), call `devices.disk.flush()` before
returning. Surface errors so the user sees them.

If you don't want `Computer::step` / `Computer::halt` to know
about disks specifically, generalize: add an optional
`flush(&mut self)` to the `Device` trait with a default no-op
impl, and call it on every device on halt:

```rust
pub trait Device {
    fn write(&mut self, value: u16) -> Result<(), Error>;
    fn read(&mut self) -> Result<Option<u16>, Error>;
    fn flush(&mut self) -> Result<(), Error> { Ok(()) }
}
```

That's clean and future-proof. Recommend doing it that way.

### 5. Examples

`examples/16.save_counter.s`:

```
# Counts from 1 to 5, writing each number to disk and to LCD0.
# Run twice with the same --disk file: the file should grow.

put 5 dvc       # select disk
put 0 gp0       # i = 0

.LOOP
  put 1 gp1
  add gp0 gp1
  put 0 gp0
  copy ans gp0   # i++

  put 5 dvc
  copy gp0 out   # disk.write(i)

  put 0 dvc
  copy gp0 out   # lcd0.write(i)

  put 5 gp1
  sub gp0 gp1
  jump LOOP ans  # if i - 5 == 0, fall through; else loop

halt
```

(Wait — this hits the same "jump if zero" inversion issue as the
echo example. Comment carefully or restructure. Worth a second
example, `17.read_disk.s`, that reads the same file and prints the
bytes to LCD0, demonstrating round-trip persistence.)

## Test plan

### Unit tests on `Disk` (use `tempfile` crate as a dev-dep)

- [ ] Detached disk: `read()` returns `None`; `write()` returns
      `Error::Write`.
- [ ] Attached disk, file doesn't exist: `write(0x41)` creates the
      file with one byte (`0x41`).
- [ ] Attached disk, file exists with `"hi"`: `read()` yields
      `Some(b'h' as u16)`, then `Some(b'i')`, then `None`.
- [ ] After `write`, dropping the `Disk` flushes (file on disk
      contains the byte).
- [ ] Round-trip: write 5 bytes, drop, re-attach to the same file,
      read 5 bytes, get them back in order.

### Integration test

- [ ] Run `examples/16.save_counter.s` twice in the same process
      against a tempfile, halt between, and assert the file grew
      from 5 bytes to 10.

### Render test

- [ ] (Optional) Add a small status line to the TUI showing
      whether disk is attached and how many bytes it's read/written
      this session. Render-test that.

## Done when

- [ ] `Disk` device exists, implements `Device` (and ideally a
      default `flush()` on the trait).
- [ ] `--disk PATH` CLI flag wires it up; absent flag = detached.
- [ ] Halting the computer flushes pending writes.
- [ ] Two examples (write + read-back) demonstrate persistence.
- [ ] Unit and integration tests above pass.

## Notes for future me

- **Don't add seek yet.** It's tempting because it'd make the
  device feel more "disk-y", but seeking is a protocol expansion
  (high byte = opcode, etc., like the screen device) and you
  haven't run into a program that needs it. Add it the first time
  you do.
- **Don't add a "size" query.** Same reason. The user is welcome
  to count bytes themselves until there's a real use case.
- **Be careful with the example assembly.** The flush-on-halt
  behavior is the load-bearing thing here; if a student forgets to
  `halt` (or your example forgets), partial writes will be lost
  and they'll have a bad time. The `Drop` impl is the safety net
  but the explicit `halt` flush is the "did this work?" answer.
- The `flush` trait method, once added, is also useful for the
  `Tty` and screen devices in principle (e.g. forcing the TUI to
  refresh on halt). Don't pre-implement; add when needed.
