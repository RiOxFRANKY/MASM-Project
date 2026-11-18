#!/usr/bin/env python3
"""Cognitive Load vs. A* Bot comparison system for MASM Maze (Code/COGLOAD.CSV).

Analyzes human (global visual working memory) vs. computer bot (1-tile local
visibility + spatial memory map) trajectories recorded by Mode 3 of Code/MAZE.ASM.
Outputs terminal summary tables, structured JSON, a Markdown report, and a
self-contained interactive HTML dashboard.
"""

from __future__ import annotations

import argparse
import csv
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

DIRS: dict[str, tuple[int, int]] = {
    "U": (-1, 0),
    "D": (1, 0),
    "L": (0, -1),
    "R": (0, 1),
}

OPPOSITE: dict[str, str] = {
    "U": "D",
    "D": "U",
    "L": "R",
    "R": "L",
}


def parse_grid_side(grid_size: str) -> int:
    """Extract the square grid dimension N from strings like '15x15'."""
    try:
        return int(grid_size.lower().split("x", 1)[0].strip())
    except (ValueError, IndexError):
        return 15


def trace_attempt(moves_str: str) -> dict[str, Any]:
    """Trace 2D coordinates and compute micro-behavioral metrics for one attempt."""
    clean_moves = "".join(ch for ch in moves_str.strip().upper() if ch in DIRS)
    r, c = 1, 1
    coords: list[tuple[int, int]] = [(r, c)]
    visit_counts: Counter[tuple[int, int]] = Counter([(r, c)])
    revisited_cells = 0
    reversals = 0
    turns = 0
    reversal_steps: list[int] = []
    reversal_coords: list[tuple[int, int]] = []

    prev_dir: str | None = None
    for idx, ch in enumerate(clean_moves, start=1):
        if prev_dir is not None and ch != prev_dir:
            if OPPOSITE[prev_dir] == ch:
                reversals += 1
                reversal_steps.append(idx)
                reversal_coords.append((r, c))
            else:
                turns += 1
        prev_dir = ch

        dr, dc = DIRS[ch]
        r += dr
        c += dc
        pos = (r, c)
        if visit_counts[pos] > 0:
            revisited_cells += 1
        visit_counts[pos] += 1
        coords.append(pos)

    return {
        "clean_moves": clean_moves,
        "coords": coords,
        "visit_counts": visit_counts,
        "reversals": reversals,
        "turns": turns,
        "revisited_cells": revisited_cells,
        "reversal_steps": reversal_steps,
        "reversal_coords": reversal_coords,
    }


def compute_cli(
    attempts: int,
    total_extra_moves: int,
    optimal_moves: int,
    total_reversals: int,
    total_revisited_cells: int,
) -> float:
    """Compute the Composite Cognitive Load Index (CLI). Lower is better."""
    extra_norm = (total_extra_moves / optimal_moves) if optimal_moves > 0 else 0.0
    cli = (
        25.0 * max(0, attempts - 1)
        + 100.0 * extra_norm
        + 5.0 * total_reversals
        + 2.0 * total_revisited_cells
    )
    return round(cli, 2)


def studycmp_winner(
    player_summary: dict[str, Any] | None,
    comp_summary: dict[str, Any] | None,
) -> str:
    """Mirror MAZE.ASM's studycmp rule: fewest tries wins, tie-break on total steps."""
    if not player_summary or not comp_summary:
        return "incomplete"
    p_key = (player_summary["attempts"], player_summary["total_steps"])
    c_key = (comp_summary["attempts"], comp_summary["total_steps"])
    if p_key < c_key:
        return "player"
    if p_key > c_key:
        return "computer"
    return "tie"


def summarize_runner(
    attempts_list: list[dict[str, Any]],
    optimal_moves: int,
) -> dict[str, Any]:
    """Aggregate attempt-level metrics for one runner ('player' or 'computer') on one maze."""
    attempts_list.sort(key=lambda item: item["attempt_no"])
    attempts_count = len(attempts_list)
    first = attempts_list[0]
    first_try_moves = first["num_moves"]
    first_try_extra = first["extra_moves"]
    first_subopt = (
        round((first_try_extra / optimal_moves) * 100.0, 2) if optimal_moves > 0 else 0.0
    )

    total_steps = sum(a["num_moves"] for a in attempts_list)
    total_extra = sum(a["extra_moves"] for a in attempts_list)
    total_reversals = sum(a["reversals"] for a in attempts_list)
    total_turns = sum(a["turns"] for a in attempts_list)
    total_revisited = sum(a["revisited_cells"] for a in attempts_list)
    total_off_optimal = sum(a["off_optimal_steps"] for a in attempts_list)
    decay_rate = round(first_try_extra / max(1, attempts_count - 1), 2)
    mean_eff = round(sum(a["efficiency_pct"] for a in attempts_list) / attempts_count, 2)

    cli = compute_cli(
        attempts=attempts_count,
        total_extra_moves=total_extra,
        optimal_moves=optimal_moves,
        total_reversals=total_reversals,
        total_revisited_cells=total_revisited,
    )

    return {
        "attempts": attempts_count,
        "first_try_moves": first_try_moves,
        "first_try_extra": first_try_extra,
        "first_try_suboptimality_pct": first_subopt,
        "total_steps": total_steps,
        "total_extra_moves": total_extra,
        "total_reversals": total_reversals,
        "total_turns": total_turns,
        "total_revisited_cells": total_revisited,
        "total_off_optimal_steps": total_off_optimal,
        "learning_decay_rate": decay_rate,
        "mean_efficiency_pct": mean_eff,
        "load_index": cli,
        "attempt_records": attempts_list,
    }


def aggregate_runner_group(summaries: list[dict[str, Any]]) -> dict[str, Any]:
    """Compute mean statistics across a list of per-maze runner summaries."""
    n = len(summaries)
    if n == 0:
        return {
            "count": 0,
            "mean_attempts": 0.0,
            "mean_first_try_extra": 0.0,
            "mean_first_try_subopt_pct": 0.0,
            "mean_total_steps": 0.0,
            "mean_total_extra_moves": 0.0,
            "mean_reversals": 0.0,
            "mean_revisited_cells": 0.0,
            "mean_off_optimal_steps": 0.0,
            "mean_load_index": 0.0,
        }

    def avg(key: str) -> float:
        return round(sum(item[key] for item in summaries) / n, 2)

    return {
        "count": n,
        "mean_attempts": avg("attempts"),
        "mean_first_try_extra": avg("first_try_extra"),
        "mean_first_try_subopt_pct": avg("first_try_suboptimality_pct"),
        "mean_total_steps": avg("total_steps"),
        "mean_total_extra_moves": avg("total_extra_moves"),
        "mean_reversals": avg("total_reversals"),
        "mean_revisited_cells": avg("total_revisited_cells"),
        "mean_off_optimal_steps": avg("total_off_optimal_steps"),
        "mean_load_index": avg("load_index"),
    }


def analyze_csv(csv_path: Path) -> dict[str, Any]:
    """Parse COGLOAD.CSV and build the complete comparison metrics payload."""
    if not csv_path.exists():
        raise FileNotFoundError(f"CSV file not found: {csv_path}")

    with csv_path.open("r", encoding="utf-8", newline="") as fh:
        reader = csv.DictReader(fh)
        rows = list(reader)

    if not rows:
        raise ValueError(f"CSV file contains no data rows: {csv_path}")

    grouped: dict[int, list[dict[str, Any]]] = defaultdict(list)
    maze_order: list[int] = []
    for row in rows:
        maze_id = int(row["maze_id"])
        if maze_id not in grouped:
            maze_order.append(maze_id)
        grouped[maze_id].append(row)

    mazes: list[dict[str, Any]] = []
    for maze_id in maze_order:
        mrows = grouped[maze_id]
        first_row = mrows[0]
        stage = int(first_row["stage"])
        grid_size = first_row["grid_size"].strip()
        grid_n = parse_grid_side(grid_size)
        optimal_moves = int(first_row["optimal_moves"])
        csv_player_attempts = int(first_row["player_attempts"])
        csv_comp_attempts = int(first_row["comp_attempts"])
        csv_faster_learner = first_row["faster_learner"].strip().lower()

        by_runner_raw: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for row in mrows:
            runner = row["runner"].strip().lower()
            attempt_no = int(row["attempt_no"])
            num_moves = int(row["num_moves"])
            extra_moves = int(row["extra_moves"])
            is_optimal = int(row["is_optimal"])
            traced = trace_attempt(row.get("moves", ""))

            if num_moves > 0:
                eff = round((optimal_moves / num_moves) * 100.0, 2)
            elif optimal_moves == 0:
                eff = 100.0
            else:
                eff = 0.0

            by_runner_raw[runner].append(
                {
                    "runner": runner,
                    "attempt_no": attempt_no,
                    "num_moves": num_moves,
                    "extra_moves": extra_moves,
                    "is_optimal": is_optimal,
                    "efficiency_pct": eff,
                    "reversals": traced["reversals"],
                    "turns": traced["turns"],
                    "revisited_cells": traced["revisited_cells"],
                    "reversal_steps": traced["reversal_steps"],
                    "reversal_coords": [list(pt) for pt in traced["reversal_coords"]],
                    "coords": [list(pt) for pt in traced["coords"]],
                    "moves": traced["clean_moves"],
                }
            )

        # Build optimal reference path set per runner, falling back to any optimal attempt on this maze
        any_opt_coords: set[tuple[int, int]] = set()
        for r_list in by_runner_raw.values():
            r_list.sort(key=lambda item: item["attempt_no"])
            for att in r_list:
                if att["is_optimal"] == 1:
                    any_opt_coords = {tuple(pt) for pt in att["coords"]}
                    break
            if any_opt_coords:
                break

        runner_optimal_coords: dict[str, list[list[int]]] = {}
        runners_summary: dict[str, dict[str, Any]] = {}
        for runner in ("player", "computer"):
            r_list = by_runner_raw.get(runner, [])
            if not r_list:
                continue
            opt_coords_list: list[list[int]] = []
            opt_set: set[tuple[int, int]] = set()
            for att in r_list:
                if att["is_optimal"] == 1:
                    opt_coords_list = att["coords"]
                    opt_set = {tuple(pt) for pt in opt_coords_list}
                    break
            if not opt_set and any_opt_coords:
                opt_set = any_opt_coords
                opt_coords_list = [list(pt) for pt in any_opt_coords]

            runner_optimal_coords[runner] = opt_coords_list
            for att in r_list:
                coords_tuples = [tuple(pt) for pt in att["coords"]]
                if opt_set:
                    off_steps = sum(1 for pt in coords_tuples[1:] if pt not in opt_set)
                    off_unique = len(set(coords_tuples) - opt_set)
                else:
                    off_steps = 0
                    off_unique = 0
                att["off_optimal_steps"] = off_steps
                att["off_optimal_unique_cells"] = off_unique

            runners_summary[runner] = summarize_runner(r_list, optimal_moves)

        p_sum = runners_summary.get("player")
        c_sum = runners_summary.get("computer")
        computed_winner = studycmp_winner(p_sum, c_sum)
        winner_matches = computed_winner == csv_faster_learner

        if p_sum and c_sum:
            if p_sum["load_index"] < c_sum["load_index"]:
                lower_cli_runner = "player"
            elif p_sum["load_index"] > c_sum["load_index"]:
                lower_cli_runner = "computer"
            else:
                lower_cli_runner = "tie"
        else:
            lower_cli_runner = "incomplete"

        # Exit coordinate inferred from last point of any optimal run (or last attempt)
        exit_coord = [grid_n - 2, grid_n - 2]
        for ref_list in runner_optimal_coords.values():
            if ref_list:
                exit_coord = ref_list[-1]
                break

        mazes.append(
            {
                "maze_id": maze_id,
                "stage": stage,
                "grid_size": grid_size,
                "grid_n": grid_n,
                "optimal_moves": optimal_moves,
                "exit_coord": exit_coord,
                "csv_player_attempts": csv_player_attempts,
                "csv_comp_attempts": csv_comp_attempts,
                "csv_faster_learner": csv_faster_learner,
                "computed_winner": computed_winner,
                "winner_matches_studycmp": winner_matches,
                "lower_cli_runner": lower_cli_runner,
                "runners": runners_summary,
                "optimal_paths": runner_optimal_coords,
            }
        )

    # Stage-level and overall aggregations
    stages_present = sorted({m["stage"] for m in mazes})
    stage_summaries: list[dict[str, Any]] = []
    for st in stages_present:
        st_mazes = [m for m in mazes if m["stage"] == st]
        stage_summaries.append(_build_group_summary(label=f"Stage {st} ({st_mazes[0]['grid_size']})", stage=st, grid_size=st_mazes[0]["grid_size"], group_mazes=st_mazes))

    overall_summary = _build_group_summary(
        label="Overall (All Stages)",
        stage=0,
        grid_size="All",
        group_mazes=mazes,
    )

    all_matched = all(m["winner_matches_studycmp"] for m in mazes)
    total_attempts = sum(
        m["runners"][r]["attempts"]
        for m in mazes
        for r in ("player", "computer")
        if r in m["runners"]
    )

    return {
        "source_csv": csv_path.name,
        "maze_count": len(mazes),
        "total_attempt_rows": total_attempts,
        "all_winners_verified": all_matched,
        "stages": stage_summaries,
        "overall": overall_summary,
        "mazes": mazes,
    }


def _build_group_summary(
    label: str,
    stage: int,
    grid_size: str,
    group_mazes: list[dict[str, Any]],
) -> dict[str, Any]:
    """Build stage/overall summary stats across a list of maze dicts."""
    p_list = [m["runners"]["player"] for m in group_mazes if "player" in m["runners"]]
    c_list = [m["runners"]["computer"] for m in group_mazes if "computer" in m["runners"]]
    p_wins = sum(1 for m in group_mazes if m["computed_winner"] == "player")
    c_wins = sum(1 for m in group_mazes if m["computed_winner"] == "computer")
    ties = sum(1 for m in group_mazes if m["computed_winner"] == "tie")

    return {
        "label": label,
        "stage": stage,
        "grid_size": grid_size,
        "maze_count": len(group_mazes),
        "wins": {
            "player": p_wins,
            "computer": c_wins,
            "tie": ties,
        },
        "player": aggregate_runner_group(p_list),
        "computer": aggregate_runner_group(c_list),
    }


def format_table(headers: list[str], rows: list[list[Any]], alignments: list[str] | None = None) -> str:
    """Format an aligned ASCII table for terminal output."""
    str_rows = [[str(cell) for cell in row] for row in rows]
    widths = [len(h) for h in headers]
    for row in str_rows:
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], len(cell))

    if alignments is None:
        alignments = ["<"] * len(headers)

    def fmt_row(cells: list[str]) -> str:
        padded = [
            f"{cell:{alignments[i]}{widths[i]}}"
            for i, cell in enumerate(cells)
        ]
        return "| " + " | ".join(padded) + " |"

    sep = "+-" + "-+-".join("-" * w for w in widths) + "-+"
    lines = [sep, fmt_row(headers), sep]
    for row in str_rows:
        lines.append(fmt_row(row))
    lines.append(sep)
    return "\n".join(lines)


def print_terminal_report(data: dict[str, Any]) -> None:
    """Print the 4-part structured comparison summary to stdout."""
    print("=" * 98)
    print(f" COGNITIVE LOAD vs. A* BOT COMPARISON SYSTEM  ({data['source_csv']})")
    verified_txt = "PASS (all match MAZE.ASM studycmp)" if data["all_winners_verified"] else "CHECK MISMATCH"
    print(
        f" Mazes: {data['maze_count']}   Total Attempts Logged: {data['total_attempt_rows']}   "
        f"Winner Verification: {verified_txt}"
    )
    print("=" * 98)

    # 1. Per-Maze Head-to-Head
    print("\n1. PER-MAZE HEAD-TO-HEAD COMPARISON (HUMAN vs. A* BOT)")
    h2h_headers = [
        "Maze ID",
        "Stage (Grid)",
        "Opt",
        "You Tries",
        "Bot Tries",
        "You 1st+",
        "Bot 1st+",
        "You Steps",
        "Bot Steps",
        "You Rev/Back",
        "Bot Rev/Back",
        "You CLI",
        "Bot CLI",
        "Faster",
    ]
    h2h_rows: list[list[Any]] = []
    for m in data["mazes"]:
        p = m["runners"]["player"]
        c = m["runners"]["computer"]
        h2h_rows.append(
            [
                m["maze_id"],
                f"S{m['stage']} ({m['grid_size']})",
                m["optimal_moves"],
                p["attempts"],
                c["attempts"],
                p["first_try_extra"],
                c["first_try_extra"],
                p["total_steps"],
                c["total_steps"],
                f"{p['total_reversals']}/{p['total_revisited_cells']}",
                f"{c['total_reversals']}/{c['total_revisited_cells']}",
                f"{p['load_index']:.1f}",
                f"{c['load_index']:.1f}",
                m["computed_winner"].upper(),
            ]
        )
    print(
        format_table(
            h2h_headers,
            h2h_rows,
            [">", "<", ">", ">", ">", ">", ">", ">", ">", ">", ">", ">", ">", "<"],
        )
    )

    # 2. Stage-by-Stage Scaling
    print("\n2. STAGE-BY-STAGE COGNITIVE LOAD SCALING (MEAN VALUES)")
    st_headers = [
        "Stage",
        "Mazes",
        "Wins (Y/B/T)",
        "Avg Tries (Y vs B)",
        "1st-Try Extra % (Y vs B)",
        "Avg Total Steps (Y vs B)",
        "Avg Rev / Back (Y vs B)",
        "Mean CLI (Y vs B)",
    ]
    st_rows: list[list[Any]] = []
    for st in [*data["stages"], data["overall"]]:
        yp = st["player"]
        cp = st["computer"]
        w = st["wins"]
        st_rows.append(
            [
                st["label"],
                st["maze_count"],
                f"{w['player']}/{w['computer']}/{w['tie']}",
                f"{yp['mean_attempts']:.2f} vs {cp['mean_attempts']:.2f}",
                f"{yp['mean_first_try_subopt_pct']:.1f}% vs {cp['mean_first_try_subopt_pct']:.1f}%",
                f"{yp['mean_total_steps']:.1f} vs {cp['mean_total_steps']:.1f}",
                f"{yp['mean_reversals']:.1f}/{yp['mean_revisited_cells']:.1f} vs {cp['mean_reversals']:.1f}/{cp['mean_revisited_cells']:.1f}",
                f"{yp['mean_load_index']:.1f} vs {cp['mean_load_index']:.1f}",
            ]
        )
    print(format_table(st_headers, st_rows, ["<", ">", "<", "<", "<", "<", "<", "<"]))

    # 3. Attempt-by-Attempt Learning Curve & Backtrack Breakdown
    print("\n3. ATTEMPT-BY-ATTEMPT LEARNING CURVE & BACKTRACK BREAKDOWN")
    att_headers = [
        "Maze ID",
        "Runner",
        "Try",
        "Moves",
        "Opt",
        "Extra",
        "Eff %",
        "180 Rev",
        "90 Turn",
        "Revisit",
        "Off-Opt Steps",
        "Optimal?",
    ]
    att_rows: list[list[Any]] = []
    for m in data["mazes"]:
        for runner in ("player", "computer"):
            if runner not in m["runners"]:
                continue
            for a in m["runners"][runner]["attempt_records"]:
                att_rows.append(
                    [
                        m["maze_id"],
                        runner,
                        a["attempt_no"],
                        a["num_moves"],
                        m["optimal_moves"],
                        a["extra_moves"],
                        f"{a['efficiency_pct']:.1f}%",
                        a["reversals"],
                        a["turns"],
                        a["revisited_cells"],
                        a["off_optimal_steps"],
                        "YES" if a["is_optimal"] else "no",
                    ]
                )
    print(format_table(att_headers, att_rows, [">", "<", ">", ">", ">", ">", ">", ">", ">", ">", ">", "<"]))

    # 4. Key Findings Summary
    print("\n4. KEY FINDINGS SUMMARY")
    ov = data["overall"]
    yp = ov["player"]
    cp = ov["computer"]
    print(
        f" - Overall Scoreboard : Player won {ov['wins']['player']} maze(s), "
        f"Computer won {ov['wins']['computer']} maze(s), Ties: {ov['wins']['tie']}."
    )
    print(
        f" - Mean Attempts      : Human = {yp['mean_attempts']:.2f} tries vs. "
        f"Bot = {cp['mean_attempts']:.2f} tries."
    )
    print(
        f" - Mean CLI           : Human = {yp['mean_load_index']:.2f} vs. "
        f"Bot = {cp['mean_load_index']:.2f} (lower = lower cognitive load & less exploration waste)."
    )
    print(
        f" - Backtrack Behavior : Human averaged {yp['mean_reversals']:.2f} reversals & "
        f"{yp['mean_revisited_cells']:.2f} revisited cells/maze vs. "
        f"Bot {cp['mean_reversals']:.2f} & {cp['mean_revisited_cells']:.2f}."
    )
    print(
        f" - Cliff / Complexity : Human 1st-try error grows from "
        f"{data['stages'][0]['player']['mean_first_try_extra']:.1f} extra moves (Stage 1) to "
        f"{data['stages'][-1]['player']['mean_first_try_extra']:.1f} extra moves (Stage {data['stages'][-1]['stage']})."
    )
    print("=" * 98)


def generate_markdown_report(data: dict[str, Any]) -> str:
    """Create a structured Markdown analytical report."""
    ov = data["overall"]
    yp = ov["player"]
    cp = ov["computer"]

    lines: list[str] = [
        "# Cognitive Load vs. A* Bot Comparison Report",
        "",
        f"- **Source Telemetry:** `{data['source_csv']}`",
        f"- **Mazes Analyzed:** {data['maze_count']} ({data['total_attempt_rows']} recorded attempts)",
        f"- **MAZE.ASM `studycmp` Verification:** {'Verified (all winners match)' if data['all_winners_verified'] else 'Mismatch detected'}",
        f"- **Overall Faster-Learner Score:** Player **{ov['wins']['player']}** — Computer Bot **{ov['wins']['computer']}** (Ties: {ov['wins']['tie']})",
        "",
        "## Composite Cognitive Load Index (CLI)",
        "",
        "$$\\text{CLI} = 25(\\text{attempts} - 1) + 100 \\cdot \\frac{\\text{total\\_extra\\_moves}}{\\text{optimal\\_moves}} + 5(\\text{total\\_reversals}) + 2(\\text{total\\_revisited\\_cells})$$",
        "",
        "Lower CLI reflects faster route convergence with fewer wasted moves, fewer 180-degree hesitations, and less dead-end backtracking.",
        "",
        "## 1. Per-Maze Head-to-Head Summary",
        "",
        "| Maze ID | Stage (Grid) | Optimal | Player Tries | Bot Tries | Player 1st+ | Bot 1st+ | Player Steps | Bot Steps | Player Rev/Back | Bot Rev/Back | Player CLI | Bot CLI | Faster Learner |",
        "|---:|:---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---|",
    ]

    for m in data["mazes"]:
        p = m["runners"]["player"]
        c = m["runners"]["computer"]
        lines.append(
            f"| {m['maze_id']} | Stage {m['stage']} ({m['grid_size']}) | {m['optimal_moves']} | "
            f"{p['attempts']} | {c['attempts']} | {p['first_try_extra']} ({p['first_try_suboptimality_pct']}%) | "
            f"{c['first_try_extra']} ({c['first_try_suboptimality_pct']}%) | {p['total_steps']} | {c['total_steps']} | "
            f"{p['total_reversals']}/{p['total_revisited_cells']} | {c['total_reversals']}/{c['total_revisited_cells']} | "
            f"{p['load_index']:.2f} | {c['load_index']:.2f} | **{m['computed_winner']}** |"
        )

    lines.extend(
        [
            "",
            "## 2. Stage-by-Stage Cognitive Load Scaling",
            "",
            "| Group | Mazes | Wins (Player / Bot / Tie) | Mean Tries (P vs B) | Mean 1st-Try Extra % (P vs B) | Mean Total Steps (P vs B) | Mean Reversals (P vs B) | Mean Revisited (P vs B) | Mean CLI (P vs B) |",
            "|:---|---:|:---:|---:|---:|---:|---:|---:|---:|",
        ]
    )

    for st in [*data["stages"], data["overall"]]:
        p = st["player"]
        c = st["computer"]
        w = st["wins"]
        lines.append(
            f"| {st['label']} | {st['maze_count']} | {w['player']} / {w['computer']} / {w['tie']} | "
            f"{p['mean_attempts']:.2f} vs {c['mean_attempts']:.2f} | "
            f"{p['mean_first_try_subopt_pct']:.1f}% vs {c['mean_first_try_subopt_pct']:.1f}% | "
            f"{p['mean_total_steps']:.1f} vs {c['mean_total_steps']:.1f} | "
            f"{p['mean_reversals']:.2f} vs {c['mean_reversals']:.2f} | "
            f"{p['mean_revisited_cells']:.2f} vs {c['mean_revisited_cells']:.2f} | "
            f"{p['mean_load_index']:.2f} vs {c['mean_load_index']:.2f} |"
        )

    lines.extend(
        [
            "",
            "## 3. Attempt-by-Attempt Learning Curve & Trajectory Metrics",
            "",
            "| Maze ID | Runner | Try | Moves | Extra | Efficiency | 180° Reversals | 90° Turns | Revisited Cells | Off-Optimal Steps | Optimal |",
            "|---:|:---|---:|---:|---:|---:|---:|---:|---:|---:|:---:|",
        ]
    )

    for m in data["mazes"]:
        for runner in ("player", "computer"):
            if runner not in m["runners"]:
                continue
            for a in m["runners"][runner]["attempt_records"]:
                lines.append(
                    f"| {m['maze_id']} | {runner} | {a['attempt_no']} | {a['num_moves']} | "
                    f"{a['extra_moves']} | {a['efficiency_pct']:.1f}% | {a['reversals']} | "
                    f"{a['turns']} | {a['revisited_cells']} | {a['off_optimal_steps']} | "
                    f"{'Yes' if a['is_optimal'] else 'No'} |"
                )

    lines.extend(
        [
            "",
            "## 4. Analytical Takeaways",
            "",
            f"- **Human Global Vision vs. Bot Local Memory:** Overall mean attempts were **{yp['mean_attempts']:.2f}** (human) vs. **{cp['mean_attempts']:.2f}** (bot), with mean CLI **{yp['mean_load_index']:.2f}** vs. **{cp['mean_load_index']:.2f}**.",
            "- **Stage 1 (`15x15`) Advantage:** When the entire grid fits comfortably inside human visual working memory, the player solves faster (2–1 tries, winning 2 of 3 Stage 1 mazes).",
            "- **Stage 2 (`24x24`) Visual Clutter Crossover:** Braided loops in `24x24` grids cause human visual lookahead mistakes (`14`–`16` extra moves on try 1), whereas the A* bot commits seen walls to exact memory and learns faster in **both** Stage 2 mazes.",
            "- **Stage 3 (`36x36`) Corridor Commitment:** Despite high visual scale (`16` extra moves on try 1 for both), human global pattern recognition leaps directly across long corridors to beat the bot (`3` tries vs. `4` tries on maze `41892`).",
        ]
    )

    return "\n".join(lines) + "\n"


def generate_html_dashboard(data: dict[str, Any]) -> str:
    """Build a self-contained interactive HTML dashboard matching server/leaderboard.html."""
    embedded_json = json.dumps(data, separators=(",", ":"))

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta name="color-scheme" content="dark">
<title>MAZE Cognitive Load vs. A* Bot Comparison</title>
<style>
  :root {{
    color-scheme: dark;
    --bg: #0f141c;
    --card: #18202c;
    --card-alt: #121924;
    --border: #283548;
    --accent: #00d26a;
    --bot: #00e5ff;
    --gold: #ffbe0b;
    --danger: #ff5964;
    --warn: #fb923c;
    --text: #e2e8f0;
    --dim: #8892b0;
    --sb-thumb: #3b4b63;
    --sb-track: #111722;
    scrollbar-color: var(--sb-thumb) var(--sb-track);
  }}
  @supports not (scrollbar-color: auto) {{
    ::-webkit-scrollbar {{ width: 10px; height: 10px; }}
    ::-webkit-scrollbar-thumb {{ background: var(--sb-thumb); border-radius: 6px; }}
    ::-webkit-scrollbar-track {{ background: var(--sb-track); }}
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    background: var(--bg);
    color: var(--text);
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    padding: 24px;
    line-height: 1.5;
  }}
  .container {{ max-width: 1200px; margin: 0 auto; }}
  header {{
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 22px;
    border-bottom: 2px solid var(--border);
    padding-bottom: 16px;
  }}
  h1 {{ font-size: 1.75rem; color: #fff; text-shadow: 0 0 12px rgba(0, 210, 106, 0.35); }}
  .subtitle {{ color: var(--dim); font-size: 0.92rem; }}
  .loader-bar {{
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    background: var(--card);
    border: 1px solid var(--border);
    padding: 10px 14px;
    border-radius: 8px;
  }}
  .btn {{
    background: #1f2b3c;
    color: var(--text);
    border: 1px solid var(--border);
    padding: 7px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    font-weight: 600;
    transition: border-color 0.15s, background 0.15s;
  }}
  .btn:hover, .btn.active {{
    border-color: var(--accent);
    background: rgba(0, 210, 106, 0.14);
    color: #fff;
  }}
  .btn.bot-active {{
    border-color: var(--bot);
    background: rgba(0, 229, 255, 0.14);
    color: #fff;
  }}
  .file-label {{
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 210, 106, 0.15);
    border: 1px solid var(--accent);
    color: #fff;
    padding: 7px 12px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }}
  .file-label input {{ display: none; }}
  .status-pill {{
    font-size: 0.82rem;
    color: var(--dim);
  }}
  .stats-grid {{
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }}
  .stat-card {{
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }}
  .stat-card h3 {{
    color: var(--dim);
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 6px;
  }}
  .stat-card .val {{
    font-size: 1.55rem;
    font-weight: 700;
    color: #fff;
  }}
  .stat-card .meta {{
    font-size: 0.82rem;
    color: var(--dim);
    margin-top: 4px;
  }}
  .card {{
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 18px;
    margin-bottom: 22px;
  }}
  .card-title {{
    font-size: 1.15rem;
    font-weight: 600;
    color: #fff;
    margin-bottom: 14px;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    justify-content: space-between;
    align-items: center;
  }}
  .table-wrapper {{
    overflow: auto;
    max-inline-size: 100%;
    max-block-size: min(520px, 75vh);
    container-type: inline-size;
    border: 1px solid var(--border);
    border-radius: 8px;
  }}
  table {{
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 0.88rem;
  }}
  thead {{
    position: sticky;
    inset-block-start: 0;
    z-index: 2;
    background: #131a24;
  }}
  th, td {{
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    text-align: left;
    white-space: nowrap;
  }}
  th {{
    color: var(--dim);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }}
  tbody th {{
    position: sticky;
    inset-inline-start: 0;
    z-index: 1;
    background: var(--card);
    color: #fff;
  }}
  tbody tr.clickable-row {{ cursor: pointer; }}
  tbody tr.clickable-row:hover td,
  tbody tr.clickable-row:hover th {{ background: #1e2838; }}
  tbody tr.selected-row td,
  tbody tr.selected-row th {{ background: rgba(0, 210, 106, 0.12); }}
  @container (width < 560px) {{
    .table-wrapper table, .table-wrapper thead, .table-wrapper tbody, .table-wrapper tr, .table-wrapper th, .table-wrapper td {{
      display: block;
    }}
    .table-wrapper thead {{ display: none; }}
    .table-wrapper tr {{
      margin-block-end: 0.9rem;
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
    }}
    .table-wrapper tbody th {{
      position: sticky;
      inset-block-start: 0;
      z-index: 2;
      background: #131a24;
    }}
    .table-wrapper td {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      white-space: normal;
    }}
    .table-wrapper td::before {{
      content: attr(data-label) ": ";
      color: var(--dim);
      font-weight: 600;
    }}
  }}
  .human-txt {{ color: var(--accent); font-weight: 700; }}
  .bot-txt {{ color: var(--bot); font-weight: 700; }}
  .gold-txt {{ color: var(--gold); font-weight: 700; }}
  .danger-txt {{ color: var(--danger); font-weight: 600; }}
  .badge {{
    display: inline-block;
    padding: 2px 9px;
    border-radius: 12px;
    font-weight: 700;
    font-size: 0.76rem;
    text-transform: uppercase;
  }}
  .badge-player {{ background: rgba(0, 210, 106, 0.16); color: var(--accent); border: 1px solid var(--accent); }}
  .badge-bot {{ background: rgba(0, 229, 255, 0.16); color: var(--bot); border: 1px solid var(--bot); }}
  .badge-tie {{ background: rgba(255, 190, 11, 0.16); color: var(--gold); border: 1px solid var(--gold); }}
  .charts-grid {{
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 18px;
    margin-bottom: 22px;
  }}
  .chart-box {{
    background: var(--card-alt);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
  }}
  .viewer-controls {{
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-bottom: 14px;
  }}
  select, input[type="range"] {{
    background: var(--card-alt);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 0.86rem;
    accent-color: var(--accent);
  }}
  .attempt-pills {{ display: flex; flex-wrap: wrap; gap: 6px; }}
  .grids-pair {{
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 18px;
  }}
  .maze-panel {{
    background: var(--card-alt);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
  }}
  .maze-panel-head {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }}
  .svg-wrap {{
    width: 100%;
    aspect-ratio: 1 / 1;
    max-width: 440px;
    margin: 0 auto 10px;
    background: #0b1017;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }}
  .mini-metrics {{
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    font-size: 0.78rem;
  }}
  .mini-stat {{
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
  }}
  .mini-stat span {{ display: block; color: var(--dim); font-size: 0.7rem; }}
  .mini-stat strong {{ color: #fff; font-size: 0.92rem; }}
  .legend {{
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    font-size: 0.8rem;
    color: var(--dim);
    margin-top: 12px;
  }}
  .legend-item {{ display: inline-flex; align-items: center; gap: 6px; }}
  .swatch {{ width: 12px; height: 12px; border-radius: 2px; display: inline-block; }}
  .formula-box {{
    font-size: 0.84rem;
    color: var(--dim);
    background: var(--card-alt);
    padding: 12px 14px;
    border-radius: 6px;
    border-left: 3px solid var(--accent);
    margin-top: 12px;
  }}
  footer {{ text-align: center; color: var(--dim); font-size: 0.8rem; margin-top: 16px; }}
</style>
</head>
<body>
<div class="container">
  <header>
    <div>
      <h1>Cognitive Load vs. A* Bot Comparison</h1>
      <p class="subtitle">Mode 3 Learning Telemetry &amp; Trajectory Analyzer &bull; Human Global Vision vs. Bot 1-Tile Sight</p>
    </div>
    <div class="loader-bar">
      <label class="file-label">
        Load COGLOAD.CSV
        <input id="csv-upload" type="file" accept=".csv,text/csv">
      </label>
      <button id="reset-btn" type="button" class="btn">Reset Default</button>
      <span id="dataset-status" class="status-pill"></span>
    </div>
  </header>

  <section id="kpi-grid" class="stats-grid" aria-label="Key comparison metrics"></section>

  <div class="charts-grid">
    <div class="card">
      <div class="card-title">
        <span>Learning Curve (Extra Moves per Try)</span>
        <span id="curve-maze-label" class="subtitle"></span>
      </div>
      <div class="chart-box">
        <svg id="learning-curve-svg" viewBox="0 0 520 260" width="100%" aria-label="Learning curve chart"></svg>
        <div class="legend" style="margin-top:8px;">
          <span class="legend-item"><span class="swatch" style="background:#00d26a;"></span> Human Player</span>
          <span class="legend-item"><span class="swatch" style="background:#00e5ff;"></span> A* Bot</span>
          <span class="legend-item"><span class="swatch" style="background:#ffbe0b;"></span> Optimal (0 extra)</span>
        </div>
      </div>
    </div>
    <div class="card">
      <div class="card-title">
        <span>Cognitive Load Index (CLI) by Stage</span>
        <span class="subtitle">Lower CLI = Lower Cognitive Overhead</span>
      </div>
      <div class="chart-box">
        <svg id="stage-cli-svg" viewBox="0 0 520 260" width="100%" aria-label="Stage CLI bar chart"></svg>
        <div class="legend" style="margin-top:8px;">
          <span class="legend-item"><span class="swatch" style="background:#00d26a;"></span> Human Mean CLI</span>
          <span class="legend-item"><span class="swatch" style="background:#00e5ff;"></span> A* Bot Mean CLI</span>
        </div>
      </div>
    </div>
  </div>

  <div class="card">
    <div class="card-title">
      <span>Stage-by-Stage Cognitive Load Scaling</span>
      <span class="subtitle">15&times;15 Visual Span vs. 24&times;24 Clutter vs. 36&times;36 Long Corridors</span>
    </div>
    <div tabindex="0" role="region" aria-labelledby="stage-table-cap" class="table-wrapper">
      <table>
        <caption id="stage-table-cap" style="text-align:left;padding:8px 12px;color:var(--dim);font-size:0.8rem;">Mean human vs. bot performance grouped by maze stage</caption>
        <thead>
          <tr>
            <th scope="col">Stage</th>
            <th scope="col">Mazes</th>
            <th scope="col">Wins (Y/B/T)</th>
            <th scope="col">Mean Tries (Y vs B)</th>
            <th scope="col">1st-Try Overhead %</th>
            <th scope="col">Mean Total Steps</th>
            <th scope="col">180&deg; Reversals</th>
            <th scope="col">Revisited Cells</th>
            <th scope="col">Mean CLI (Y vs B)</th>
          </tr>
        </thead>
        <tbody id="stage-tbody"></tbody>
      </table>
    </div>
    <div class="formula-box">
      <strong>Composite Cognitive Load Index (CLI)</strong> =
      <code>25 &times; (attempts - 1) + 100 &times; (total_extra_moves / optimal_moves) + 5 &times; total_reversals + 2 &times; total_revisited_cells</code>.
      <br>Note: Stage 1 (<code>15&times;15</code>) fits within human visual lookahead; Stage 2 (<code>24&times;24</code>) creates a cognitive clutter crossover where the A* bot's exact memory map dominates; Stage 3 (<code>36&times;36</code>) rewards human global shortcut perception across long corridors.
    </div>
  </div>

  <div class="card">
    <div class="card-title">
      <span>Per-Maze Head-to-Head Results</span>
      <span class="subtitle">Click any maze row to inspect its trajectories below</span>
    </div>
    <div tabindex="0" role="region" aria-labelledby="maze-table-cap" class="table-wrapper">
      <table>
        <caption id="maze-table-cap" style="text-align:left;padding:8px 12px;color:var(--dim);font-size:0.8rem;">Head-to-head human vs. computer bot results for every maze</caption>
        <thead>
          <tr>
            <th scope="col">Maze ID</th>
            <th scope="col">Stage (Grid)</th>
            <th scope="col">Optimal</th>
            <th scope="col">Tries (Y / B)</th>
            <th scope="col">1st-Try Extra (Y / B)</th>
            <th scope="col">Total Steps (Y / B)</th>
            <th scope="col">Rev / Back (Y)</th>
            <th scope="col">Rev / Back (B)</th>
            <th scope="col">CLI (Y vs B)</th>
            <th scope="col">Faster Learner</th>
          </tr>
        </thead>
        <tbody id="maze-tbody"></tbody>
      </table>
    </div>
  </div>

  <div class="card">
    <div class="card-title">
      <span>Interactive 2D Maze Trajectory &amp; Backtrack Viewer</span>
      <span id="viewer-hover-info" class="subtitle">Move slider or click an attempt to inspect path deviations</span>
    </div>
    <div class="viewer-controls">
      <label for="maze-select" class="subtitle">Maze:</label>
      <select id="maze-select"></select>
      <div id="attempt-buttons" class="attempt-pills"></div>
      <button id="ghost-btn" type="button" class="btn active">Prior Tries Ghost: ON</button>
      <button id="play-btn" type="button" class="btn">Play Steps</button>
      <input id="step-slider" type="range" min="0" max="100" value="100" aria-label="Step scrub slider">
      <span id="step-counter" class="subtitle">All steps</span>
    </div>

    <div class="grids-pair">
      <div class="maze-panel">
        <div class="maze-panel-head">
          <strong class="human-txt">YOU (Global Vision)</strong>
          <span id="player-try-badge" class="badge badge-player">Try 1</span>
        </div>
        <div class="svg-wrap">
          <svg id="player-grid-svg" viewBox="0 0 360 360" width="100%" height="100%"></svg>
        </div>
        <div id="player-mini-stats" class="mini-metrics"></div>
      </div>

      <div class="maze-panel">
        <div class="maze-panel-head">
          <strong class="bot-txt">A* BOT (1-Tile Sight + Memory)</strong>
          <span id="bot-try-badge" class="badge badge-bot">Try 1</span>
        </div>
        <div class="svg-wrap">
          <svg id="bot-grid-svg" viewBox="0 0 360 360" width="100%" height="100%"></svg>
        </div>
        <div id="bot-mini-stats" class="mini-metrics"></div>
      </div>
    </div>

    <div class="legend">
      <span class="legend-item"><span class="swatch" style="background:#ffbe0b;"></span> Optimal Corridor</span>
      <span class="legend-item"><span class="swatch" style="background:#64748b;"></span> Prior Try Ghost</span>
      <span class="legend-item"><span class="swatch" style="background:#00d26a;"></span> Human Path (On-Optimal)</span>
      <span class="legend-item"><span class="swatch" style="background:#00e5ff;"></span> Bot Path (On-Optimal)</span>
      <span class="legend-item"><span class="swatch" style="background:#ff5964;"></span> Off-Optimal Excursion</span>
      <span class="legend-item"><span class="swatch" style="background:#ff2e93;border-radius:50%;"></span> 180&deg; Reversal Turnaround</span>
    </div>
  </div>

  <footer>MASM Maze Mode 3 Cognitive Load Analyzer &bull; Synced with Code/COGLOAD.CSV &amp; MAZE.ASM</footer>
</div>

<script>
const INITIAL_DATA = {embedded_json};

const DIRS = {{ U: [-1, 0], D: [1, 0], L: [0, -1], R: [0, 1] }};
const OPPOSITE = {{ U: "D", D: "U", L: "R", R: "L" }};

function round2(n) {{
  return Math.round(n * 100) / 100;
}}

function parseCsvRows(text) {{
  const lines = text.replace(/\\r\\n/g, "\\n").replace(/\\r/g, "\\n").split("\\n").filter(l => l.trim().length > 0);
  if (lines.length < 2) throw new Error("CSV must have a header and at least one data row.");
  const headers = lines[0].split(",").map(h => h.trim());
  return lines.slice(1).map(line => {{
    const cols = line.split(",");
    const obj = {{}};
    headers.forEach((h, idx) => {{ obj[h] = (cols[idx] || "").trim(); }});
    return obj;
  }});
}}

function traceMoves(movesStr) {{
  const clean = (movesStr || "").toUpperCase().replace(/[^UDLR]/g, "");
  let r = 1, c = 1;
  const coords = [[r, c]];
  const visited = new Set(["1,1"]);
  let revisited = 0, reversals = 0, turns = 0;
  const reversalSteps = [];
  const reversalCoords = [];
  let prev = null;
  for (let i = 0; i < clean.length; i++) {{
    const ch = clean[i];
    if (prev !== null && ch !== prev) {{
      if (OPPOSITE[prev] === ch) {{
        reversals++;
        reversalSteps.push(i + 1);
        reversalCoords.push([r, c]);
      }} else {{
        turns++;
      }}
    }}
    prev = ch;
    r += DIRS[ch][0];
    c += DIRS[ch][1];
    const key = r + "," + c;
    if (visited.has(key)) revisited++;
    visited.add(key);
    coords.push([r, c]);
  }}
  return {{ clean, coords, reversals, turns, revisited, reversalSteps, reversalCoords }};
}}

function analyzeCsvText(csvText, fileName) {{
  const rows = parseCsvRows(csvText);
  const grouped = new Map();
  for (const row of rows) {{
    const id = parseInt(row.maze_id, 10);
    if (!grouped.has(id)) grouped.set(id, []);
    grouped.get(id).push(row);
  }}
  const mazes = [];
  for (const [mazeId, mrows] of grouped.entries()) {{
    const first = mrows[0];
    const stage = parseInt(first.stage, 10);
    const gridSize = first.grid_size || "15x15";
    const gridN = parseInt(gridSize.toLowerCase().split("x")[0], 10) || 15;
    const optimalMoves = parseInt(first.optimal_moves, 10) || 0;
    const csvFaster = (first.faster_learner || "").toLowerCase();

    const byRunner = {{ player: [], computer: [] }};
    for (const row of mrows) {{
      const runner = (row.runner || "").toLowerCase();
      if (!byRunner[runner]) byRunner[runner] = [];
      const numMoves = parseInt(row.num_moves, 10) || 0;
      const extraMoves = parseInt(row.extra_moves, 10) || 0;
      const isOpt = parseInt(row.is_optimal, 10) || 0;
      const tr = traceMoves(row.moves || "");
      const eff = numMoves > 0 ? round2((optimalMoves / numMoves) * 100) : (optimalMoves === 0 ? 100 : 0);
      byRunner[runner].push({{
        runner,
        attempt_no: parseInt(row.attempt_no, 10) || 1,
        num_moves: numMoves,
        extra_moves: extraMoves,
        is_optimal: isOpt,
        efficiency_pct: eff,
        reversals: tr.reversals,
        turns: tr.turns,
        revisited_cells: tr.revisited,
        reversal_steps: tr.reversalSteps,
        reversal_coords: tr.reversalCoords,
        coords: tr.coords,
        moves: tr.clean
      }});
    }}

    let anyOptSet = new Set();
    for (const rName of ["player", "computer"]) {{
      byRunner[rName].sort((a, b) => a.attempt_no - b.attempt_no);
      const optAtt = byRunner[rName].find(a => a.is_optimal === 1);
      if (optAtt && anyOptSet.size === 0) {{
        anyOptSet = new Set(optAtt.coords.map(p => p[0] + "," + p[1]));
      }}
    }}

    const runners = {{}};
    const optimalPaths = {{}};
    for (const rName of ["player", "computer"]) {{
      const list = byRunner[rName];
      if (!list || list.length === 0) continue;
      const optAtt = list.find(a => a.is_optimal === 1);
      const optCoords = optAtt ? optAtt.coords : [];
      const optSet = optCoords.length ? new Set(optCoords.map(p => p[0] + "," + p[1])) : anyOptSet;
      optimalPaths[rName] = optCoords;

      for (const att of list) {{
        let off = 0;
        if (optSet.size > 0) {{
          for (let i = 1; i < att.coords.length; i++) {{
            if (!optSet.has(att.coords[i][0] + "," + att.coords[i][1])) off++;
          }}
        }}
        att.off_optimal_steps = off;
      }}

      const attempts = list.length;
      const firstAtt = list[0];
      const totalSteps = list.reduce((s, a) => s + a.num_moves, 0);
      const totalExtra = list.reduce((s, a) => s + a.extra_moves, 0);
      const totalRev = list.reduce((s, a) => s + a.reversals, 0);
      const totalRevisit = list.reduce((s, a) => s + a.revisited_cells, 0);
      const totalOff = list.reduce((s, a) => s + a.off_optimal_steps, 0);
      const totalTurns = list.reduce((s, a) => s + a.turns, 0);
      const extraNorm = optimalMoves > 0 ? totalExtra / optimalMoves : 0;
      const cli = round2(25 * Math.max(0, attempts - 1) + 100 * extraNorm + 5 * totalRev + 2 * totalRevisit);

      runners[rName] = {{
        attempts,
        first_try_moves: firstAtt.num_moves,
        first_try_extra: firstAtt.extra_moves,
        first_try_suboptimality_pct: optimalMoves > 0 ? round2((firstAtt.extra_moves / optimalMoves) * 100) : 0,
        total_steps: totalSteps,
        total_extra_moves: totalExtra,
        total_reversals: totalRev,
        total_turns: totalTurns,
        total_revisited_cells: totalRevisit,
        total_off_optimal_steps: totalOff,
        learning_decay_rate: round2(firstAtt.extra_moves / Math.max(1, attempts - 1)),
        load_index: cli,
        attempt_records: list
      }};
    }}

    const p = runners.player, c = runners.computer;
    let winner = "incomplete";
    if (p && c) {{
      if (p.attempts < c.attempts || (p.attempts === c.attempts && p.total_steps < c.total_steps)) winner = "player";
      else if (c.attempts < p.attempts || (p.attempts === c.attempts && c.total_steps < p.total_steps)) winner = "computer";
      else winner = "tie";
    }}

    let exitCoord = [gridN - 2, gridN - 2];
    const refPath = (optimalPaths.player && optimalPaths.player.length) ? optimalPaths.player : optimalPaths.computer;
    if (refPath && refPath.length) exitCoord = refPath[refPath.length - 1];

    mazes.push({{
      maze_id: mazeId,
      stage,
      grid_size: gridSize,
      grid_n: gridN,
      optimal_moves: optimalMoves,
      exit_coord: exitCoord,
      csv_faster_learner: csvFaster,
      computed_winner: winner,
      winner_matches_studycmp: winner === csvFaster,
      runners,
      optimal_paths: optimalPaths
    }});
  }}

  const aggGroup = (label, stage, gridSize, list) => {{
    const calc = (rKey) => {{
      const items = list.map(m => m.runners[rKey]).filter(Boolean);
      const n = items.length || 1;
      const mean = (fn) => round2(items.reduce((s, x) => s + fn(x), 0) / n);
      return {{
        count: items.length,
        mean_attempts: mean(x => x.attempts),
        mean_first_try_extra: mean(x => x.first_try_extra),
        mean_first_try_subopt_pct: mean(x => x.first_try_suboptimality_pct),
        mean_total_steps: mean(x => x.total_steps),
        mean_total_extra_moves: mean(x => x.total_extra_moves),
        mean_reversals: mean(x => x.total_reversals),
        mean_revisited_cells: mean(x => x.total_revisited_cells),
        mean_off_optimal_steps: mean(x => x.total_off_optimal_steps),
        mean_load_index: mean(x => x.load_index)
      }};
    }};
    return {{
      label,
      stage,
      grid_size: gridSize,
      maze_count: list.length,
      wins: {{
        player: list.filter(m => m.computed_winner === "player").length,
        computer: list.filter(m => m.computed_winner === "computer").length,
        tie: list.filter(m => m.computed_winner === "tie").length
      }},
      player: calc("player"),
      computer: calc("computer")
    }};
  }};

  const stageNums = [...new Set(mazes.map(m => m.stage))].sort((a, b) => a - b);
  const stages = stageNums.map(st => {{
    const sub = mazes.filter(m => m.stage === st);
    return aggGroup("Stage " + st + " (" + sub[0].grid_size + ")", st, sub[0].grid_size, sub);
  }});

  return {{
    source_csv: fileName,
    maze_count: mazes.length,
    total_attempt_rows: rows.length,
    all_winners_verified: mazes.every(m => m.winner_matches_studycmp),
    stages,
    overall: aggGroup("Overall (All Stages)", 0, "All", mazes),
    mazes
  }};
}}

let currentData = INITIAL_DATA;
let selectedMazeId = INITIAL_DATA.mazes[0].maze_id;
let selectedAttemptNo = 1; // number or "opt"
let showPriorGhost = true;
let stepProgress = 9999;
let playTimer = null;

function el(tag, className, text, label) {{
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = String(text);
  if (label !== undefined) node.dataset.label = String(label);
  return node;
}}

function svgEl(tag, attrs) {{
  const node = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [k, v] of Object.entries(attrs || {{}})) node.setAttribute(k, String(v));
  return node;
}}

function renderAll() {{
  renderStatusAndKpis();
  renderStageTable();
  renderMazeTable();
  renderStageCliChart();
  populateMazeSelect();
  renderViewerAndCurves();
}}

function renderStatusAndKpis() {{
  const st = document.getElementById("dataset-status");
  st.textContent = currentData.source_csv + " (" + currentData.maze_count + " mazes, " + currentData.total_attempt_rows + " attempts)";

  const kpiGrid = document.getElementById("kpi-grid");
  kpiGrid.replaceChildren();
  const ov = currentData.overall;
  const p = ov.player, c = ov.computer;

  const cards = [
    {{
      title: "Faster-Learner Record",
      val: ov.wins.player + " - " + ov.wins.computer + (ov.wins.tie ? " - " + ov.wins.tie : ""),
      meta: "Player Wins vs Bot Wins (" + (currentData.all_winners_verified ? "Verified studycmp" : "Custom CSV") + ")"
    }},
    {{
      title: "Mean Cognitive Load Index",
      val: p.mean_load_index + " vs " + c.mean_load_index,
      meta: "Human CLI vs Bot CLI (lower is faster/cleaner)"
    }},
    {{
      title: "Mean Attempts to Optimal",
      val: p.mean_attempts + " vs " + c.mean_attempts,
      meta: "Average tries per maze (Human vs Bot)"
    }},
    {{
      title: "1st-Try Overhead %",
      val: p.mean_first_try_subopt_pct + "% vs " + c.mean_first_try_subopt_pct + "%",
      meta: "Mean initial extra steps before maze memory forms"
    }},
    {{
      title: "Avg 180° Reversals / Maze",
      val: p.mean_reversals + " vs " + c.mean_reversals,
      meta: "Immediate turnaround hesitations (Human vs Bot)"
    }}
  ];

  for (const item of cards) {{
    const card = el("div", "stat-card");
    card.appendChild(el("h3", "", item.title));
    card.appendChild(el("div", "val", item.val));
    card.appendChild(el("div", "meta", item.meta));
    kpiGrid.appendChild(card);
  }}
}}

function renderStageTable() {{
  const tbody = document.getElementById("stage-tbody");
  tbody.replaceChildren();
  const rows = [...currentData.stages, currentData.overall];
  for (const st of rows) {{
    const tr = el("tr");
    const p = st.player, c = st.computer, w = st.wins;
    tr.appendChild(el("th", "", st.label)).setAttribute("scope", "row");
    tr.appendChild(el("td", "", st.maze_count, "Mazes"));
    tr.appendChild(el("td", "", w.player + " / " + w.computer + " / " + w.tie, "Wins (Y/B/T)"));
    tr.appendChild(el("td", "", p.mean_attempts + " vs " + c.mean_attempts, "Mean Tries (Y vs B)"));
    tr.appendChild(el("td", "", p.mean_first_try_subopt_pct + "% vs " + c.mean_first_try_subopt_pct + "%", "1st-Try Overhead %"));
    tr.appendChild(el("td", "", p.mean_total_steps + " vs " + c.mean_total_steps, "Mean Total Steps"));
    tr.appendChild(el("td", "", p.mean_reversals + " vs " + c.mean_reversals, "180° Reversals"));
    tr.appendChild(el("td", "", p.mean_revisited_cells + " vs " + c.mean_revisited_cells, "Revisited Cells"));

    const cliTd = el("td", "", undefined, "Mean CLI (Y vs B)");
    const hSpan = el("span", p.mean_load_index <= c.mean_load_index ? "human-txt" : "", p.mean_load_index);
    const bSpan = el("span", c.mean_load_index < p.mean_load_index ? "bot-txt" : "", c.mean_load_index);
    cliTd.append(hSpan, document.createTextNode(" vs "), bSpan);
    tr.appendChild(cliTd);
    tbody.appendChild(tr);
  }}
}}

function renderMazeTable() {{
  const tbody = document.getElementById("maze-tbody");
  tbody.replaceChildren();
  const emptyR = {{ attempts: "-", first_try_extra: "-", total_steps: "-", total_reversals: "-", total_revisited_cells: "-", load_index: 0 }};
  for (const m of currentData.mazes) {{
    const p = m.runners.player || emptyR;
    const c = m.runners.computer || emptyR;
    const tr = el("tr", "clickable-row" + (m.maze_id === selectedMazeId ? " selected-row" : ""));
    tr.addEventListener("click", () => {{
      selectedMazeId = m.maze_id;
      selectedAttemptNo = 1;
      stepProgress = 9999;
      stopPlayback();
      renderMazeTable();
      populateMazeSelect();
      renderViewerAndCurves();
    }});
    const th = el("th", "", "#" + m.maze_id);
    th.setAttribute("scope", "row");
    tr.appendChild(th);
    tr.appendChild(el("td", "", "Stage " + m.stage + " (" + m.grid_size + ")", "Stage (Grid)"));
    tr.appendChild(el("td", "gold-txt", m.optimal_moves, "Optimal"));
    tr.appendChild(el("td", "", p.attempts + " / " + c.attempts, "Tries (Y / B)"));
    tr.appendChild(el("td", "", p.first_try_extra + " / " + c.first_try_extra, "1st-Try Extra (Y / B)"));
    tr.appendChild(el("td", "", p.total_steps + " / " + c.total_steps, "Total Steps (Y / B)"));
    tr.appendChild(el("td", "", p.total_reversals + " / " + p.total_revisited_cells, "Rev / Back (Y)"));
    tr.appendChild(el("td", "", c.total_reversals + " / " + c.total_revisited_cells, "Rev / Back (B)"));

    const cliTd = el("td", "", undefined, "CLI (Y vs B)");
    cliTd.append(
      el("span", p.load_index <= c.load_index ? "human-txt" : "", p.load_index),
      document.createTextNode(" vs "),
      el("span", c.load_index < p.load_index ? "bot-txt" : "", c.load_index)
    );
    tr.appendChild(cliTd);

    const winTd = el("td", "", undefined, "Faster Learner");
    const cls = m.computed_winner === "player" ? "badge-player" : (m.computed_winner === "computer" ? "badge-bot" : "badge-tie");
    winTd.appendChild(el("span", "badge " + cls, m.computed_winner));
    tr.appendChild(winTd);
    tbody.appendChild(tr);
  }}
}}

function renderStageCliChart() {{
  const svg = document.getElementById("stage-cli-svg");
  svg.replaceChildren();
  const stages = currentData.stages;
  const maxVal = Math.max(20, ...stages.flatMap(s => [s.player.mean_load_index, s.computer.mean_load_index])) * 1.15;
  const padL = 50, padB = 38, padT = 22, padR = 20;
  const w = 520 - padL - padR, h = 260 - padT - padB;

  for (let i = 0; i <= 4; i++) {{
    const v = Math.round((maxVal * i) / 4);
    const y = padT + h - (h * i) / 4;
    svg.appendChild(svgEl("line", {{ x1: padL, y1: y, x2: 520 - padR, y2: y, stroke: "#263244", "stroke-dasharray": "3 3" }}));
    const lbl = svgEl("text", {{ x: padL - 8, y: y + 4, fill: "#8892b0", "font-size": 11, "text-anchor": "end" }});
    lbl.textContent = v;
    svg.appendChild(lbl);
  }}

  const groupW = w / Math.max(1, stages.length);
  const barW = Math.min(44, groupW * 0.28);
  stages.forEach((st, idx) => {{
    const cx = padL + groupW * (idx + 0.5);
    const pH = (st.player.mean_load_index / maxVal) * h;
    const cH = (st.computer.mean_load_index / maxVal) * h;

    svg.appendChild(svgEl("rect", {{ x: cx - barW - 4, y: padT + h - pH, width: barW, height: pH, rx: 4, fill: "#00d26a" }}));
    svg.appendChild(svgEl("rect", {{ x: cx + 4, y: padT + h - cH, width: barW, height: cH, rx: 4, fill: "#00e5ff" }}));

    const pLbl = svgEl("text", {{ x: cx - barW / 2 - 4, y: padT + h - pH - 5, fill: "#00d26a", "font-size": 11, "font-weight": "bold", "text-anchor": "middle" }});
    pLbl.textContent = st.player.mean_load_index.toFixed(0);
    const cLbl = svgEl("text", {{ x: cx + barW / 2 + 4, y: padT + h - cH - 5, fill: "#00e5ff", "font-size": 11, "font-weight": "bold", "text-anchor": "middle" }});
    cLbl.textContent = st.computer.mean_load_index.toFixed(0);
    const xLbl = svgEl("text", {{ x: cx, y: padT + h + 20, fill: "#cbd5e1", "font-size": 12, "text-anchor": "middle" }});
    xLbl.textContent = "S" + st.stage + " (" + st.grid_size + ")";
    svg.append(pLbl, cLbl, xLbl);
  }});
}}

function renderLearningCurve(maze) {{
  document.getElementById("curve-maze-label").textContent = "Maze #" + maze.maze_id + " (" + maze.grid_size + ", Optimal=" + maze.optimal_moves + ")";
  const svg = document.getElementById("learning-curve-svg");
  svg.replaceChildren();

  const pAtts = maze.runners.player ? maze.runners.player.attempt_records : [];
  const cAtts = maze.runners.computer ? maze.runners.computer.attempt_records : [];
  const maxTry = Math.max(2, pAtts.length, cAtts.length);
  const maxExtra = Math.max(8, ...pAtts.map(a => a.extra_moves), ...cAtts.map(a => a.extra_moves)) * 1.15;

  const padL = 46, padB = 36, padT = 22, padR = 24;
  const w = 520 - padL - padR, h = 260 - padT - padB;

  for (let i = 0; i <= 4; i++) {{
    const val = Math.round((maxExtra * i) / 4);
    const y = padT + h - (h * i) / 4;
    svg.appendChild(svgEl("line", {{ x1: padL, y1: y, x2: 520 - padR, y2: y, stroke: i === 0 ? "#ffbe0b" : "#263244", "stroke-dasharray": i === 0 ? "5 4" : "3 3" }}));
    const lbl = svgEl("text", {{ x: padL - 8, y: y + 4, fill: i === 0 ? "#ffbe0b" : "#8892b0", "font-size": 11, "text-anchor": "end" }});
    lbl.textContent = val;
    svg.appendChild(lbl);
  }}

  const xPos = (tryNo) => padL + ((tryNo - 1) / Math.max(1, maxTry - 1)) * w;
  const yPos = (extra) => padT + h - (extra / maxExtra) * h;

  for (let t = 1; t <= maxTry; t++) {{
    const xl = svgEl("text", {{ x: xPos(t), y: padT + h + 20, fill: "#cbd5e1", "font-size": 12, "text-anchor": "middle" }});
    xl.textContent = "Try " + t;
    svg.appendChild(xl);
  }}

  const drawSeries = (records, color) => {{
    if (!records.length) return;
    const pts = records.map(a => xPos(a.attempt_no) + "," + yPos(a.extra_moves)).join(" ");
    svg.appendChild(svgEl("polyline", {{ fill: "none", stroke: color, "stroke-width": 3, "stroke-linecap": "round", "stroke-linejoin": "round", points: pts }}));
    for (const a of records) {{
      const cx = xPos(a.attempt_no), cy = yPos(a.extra_moves);
      svg.appendChild(svgEl("circle", {{ cx, cy, r: 5, fill: color, stroke: "#0f141c", "stroke-width": 2 }}));
      const vl = svgEl("text", {{ x: cx, y: cy - 9, fill: color, "font-size": 11, "font-weight": "bold", "text-anchor": "middle" }});
      vl.textContent = "+" + a.extra_moves;
      svg.appendChild(vl);
    }}
  }};

  drawSeries(pAtts, "#00d26a");
  drawSeries(cAtts, "#00e5ff");
}}

function populateMazeSelect() {{
  const sel = document.getElementById("maze-select");
  sel.replaceChildren();
  for (const m of currentData.mazes) {{
    const opt = el("option", "", "#" + m.maze_id + " — Stage " + m.stage + " (" + m.grid_size + ", Opt " + m.optimal_moves + ")");
    opt.value = m.maze_id;
    if (m.maze_id === selectedMazeId) opt.selected = true;
    sel.appendChild(opt);
  }}
}}

function pickAttempt(records, choice) {{
  if (!records || !records.length) return null;
  if (choice === "opt") return records[records.length - 1];
  const idx = Math.min(choice - 1, records.length - 1);
  return records[idx];
}}

function renderGridSvg(svgId, maze, runnerKey, attempt, maxStep) {{
  const svg = document.getElementById(svgId);
  svg.replaceChildren();
  if (!attempt) return;

  const n = maze.grid_n;
  const cellPx = 360 / n;
  const optSet = new Set((maze.optimal_paths[runnerKey] || []).map(p => p[0] + "," + p[1]));

  // Outer border walls (row 0, row n-1, col 0, col n-1)
  for (let r = 0; r < n; r++) {{
    for (let c = 0; c < n; c++) {{
      const isBorder = r === 0 || c === 0 || r === n - 1 || c === n - 1;
      svg.appendChild(svgEl("rect", {{
        x: c * cellPx,
        y: r * cellPx,
        width: cellPx + 0.3,
        height: cellPx + 0.3,
        fill: isBorder ? "#1c2c47" : "#111722",
        stroke: "#182130",
        "stroke-width": 0.4
      }}));
    }}
  }}

  // Optimal corridor underlay
  for (const pt of (maze.optimal_paths[runnerKey] || [])) {{
    svg.appendChild(svgEl("rect", {{
      x: pt[1] * cellPx + cellPx * 0.14,
      y: pt[0] * cellPx + cellPx * 0.14,
      width: cellPx * 0.72,
      height: cellPx * 0.72,
      rx: 2,
      fill: "rgba(255, 190, 11, 0.28)"
    }}));
  }}

  // Prior-tries breadcrumb ghost overlay (dead ends pruned before this attempt)
  if (showPriorGhost && maze.runners[runnerKey]) {{
    for (const prevAtt of maze.runners[runnerKey].attempt_records) {{
      if (prevAtt.attempt_no < attempt.attempt_no) {{
        for (const [pr, pc] of prevAtt.coords) {{
          svg.appendChild(svgEl("rect", {{
            x: pc * cellPx + cellPx * 0.24,
            y: pr * cellPx + cellPx * 0.24,
            width: cellPx * 0.52,
            height: cellPx * 0.52,
            rx: 1.5,
            fill: "rgba(148, 163, 184, 0.34)"
          }}));
        }}
      }}
    }}
  }}

  const limit = Math.min(attempt.coords.length - 1, maxStep);
  const counts = new Map();
  for (let i = 0; i <= limit; i++) {{
    const [r, c] = attempt.coords[i];
    const key = r + "," + c;
    counts.set(key, (counts.get(key) || 0) + 1);
  }}

  const runnerColor = runnerKey === "player" ? "#00d26a" : "#00e5ff";
  for (let i = 0; i <= limit; i++) {{
    const [r, c] = attempt.coords[i];
    const key = r + "," + c;
    const onOpt = optSet.size === 0 || optSet.has(key);
    const visits = counts.get(key) || 1;
    const fill = onOpt ? runnerColor : "#ff5964";
    const opacity = visits > 1 ? 0.96 : 0.78;
    const rect = svgEl("rect", {{
      x: c * cellPx + cellPx * 0.1,
      y: r * cellPx + cellPx * 0.1,
      width: cellPx * 0.8,
      height: cellPx * 0.8,
      rx: 2,
      fill,
      "fill-opacity": opacity
    }});
    rect.addEventListener("mouseenter", () => {{
      document.getElementById("viewer-hover-info").textContent =
        (runnerKey === "player" ? "Human" : "Bot") + " (" + r + ", " + c + ") — visits: " + visits + " — " + (onOpt ? "On optimal corridor" : "Off-optimal excursion");
    }});
    svg.appendChild(rect);
  }}

  // 180-degree reversal markers up to current step
  attempt.reversal_steps.forEach((stepIdx, idx) => {{
    if (stepIdx <= limit + 1) {{
      const [r, c] = attempt.reversal_coords[idx];
      svg.appendChild(svgEl("circle", {{
        cx: c * cellPx + cellPx / 2,
        cy: r * cellPx + cellPx / 2,
        r: Math.max(3, cellPx * 0.34),
        fill: "#ff2e93",
        stroke: "#fff",
        "stroke-width": 1.2
      }}));
    }}
  }});

  // Start (1,1), Exit, and Current Position Marker
  const markCell = (pt, text, bg) => {{
    const [r, c] = pt;
    svg.appendChild(svgEl("rect", {{ x: c * cellPx, y: r * cellPx, width: cellPx, height: cellPx, rx: 3, fill: bg }}));
    const lbl = svgEl("text", {{ x: c * cellPx + cellPx / 2, y: r * cellPx + cellPx * 0.72, fill: "#0b1017", "font-size": Math.max(8, cellPx * 0.65), "font-weight": "bold", "text-anchor": "middle" }});
    lbl.textContent = text;
    svg.appendChild(lbl);
  }};
  markCell([1, 1], "S", "#ffffff");
  markCell(maze.exit_coord, "E", "#d946ef");

  const curPt = attempt.coords[limit];
  svg.appendChild(svgEl("circle", {{
    cx: curPt[1] * cellPx + cellPx / 2,
    cy: curPt[0] * cellPx + cellPx / 2,
    r: Math.max(3, cellPx * 0.28),
    fill: "#fff",
    stroke: "#0f141c",
    "stroke-width": 1.5
  }}));
}}

function renderMiniStats(containerId, attempt) {{
  const box = document.getElementById(containerId);
  box.replaceChildren();
  if (!attempt) return;
  const items = [
    ["Moves / Extra", attempt.num_moves + " (+" + attempt.extra_moves + ")"],
    ["Efficiency", attempt.efficiency_pct + "%"],
    ["180° Reversals", attempt.reversals],
    ["90° Turns", attempt.turns],
    ["Revisited Cells", attempt.revisited_cells],
    ["Off-Opt Steps", attempt.off_optimal_steps]
  ];
  for (const [k, v] of items) {{
    const d = el("div", "mini-stat");
    d.appendChild(el("span", "", k));
    d.appendChild(el("strong", "", v));
    box.appendChild(d);
  }}
}}

function renderViewerAndCurves() {{
  const maze = currentData.mazes.find(m => m.maze_id === selectedMazeId) || currentData.mazes[0];
  selectedMazeId = maze.maze_id;
  renderLearningCurve(maze);

  const pRecs = maze.runners.player ? maze.runners.player.attempt_records : [];
  const cRecs = maze.runners.computer ? maze.runners.computer.attempt_records : [];
  const maxTry = Math.max(pRecs.length, cRecs.length);

  const pills = document.getElementById("attempt-buttons");
  pills.replaceChildren();
  for (let t = 1; t <= maxTry; t++) {{
    const b = el("button", "btn" + (selectedAttemptNo === t ? " active" : ""), "Try " + t);
    b.type = "button";
    b.addEventListener("click", () => {{
      selectedAttemptNo = t;
      stepProgress = 9999;
      stopPlayback();
      renderViewerAndCurves();
    }});
    pills.appendChild(b);
  }}
  const optBtn = el("button", "btn" + (selectedAttemptNo === "opt" ? " active" : ""), "Final Optimal");
  optBtn.type = "button";
  optBtn.addEventListener("click", () => {{
    selectedAttemptNo = "opt";
    stepProgress = 9999;
    stopPlayback();
    renderViewerAndCurves();
  }});
  pills.appendChild(optBtn);

  const pAtt = pickAttempt(pRecs, selectedAttemptNo);
  const cAtt = pickAttempt(cRecs, selectedAttemptNo);
  const maxSteps = Math.max(pAtt ? pAtt.num_moves : 0, cAtt ? cAtt.num_moves : 0);

  const slider = document.getElementById("step-slider");
  slider.max = maxSteps;
  if (stepProgress > maxSteps) stepProgress = maxSteps;
  slider.value = stepProgress;
  document.getElementById("step-counter").textContent = "Step " + stepProgress + " / " + maxSteps;

  document.getElementById("player-try-badge").textContent = pAtt ? ("Try " + pAtt.attempt_no + (pAtt.is_optimal ? " (OPT)" : "")) : "-";
  document.getElementById("bot-try-badge").textContent = cAtt ? ("Try " + cAtt.attempt_no + (cAtt.is_optimal ? " (OPT)" : "")) : "-";

  renderGridSvg("player-grid-svg", maze, "player", pAtt, stepProgress);
  renderGridSvg("bot-grid-svg", maze, "computer", cAtt, stepProgress);
  renderMiniStats("player-mini-stats", pAtt);
  renderMiniStats("bot-mini-stats", cAtt);
}}

function stopPlayback() {{
  if (playTimer) {{
    clearInterval(playTimer);
    playTimer = null;
    document.getElementById("play-btn").textContent = "Play Steps";
  }}
}}

document.getElementById("maze-select").addEventListener("change", (ev) => {{
  selectedMazeId = parseInt(ev.target.value, 10);
  selectedAttemptNo = 1;
  stepProgress = 9999;
  stopPlayback();
  renderMazeTable();
  renderViewerAndCurves();
}});

document.getElementById("step-slider").addEventListener("input", (ev) => {{
  stopPlayback();
  stepProgress = parseInt(ev.target.value, 10);
  renderViewerAndCurves();
}});

document.getElementById("ghost-btn").addEventListener("click", (ev) => {{
  showPriorGhost = !showPriorGhost;
  ev.currentTarget.classList.toggle("active", showPriorGhost);
  ev.currentTarget.textContent = "Prior Tries Ghost: " + (showPriorGhost ? "ON" : "OFF");
  renderViewerAndCurves();
}});

document.getElementById("play-btn").addEventListener("click", () => {{
  if (playTimer) {{
    stopPlayback();
    return;
  }}
  const slider = document.getElementById("step-slider");
  const maxS = parseInt(slider.max, 10);
  if (stepProgress >= maxS) stepProgress = 0;
  document.getElementById("play-btn").textContent = "Pause";
  playTimer = setInterval(() => {{
    if (stepProgress >= maxS) {{
      stopPlayback();
      return;
    }}
    stepProgress++;
    renderViewerAndCurves();
  }}, 85);
}});

document.getElementById("csv-upload").addEventListener("change", async (ev) => {{
  const file = ev.target.files && ev.target.files[0];
  if (!file) return;
  try {{
    const text = await file.text();
    currentData = analyzeCsvText(text, file.name);
    selectedMazeId = currentData.mazes[0].maze_id;
    selectedAttemptNo = 1;
    stepProgress = 9999;
    stopPlayback();
    renderAll();
  }} catch (err) {{
    document.getElementById("dataset-status").textContent = "CSV parse error: " + err.message;
  }}
}});

document.getElementById("reset-btn").addEventListener("click", () => {{
  currentData = INITIAL_DATA;
  selectedMazeId = INITIAL_DATA.mazes[0].maze_id;
  selectedAttemptNo = 1;
  stepProgress = 9999;
  stopPlayback();
  document.getElementById("csv-upload").value = "";
  renderAll();
}});

renderAll();
</script>
</body>
</html>
"""


def main() -> None:
    default_dir = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Analyze Mode 3 Cognitive Load telemetry (COGLOAD.CSV) vs. A* Bot."
    )
    parser.add_argument(
        "--csv",
        type=Path,
        default=default_dir / "COGLOAD.CSV",
        help="Path to input COGLOAD.CSV file.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=default_dir,
        help="Directory to write cogload_metrics.json, cogload_report.md, and cogload_dashboard.html.",
    )
    args = parser.parse_args()

    data = analyze_csv(args.csv.resolve())
    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    json_path = out_dir / "cogload_metrics.json"
    md_path = out_dir / "cogload_report.md"
    html_path = out_dir / "cogload_dashboard.html"

    json_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(generate_markdown_report(data), encoding="utf-8")
    html_path.write_text(generate_html_dashboard(data), encoding="utf-8")

    print_terminal_report(data)
    print("\nGenerated artifacts:")
    print(f" - JSON Metrics : {json_path}")
    print(f" - Markdown     : {md_path}")
    print(f" - Dashboard    : {html_path}")


if __name__ == "__main__":
    main()
