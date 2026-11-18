# Cognitive Load vs. A* Bot Comparison Report

- **Source Telemetry:** `COGLOAD.CSV`
- **Mazes Analyzed:** 6 (29 recorded attempts)
- **MAZE.ASM `studycmp` Verification:** Verified (all winners match)
- **Overall Faster-Learner Score:** Player **3** — Computer Bot **3** (Ties: 0)

## Composite Cognitive Load Index (CLI)

$$\text{CLI} = 25(\text{attempts} - 1) + 100 \cdot \frac{\text{total\_extra\_moves}}{\text{optimal\_moves}} + 5(\text{total\_reversals}) + 2(\text{total\_revisited\_cells})$$

Lower CLI reflects faster route convergence with fewer wasted moves, fewer 180-degree hesitations, and less dead-end backtracking.

## 1. Per-Maze Head-to-Head Summary

| Maze ID | Stage (Grid) | Optimal | Player Tries | Bot Tries | Player 1st+ | Bot 1st+ | Player Steps | Bot Steps | Player Rev/Back | Bot Rev/Back | Player CLI | Bot CLI | Faster Learner |
|---:|:---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---|
| 19482 | Stage 1 (15x15) | 24 | 2 | 3 | 6 (25.0%) | 12 (50.0%) | 54 | 88 | 3/4 | 6/12 | 73.00 | 170.67 | **player** |
| 38353 | Stage 1 (15x15) | 25 | 3 | 2 | 12 (48.0%) | 6 (24.0%) | 89 | 56 | 3/7 | 0/0 | 135.00 | 49.00 | **computer** |
| 53949 | Stage 2 (24x24) | 61 | 3 | 1 | 14 (22.95%) | 0 (0.0%) | 199 | 61 | 4/10 | 0/0 | 116.23 | 0.00 | **computer** |
| 41892 | Stage 3 (36x36) | 78 | 3 | 4 | 16 (20.51%) | 16 (20.51%) | 254 | 340 | 5/14 | 6/19 | 128.64 | 178.90 | **player** |
| 5325 | Stage 1 (15x15) | 24 | 1 | 2 | 0 (0.0%) | 4 (16.67%) | 24 | 52 | 0/0 | 0/0 | 0.00 | 41.67 | **player** |
| 52064 | Stage 2 (24x24) | 50 | 3 | 2 | 16 (32.0%) | 34 (68.0%) | 168 | 134 | 2/3 | 3/12 | 102.00 | 132.00 | **computer** |

## 2. Stage-by-Stage Cognitive Load Scaling

| Group | Mazes | Wins (Player / Bot / Tie) | Mean Tries (P vs B) | Mean 1st-Try Extra % (P vs B) | Mean Total Steps (P vs B) | Mean Reversals (P vs B) | Mean Revisited (P vs B) | Mean CLI (P vs B) |
|:---|---:|:---:|---:|---:|---:|---:|---:|---:|
| Stage 1 (15x15) | 3 | 2 / 1 / 0 | 2.00 vs 2.33 | 24.3% vs 30.2% | 55.7 vs 65.3 | 2.00 vs 2.00 | 3.67 vs 4.00 | 69.33 vs 87.11 |
| Stage 2 (24x24) | 2 | 0 / 2 / 0 | 3.00 vs 1.50 | 27.5% vs 34.0% | 183.5 vs 97.5 | 3.00 vs 1.50 | 6.50 vs 6.00 | 109.12 vs 66.00 |
| Stage 3 (36x36) | 1 | 1 / 0 / 0 | 3.00 vs 4.00 | 20.5% vs 20.5% | 254.0 vs 340.0 | 5.00 vs 6.00 | 14.00 vs 19.00 | 128.64 vs 178.90 |
| Overall (All Stages) | 6 | 3 / 3 / 0 | 2.50 vs 2.33 | 24.7% vs 29.9% | 131.3 vs 121.8 | 2.83 vs 2.50 | 6.33 vs 7.17 | 92.48 vs 95.37 |

## 3. Attempt-by-Attempt Learning Curve & Trajectory Metrics

| Maze ID | Runner | Try | Moves | Extra | Efficiency | 180° Reversals | 90° Turns | Revisited Cells | Off-Optimal Steps | Optimal |
|---:|:---|---:|---:|---:|---:|---:|---:|---:|---:|:---:|
| 19482 | player | 1 | 30 | 6 | 80.0% | 3 | 5 | 4 | 3 | No |
| 19482 | player | 2 | 24 | 0 | 100.0% | 0 | 5 | 0 | 0 | Yes |
| 19482 | computer | 1 | 36 | 12 | 66.7% | 4 | 5 | 8 | 19 | No |
| 19482 | computer | 2 | 28 | 4 | 85.7% | 2 | 5 | 4 | 0 | No |
| 19482 | computer | 3 | 24 | 0 | 100.0% | 0 | 5 | 0 | 0 | Yes |
| 38353 | player | 1 | 37 | 12 | 67.6% | 2 | 20 | 6 | 10 | No |
| 38353 | player | 2 | 27 | 2 | 92.6% | 1 | 12 | 1 | 1 | No |
| 38353 | player | 3 | 25 | 0 | 100.0% | 0 | 12 | 0 | 0 | Yes |
| 38353 | computer | 1 | 31 | 6 | 80.7% | 0 | 14 | 0 | 10 | No |
| 38353 | computer | 2 | 25 | 0 | 100.0% | 0 | 10 | 0 | 0 | Yes |
| 53949 | player | 1 | 75 | 14 | 81.3% | 3 | 31 | 9 | 11 | No |
| 53949 | player | 2 | 63 | 2 | 96.8% | 1 | 27 | 1 | 1 | No |
| 53949 | player | 3 | 61 | 0 | 100.0% | 0 | 27 | 0 | 0 | Yes |
| 53949 | computer | 1 | 61 | 0 | 100.0% | 0 | 27 | 0 | 0 | Yes |
| 41892 | player | 1 | 94 | 16 | 83.0% | 3 | 20 | 10 | 12 | No |
| 41892 | player | 2 | 82 | 4 | 95.1% | 2 | 14 | 4 | 0 | No |
| 41892 | player | 3 | 78 | 0 | 100.0% | 0 | 14 | 0 | 0 | Yes |
| 41892 | computer | 1 | 94 | 16 | 83.0% | 2 | 20 | 9 | 8 | No |
| 41892 | computer | 2 | 86 | 8 | 90.7% | 2 | 16 | 6 | 3 | No |
| 41892 | computer | 3 | 82 | 4 | 95.1% | 2 | 14 | 4 | 0 | No |
| 41892 | computer | 4 | 78 | 0 | 100.0% | 0 | 14 | 0 | 0 | Yes |
| 5325 | player | 1 | 24 | 0 | 100.0% | 0 | 7 | 0 | 0 | Yes |
| 5325 | computer | 1 | 28 | 4 | 85.7% | 0 | 13 | 0 | 17 | No |
| 5325 | computer | 2 | 24 | 0 | 100.0% | 0 | 7 | 0 | 0 | Yes |
| 52064 | player | 1 | 66 | 16 | 75.8% | 2 | 26 | 3 | 55 | No |
| 52064 | player | 2 | 52 | 2 | 96.2% | 0 | 21 | 0 | 34 | No |
| 52064 | player | 3 | 50 | 0 | 100.0% | 0 | 26 | 0 | 0 | Yes |
| 52064 | computer | 1 | 84 | 34 | 59.5% | 3 | 38 | 12 | 73 | No |
| 52064 | computer | 2 | 50 | 0 | 100.0% | 0 | 26 | 0 | 0 | Yes |

## 4. Analytical Takeaways

- **Human Global Vision vs. Bot Local Memory:** Overall mean attempts were **2.50** (human) vs. **2.33** (bot), with mean CLI **92.48** vs. **95.37**.
- **Stage 1 (`15x15`) Advantage:** When the entire grid fits comfortably inside human visual working memory, the player solves faster (2–1 tries, winning 2 of 3 Stage 1 mazes).
- **Stage 2 (`24x24`) Visual Clutter Crossover:** Braided loops in `24x24` grids cause human visual lookahead mistakes (`14`–`16` extra moves on try 1), whereas the A* bot commits seen walls to exact memory and learns faster in **both** Stage 2 mazes.
- **Stage 3 (`36x36`) Corridor Commitment:** Despite high visual scale (`16` extra moves on try 1 for both), human global pattern recognition leaps directly across long corridors to beat the bot (`3` tries vs. `4` tries on maze `41892`).
