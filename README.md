# MASM Maze

`MAZE.ASM` is a 16-bit DOS maze game with 3 levels on a 25x25 grid.

- `#` is a wall, `@` is the player, `E` is the exit.
- Move with the arrow keys (or W A S D). Press ESC to quit.
- Reach `E` to go to the next level.

## Build (MASM + LINK, e.g. in DOSBox)

```
masm MAZE.ASM;
link MAZE.OBJ;
MAZE
```

Or with JWasm: `jwasm -mz MAZE.ASM`

# Problem Statement
Write a MASM program to draw a simple maze using the # character to represent walls and empty spaces for paths. Place a player character (e.g., @) at a defined starting point in the maze. Allow the user to navigate the maze using the arrow keys (Up, Down, Left, Right) by handling keyboard interrupts. Ensure the player cannot move through walls (#) and can only move along valid paths. Continuously update and store the player's current position, and refresh the maze view after each move