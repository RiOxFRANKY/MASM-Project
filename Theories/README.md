# Theories — Index

Separate theory files, one per topic (no monolithic doc):

| # | File | Covers |
|---|---|---|
| 01 | [01-MASM-Fundamentals.md](01-MASM-Fundamentals.md) | What MASM is, build pipeline, directives (`.model`, `.stack`, `db`, `proc`, `end`), segments & DS setup, addressing modes, ASCII |
| 02 | [02-Concepts-Used.md](02-Concepts-Used.md) | INT 10h/16h/21h, scancodes vs ASCII, flags & jumps, MUL/AAM, stack, text screen, 2-D arrays, blocking input |
| 03 | [03-How-the-Code-Works.md](03-How-the-Code-Works.md) | Line-by-line walkthrough of `Code/MAZE.ASM`, `getcell` address math, `draw` rendering, data flow |
| 04 | [04-Upgrades.md](04-Upgrades.md) | Bug fixes, code-quality refactors, performance (flicker, `B800h`), feature upgrades, work order |
| 05 | [05-Interview-Questions.md](05-Interview-Questions.md) | 54 Q&A + true/false, grouped by topic |
| 06 | [06-Extraordinary-Addons.md](06-Extraordinary-Addons.md) | Procedural mazes, BFS solver, colour video, INT 09h hooking, sound/timer, deep-system extras |
