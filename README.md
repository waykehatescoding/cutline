# Cutline

A fast, lightweight non-linear video editor for Windows, built in Rust.

## Install

Download the installer from Releases, or build from source:

```
cargo build --release
```

ffmpeg and ffprobe must be on PATH or next to `cutline.exe`.

## Usage

Run `cutline`, or `cutline <file>` to start with a file. Enable Explorer integration from the Help menu to get "Edit this video" on right click.

| Shortcut | Action |
| --- | --- |
| Space | Play / pause |
| S | Split at playhead |
| Delete | Delete selection |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+S | Save project |
| Ctrl+O | Open project |
| Ctrl+E | Export |
| Left / Right | Step one frame |
