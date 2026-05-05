# Bundled Test ROMs

ROMs in this directory are used by integration tests and manual QA.

## nestest.nes

- **Author:** Kevin Horton (kevtris)
- **License:** Public domain (community consensus, used by every major NES emulator)
- **Source:** http://nickmass.com/images/nestest.nes
- **Purpose:** CPU instruction validation. The canonical reference for "does
  the 6502 core execute every documented opcode correctly?"

## <homebrew>.nes

(Added in plan Task 12 — a CC0 / public-domain homebrew demo so the user
can see something move on the screen, not just CPU-test patterns.)

## Commercial ROMs

Commercial NES ROMs (Super Mario Bros, Zelda, Contra, etc.) are NOT bundled
here for copyright reasons. To run one, point glyph8 at a ROM file you
legally own:

    glyph8 path/to/your.nes
