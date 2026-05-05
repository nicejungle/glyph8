# Bundled Test ROMs

ROMs in this directory are used by integration tests and manual QA.

## nestest.nes

- **Author:** Kevin Horton (kevtris)
- **License:** Public domain (community consensus, used by every major NES emulator)
- **Source:** http://nickmass.com/images/nestest.nes
- **Purpose:** CPU instruction validation. The canonical reference for "does
  the 6502 core execute every documented opcode correctly?"

## boing.nes

- **Author:** Brad Smith (rainwarrior)
- **License:** CC BY 4.0 — "you are free to reuse this source code for your own purposes, provided that you include an attribution to me (Brad Smith) in documentation and accessible credits for the work it is used in" (per project README).
- **Source:** https://github.com/bbbradsmith/boingnes/releases/download/1.0/boing.nes
- **Purpose:** Visual sanity check — a recreation of the classic Amiga
  "Boing Ball" demo. Animation begins on frame 1 (no title screen),
  which lets beta users see immediately that the emulator is rendering
  motion correctly.

## Commercial ROMs

Commercial NES ROMs (Super Mario Bros, Zelda, Contra, etc.) are NOT bundled
here for copyright reasons. To run one, point glyph8 at a ROM file you
legally own:

    glyph8 path/to/your.nes
