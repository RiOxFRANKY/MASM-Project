# MASM Maze

`MAZE.ASM` is a 16-bit DOS maze game with 10 levels.

- `#` is a wall, `@` is the player, `E` is the exit.
- Move with the arrow keys. Press ESC to quit.
- Reach `E` to go to the next level.

## Build (MASM + LINK, e.g. in DOSBox)

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

Or with JWasm: `jwasm -mz MAZE.ASM`
