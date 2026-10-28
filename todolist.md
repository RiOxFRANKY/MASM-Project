# Maze Project TODO

Implementation status for `Code/MAZE.ASM`. The game now generates 10x10, 20x20, and 30x30 mazes, supports one- and two-player games, and uses BFS for hints and scoring.

## Requested Work

### 1. Maze dimensions and display

- [x] Use a square grid with level sizes: level 1 = 10x10, level 2 = 20x20, level 3 = 30x30.
- [x] Define `total_blocks` for a level of size `a x a` as `a²` (100, 400, and 900 blocks respectively).
- [x] Adapt the display for 30x30 mazes with a vertical viewport; show two side-by-side copies in two-player mode.
- [x] Use grid size as the difficulty progression; no additional difficulty modifiers are included.

### 2. Generate mazes at runtime

- [x] Replace the hardcoded maze layouts with a runtime generator.
- [x] Generate a connected maze with a valid start and exit using an explicit data-segment stack.
- [x] Seed generation from BIOS ticks by default; `FIXED_SEED` enables repeatable test layouts.
- [x] Run BFS on each generated maze and regenerate if the exit is not reachable from the start.
- [x] Reuse BFS distances for hints and shortest-path scoring.

### 3. Add the start menu and one-player flow

- [x] Add a start screen with one- and two-player choices.
- [x] In one-player mode, let the player choose a starting level.
- [x] Continue from the chosen level through level 3.

### 4. Make hints temporary and charge for each use

- [x] Keep the BFS hint visible only until that player makes a legal move.
- [x] Hide the current hint after that move.
- [x] Charge 20 points for every hint request.
- [x] Rejected wall moves neither clear hints nor consume turns.

### 5. Add two-player turn-based mode

- [x] Ask for both names only in two-player mode; blank names default to `P1` and `P2`.
- [x] Show side-by-side views of the same generated maze and each player's state.
- [x] Alternate turns after valid moves and show the active player.
- [x] Track each player's position, hint state, visited cells, and score independently.
- [x] Start two-player games at level 1; award 20 points to the higher-scoring player after both finish each level.
- [x] Show names, total scores, and level wins on the final scoreboard.

### 6. Track and display scores

- [x] Track distinct cells visited by each player; revisits do not add to the count.
- [x] Determine shortest-path cells with BFS, counting start and exit (distance in moves + 1).
- [x] Apply the requested level score formula:

  `level score = a² - (distinct_blocks_travelled - distinct_blocks_in_shortest_path)`

- [x] Subtract 20 points per hint request; clamp each level score to zero or above. In two-player mode the higher level score receives the separate 20-point bonus; ties receive no bonus.
- [x] Carry total scores across levels and show live level scores and cumulative scores in the HUD and final scoreboard.

## Implemented Rule Decisions

- “Distinct blocks in shortest path” means shortest-route cells, including start and exit.
- Scores carry across levels. One-player mode has no level-winner bonus; in two-player mode, a tie earns no bonus.
- Only a valid move switches turns. Hint requests do not switch turns.
- Both players use the same generated level layout.
- `vcell` now writes directly to `B800h`, matching the existing rendering notes.

## Verification

- JWasm assembly: 0 errors, 0 warnings.
- Independent DFS/BFS model: all walkable cells reachable for 10x10, 20x20, and 30x30 grids.
- Interactive testing in DOSBox is still needed; DOSBox is not installed in the current environment.

## Suggested Implementation Order

1. [x] Settle display and scoring rules.
2. [x] Implement and validate runtime generation and BFS.
3. [x] Add the menu, temporary hints, and scoring.
4. [x] Add two-player turns, names, shared maze views, and level bonuses.
5. [x] Add the final scoreboard.
6. [ ] Run interactive checks for both modes in DOSBox.