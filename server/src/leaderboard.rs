use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Points awarded for game milestones.
pub const PTS_MATCH_WIN: u32 = 150;
pub const PTS_MATCH_DRAW: u32 = 50;
pub const PTS_LEVEL_EXIT_WIN: u32 = 100;
pub const PTS_LEVEL_TIME_WIN: u32 = 60;
pub const PTS_LEVEL_DRAW: u32 = 30;
pub const PTS_LEVEL_PARTICIPATION: u32 = 10;

#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub ip: IpAddr,
    pub name: String,
    pub score: u32,
    pub matches_played: u32,
    pub matches_won: u32,
    pub matches_lost: u32,
    pub matches_drawn: u32,
    pub levels_played: u32,
    pub levels_won_exit: u32,
    pub levels_won_time: u32,
    pub levels_drawn: u32,
    pub total_time_bonus: u32,
    pub last_active_secs: u64,
}

impl PlayerStats {
    pub fn new(ip: IpAddr) -> Self {
        let name = ip.to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        PlayerStats {
            ip,
            name,
            score: 0,
            matches_played: 0,
            matches_won: 0,
            matches_lost: 0,
            matches_drawn: 0,
            levels_played: 0,
            levels_won_exit: 0,
            levels_won_time: 0,
            levels_drawn: 0,
            total_time_bonus: 0,
            last_active_secs: now,
        }
    }

    pub fn win_rate(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            (self.matches_won as f32 / self.matches_played as f32) * 100.0
        }
    }

    pub fn total_levels_won(&self) -> u32 {
        self.levels_won_exit + self.levels_won_time
    }
}

#[derive(Debug, Clone)]
pub struct MatchHistoryEntry {
    pub p1_name: String,
    pub p2_name: String,
    pub p1_score: u8,
    pub p2_score: u8,
    pub winner: String,
    pub timestamp_secs: u64,
}

#[derive(Default)]
pub struct Leaderboard {
    pub players: HashMap<IpAddr, PlayerStats>,
    pub match_history: Vec<MatchHistoryEntry>,
}

impl Leaderboard {
    pub fn new() -> Self {
        let mut board = Self::default();
        board.load_from_file("leaderboard.json");
        board
    }

    pub fn set_player_name(&mut self, ip: IpAddr, name: String) {
        let entry = self.players.entry(ip).or_insert_with(|| PlayerStats::new(ip));
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            entry.name = trimmed.chars().take(20).collect();
        }
    }

    pub fn record_level_result(
        &mut self,
        winner_ip: Option<IpAddr>,
        loser_ip: Option<IpAddr>,
        is_draw: bool,
        by_exit: bool,
        time_bonus: u32,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if is_draw {
            if let Some(p1) = winner_ip {
                let e1 = self.players.entry(p1).or_insert_with(|| PlayerStats::new(p1));
                e1.levels_played += 1;
                e1.levels_drawn += 1;
                e1.score += PTS_LEVEL_DRAW + PTS_LEVEL_PARTICIPATION;
                e1.last_active_secs = now;
            }
            if let Some(p2) = loser_ip {
                let e2 = self.players.entry(p2).or_insert_with(|| PlayerStats::new(p2));
                e2.levels_played += 1;
                e2.levels_drawn += 1;
                e2.score += PTS_LEVEL_DRAW + PTS_LEVEL_PARTICIPATION;
                e2.last_active_secs = now;
            }
            return;
        }

        if let Some(w) = winner_ip {
            let win_entry = self.players.entry(w).or_insert_with(|| PlayerStats::new(w));
            win_entry.levels_played += 1;
            win_entry.score += PTS_LEVEL_PARTICIPATION;
            if by_exit {
                win_entry.levels_won_exit += 1;
                win_entry.score += PTS_LEVEL_EXIT_WIN + time_bonus;
                win_entry.total_time_bonus += time_bonus;
            } else {
                win_entry.levels_won_time += 1;
                win_entry.score += PTS_LEVEL_TIME_WIN;
            }
            win_entry.last_active_secs = now;
        }

        if let Some(l) = loser_ip {
            let lose_entry = self.players.entry(l).or_insert_with(|| PlayerStats::new(l));
            lose_entry.levels_played += 1;
            lose_entry.score += PTS_LEVEL_PARTICIPATION;
            lose_entry.last_active_secs = now;
        }
    }

    pub fn record_match_result(
        &mut self,
        p1_ip: IpAddr,
        p2_ip: IpAddr,
        p1_level_wins: u8,
        p2_level_wins: u8,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let p1_name = self.players.entry(p1_ip).or_insert_with(|| PlayerStats::new(p1_ip)).name.clone();
        let p2_name = self.players.entry(p2_ip).or_insert_with(|| PlayerStats::new(p2_ip)).name.clone();

        let winner_desc;
        if p1_level_wins > p2_level_wins {
            winner_desc = p1_name.clone();
            if let Some(p1) = self.players.get_mut(&p1_ip) {
                p1.matches_played += 1;
                p1.matches_won += 1;
                p1.score += PTS_MATCH_WIN;
                p1.last_active_secs = now;
            }
            if let Some(p2) = self.players.get_mut(&p2_ip) {
                p2.matches_played += 1;
                p2.matches_lost += 1;
                p2.last_active_secs = now;
            }
        } else if p2_level_wins > p1_level_wins {
            winner_desc = p2_name.clone();
            if let Some(p2) = self.players.get_mut(&p2_ip) {
                p2.matches_played += 1;
                p2.matches_won += 1;
                p2.score += PTS_MATCH_WIN;
                p2.last_active_secs = now;
            }
            if let Some(p1) = self.players.get_mut(&p1_ip) {
                p1.matches_played += 1;
                p1.matches_lost += 1;
                p1.last_active_secs = now;
            }
        } else {
            winner_desc = "Draw".to_string();
            if let Some(p1) = self.players.get_mut(&p1_ip) {
                p1.matches_played += 1;
                p1.matches_drawn += 1;
                p1.score += PTS_MATCH_DRAW;
                p1.last_active_secs = now;
            }
            if let Some(p2) = self.players.get_mut(&p2_ip) {
                p2.matches_played += 1;
                p2.matches_drawn += 1;
                p2.score += PTS_MATCH_DRAW;
                p2.last_active_secs = now;
            }
        }

        self.match_history.push(MatchHistoryEntry {
            p1_name,
            p2_name,
            p1_score: p1_level_wins,
            p2_score: p2_level_wins,
            winner: winner_desc,
            timestamp_secs: now,
        });

        // Keep last 50 matches
        if self.match_history.len() > 50 {
            self.match_history.remove(0);
        }

        self.save_to_files("leaderboard.json", "leaderboard.txt");
    }

    pub fn record_forfeit(&mut self, winner_ip: IpAddr, leaver_ip: IpAddr) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let p1_name = self.players.entry(winner_ip).or_insert_with(|| PlayerStats::new(winner_ip)).name.clone();
        let p2_name = self.players.entry(leaver_ip).or_insert_with(|| PlayerStats::new(leaver_ip)).name.clone();

        if let Some(p1) = self.players.get_mut(&winner_ip) {
            p1.matches_played += 1;
            p1.matches_won += 1;
            p1.score += PTS_MATCH_WIN;
            p1.last_active_secs = now;
        }
        if let Some(p2) = self.players.get_mut(&leaver_ip) {
            p2.matches_played += 1;
            p2.matches_lost += 1;
            p2.last_active_secs = now;
        }

        self.match_history.push(MatchHistoryEntry {
            p1_name: p1_name.clone(),
            p2_name,
            p1_score: 1,
            p2_score: 0,
            winner: format!("{p1_name} (Forfeit)"),
            timestamp_secs: now,
        });

        self.save_to_files("leaderboard.json", "leaderboard.txt");
    }

    pub fn get_sorted(&self) -> Vec<&PlayerStats> {
        let mut list: Vec<&PlayerStats> = self.players.values().collect();
        list.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.matches_won.cmp(&a.matches_won))
                .then_with(|| b.levels_won_exit.cmp(&a.levels_won_exit))
                .then_with(|| b.win_rate().total_cmp(&a.win_rate()))
                .then_with(|| a.matches_lost.cmp(&b.matches_lost))
        });
        list
    }

    pub fn generate_terminal_table(&self) -> String {
        let sorted = self.get_sorted();
        let mut out = String::new();
        out.push_str("\n========================================================================================\n");
        out.push_str("                              MAZE PvP SERVER LEADERBOARD                              \n");
        out.push_str("========================================================================================\n");
        out.push_str(
            "Rank  Player (IP/Name)         Score   Matches (W-L-D)   Win%   Levels (Exit/Time)  Bonus\n",
        );
        out.push_str("----------------------------------------------------------------------------------------\n");

        if sorted.is_empty() {
            out.push_str("  No players have played matches yet. Waiting for games to start...\n");
        } else {
            for (idx, p) in sorted.iter().enumerate() {
                let rank = idx + 1;
                let display_name = if p.name.is_empty() || p.name == p.ip.to_string() {
                    p.ip.to_string()
                } else {
                    format!("{} ({})", p.name, p.ip)
                };
                let truncated: String = display_name.chars().take(22).collect();
                out.push_str(&format!(
                    "{:>4}  {:<22}  {:>6}   {:>3} ({:>2}-{:>2}-{:>2})  {:>4.0}%   {:>3} ({:>2} / {:>2})   {:>5}\n",
                    format!("#{rank}"),
                    truncated,
                    p.score,
                    p.matches_played,
                    p.matches_won,
                    p.matches_lost,
                    p.matches_drawn,
                    p.win_rate(),
                    p.total_levels_won(),
                    p.levels_won_exit,
                    p.levels_won_time,
                    p.total_time_bonus,
                ));
            }
        }

        out.push_str("========================================================================================\n");
        out.push_str("Scoring: Exit Win +100 | Time Win +60 | Match Win +150 | Draw +50/+30 | Speed Bonus +sec\n\n");
        out
    }

    pub fn generate_json(&self) -> String {
        let sorted = self.get_sorted();
        let mut out = String::from("{\n  \"players\": [\n");
        for (idx, p) in sorted.iter().enumerate() {
            let comma = if idx + 1 == sorted.len() { "" } else { "," };
            out.push_str(&format!(
                "    {{\n      \"rank\": {},\n      \"ip\": \"{}\",\n      \"name\": \"{}\",\n      \"score\": {},\n      \"matches_played\": {},\n      \"matches_won\": {},\n      \"matches_lost\": {},\n      \"matches_drawn\": {},\n      \"win_rate\": {:.1},\n      \"levels_played\": {},\n      \"levels_won_exit\": {},\n      \"levels_won_time\": {},\n      \"levels_drawn\": {},\n      \"total_time_bonus\": {},\n      \"last_active_secs\": {}\n    }}{comma}\n",
                idx + 1,
                p.ip,
                p.name.replace('"', "\\\""),
                p.score,
                p.matches_played,
                p.matches_won,
                p.matches_lost,
                p.matches_drawn,
                p.win_rate(),
                p.levels_played,
                p.levels_won_exit,
                p.levels_won_time,
                p.levels_drawn,
                p.total_time_bonus,
                p.last_active_secs
            ));
        }
        out.push_str("  ],\n  \"recent_matches\": [\n");
        for (idx, m) in self.match_history.iter().rev().take(10).enumerate() {
            let comma = if idx + 1 == self.match_history.len().min(10) { "" } else { "," };
            out.push_str(&format!(
                "    {{\n      \"p1\": \"{}\",\n      \"p2\": \"{}\",\n      \"score\": \"{}-{}\",\n      \"winner\": \"{}\",\n      \"timestamp\": {}\n    }}{comma}\n",
                m.p1_name.replace('"', "\\\""),
                m.p2_name.replace('"', "\\\""),
                m.p1_score,
                m.p2_score,
                m.winner.replace('"', "\\\""),
                m.timestamp_secs
            ));
        }
        out.push_str("  ]\n}");
        out
    }

    pub fn generate_html(&self) -> String {
        let sorted = self.get_sorted();
        let mut rows = String::new();

        if sorted.is_empty() {
            rows.push_str("<tr><td colspan='8' style='text-align:center; padding: 25px; color:#888;'>No games played yet. Connect from MAZE.EXE (Option 2) to start!</td></tr>");
        } else {
            for (idx, p) in sorted.iter().enumerate() {
                let rank = idx + 1;
                let badge = match rank {
                    1 => "<span class='badge gold'>🥇 1</span>".to_string(),
                    2 => "<span class='badge silver'>🥈 2</span>".to_string(),
                    3 => "<span class='badge bronze'>🥉 3</span>".to_string(),
                    r => format!("<span class='rank-num'>#{r}</span>"),
                };
                let display_name = if p.name.is_empty() || p.name == p.ip.to_string() {
                    p.ip.to_string()
                } else {
                    format!("{} <small style='color:#777'>({})</small>", p.name, p.ip)
                };

                rows.push_str(&format!(
                    "<tr>\
                        <td>{badge}</td>\
                        <td class='player-name'>{display_name}</td>\
                        <td class='score-cell'>{score}</td>\
                        <td>{mp} <span class='record'>({mw}-{ml}-{md})</span></td>\
                        <td><div class='bar-bg'><div class='bar-fill' style='width: {wr:.0}%'></div></div> {wr:.1}%</td>\
                        <td>{lw} <small style='color:#888'>({l_exit} exit / {l_time} time)</small></td>\
                        <td class='bonus-cell'>+{bonus}</td>\
                    </tr>\n",
                    badge = badge,
                    display_name = display_name,
                    score = p.score,
                    mp = p.matches_played,
                    mw = p.matches_won,
                    ml = p.matches_lost,
                    md = p.matches_drawn,
                    wr = p.win_rate(),
                    lw = p.total_levels_won(),
                    l_exit = p.levels_won_exit,
                    l_time = p.levels_won_time,
                    bonus = p.total_time_bonus
                ));
            }
        }

        let mut match_rows = String::new();
        if self.match_history.is_empty() {
            match_rows.push_str("<tr><td colspan='4' style='text-align:center; padding:15px; color:#666;'>No completed matches yet.</td></tr>");
        } else {
            for m in self.match_history.iter().rev().take(6) {
                match_rows.push_str(&format!(
                    "<tr><td>{} vs {}</td><td style='font-weight:bold;'>{}-{}</td><td style='color:#00ff99;'>{}</td><td>{}</td></tr>\n",
                    m.p1_name, m.p2_name, m.p1_score, m.p2_score, m.winner, "Finished"
                ));
            }
        }

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="refresh" content="5">
<title>MAZE PvP Leaderboard</title>
<style>
  :root {{
    --bg: #0f141c;
    --card: #18202c;
    --border: #283548;
    --accent: #00d26a;
    --accent-glow: rgba(0, 210, 106, 0.25);
    --gold: #ffbe0b;
    --silver: #e0e1dd;
    --bronze: #cd7f32;
    --text: #e2e8f0;
    --dim: #8892b0;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; }}
  body {{ background: var(--bg); color: var(--text); padding: 25px; line-height: 1.5; }}
  .container {{ max-width: 1050px; margin: 0 auto; }}
  header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 25px; border-bottom: 2px solid var(--border); padding-bottom: 15px; }}
  h1 {{ font-size: 2rem; color: #fff; text-shadow: 0 0 10px rgba(0, 210, 106, 0.5); letter-spacing: 1px; }}
  .live-dot {{ display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: #00ff66; box-shadow: 0 0 8px #00ff66; margin-right: 8px; animation: pulse 1.5s infinite; }}
  @keyframes pulse {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0.3; }} 100% {{ opacity: 1; }} }}
  .stats-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin-bottom: 25px; }}
  .stat-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 18px; text-align: center; }}
  .stat-card h3 {{ color: var(--dim); font-size: 0.85rem; text-transform: uppercase; margin-bottom: 6px; letter-spacing: 0.5px; }}
  .stat-card .val {{ font-size: 1.8rem; font-weight: bold; color: var(--accent); }}
  .card {{ background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 20px; margin-bottom: 25px; box-shadow: 0 4px 15px rgba(0,0,0,0.3); }}
  .card-title {{ font-size: 1.25rem; margin-bottom: 15px; color: #fff; display: flex; justify-content: space-between; align-items: center; }}
  table {{ width: 100%; border-collapse: collapse; text-align: left; }}
  th, td {{ padding: 12px 14px; border-bottom: 1px solid var(--border); }}
  th {{ color: var(--dim); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.5px; }}
  tr:hover {{ background: rgba(255,255,255,0.02); }}
  .player-name {{ font-weight: 600; color: #fff; }}
  .score-cell {{ font-weight: 700; color: var(--gold); font-size: 1.15rem; }}
  .bonus-cell {{ color: #00e5ff; font-weight: 500; }}
  .record {{ color: var(--dim); font-size: 0.85rem; }}
  .badge {{ padding: 3px 10px; border-radius: 12px; font-weight: bold; font-size: 0.85rem; display: inline-block; }}
  .badge.gold {{ background: rgba(255, 190, 11, 0.2); color: var(--gold); border: 1px solid var(--gold); }}
  .badge.silver {{ background: rgba(224, 225, 221, 0.2); color: var(--silver); border: 1px solid var(--silver); }}
  .badge.bronze {{ background: rgba(205, 127, 50, 0.2); color: var(--bronze); border: 1px solid var(--bronze); }}
  .rank-num {{ color: var(--dim); font-weight: bold; margin-left: 6px; }}
  .bar-bg {{ display: inline-block; width: 60px; height: 7px; background: #222b38; border-radius: 4px; overflow: hidden; vertical-align: middle; margin-right: 6px; }}
  .bar-fill {{ height: 100%; background: var(--accent); border-radius: 4px; }}
  .formula {{ font-size: 0.85rem; color: var(--dim); line-height: 1.6; background: #111722; padding: 12px; border-radius: 6px; border-left: 3px solid var(--accent); }}
  footer {{ text-align: center; color: var(--dim); font-size: 0.8rem; margin-top: 30px; }}
</style>
</head>
<body>
<div class="container">
  <header>
    <div>
      <h1>🏁 MAZE PvP LEADERBOARD</h1>
      <p style="color: var(--dim); font-size: 0.9rem;">16-bit Retro MS-DOS Maze Racing Competition</p>
    </div>
    <div style="font-size: 0.9rem; color: #aaa;">
      <span class="live-dot"></span>Live Server Active (auto-refreshes 5s)
    </div>
  </header>

  <div class="stats-grid">
    <div class="stat-card">
      <h3>Active Competitors</h3>
      <div class="val">{num_players}</div>
    </div>
    <div class="stat-card">
      <h3>Matches Played</h3>
      <div class="val">{num_matches}</div>
    </div>
    <div class="stat-card">
      <h3>Top Score</h3>
      <div class="val">{top_score}</div>
    </div>
  </div>

  <div class="card">
    <div class="card-title">
      <span>🏆 Player Rankings</span>
      <span style="font-size: 0.8rem; color: var(--dim);">Sorted by Total Points &amp; Wins</span>
    </div>
    <table>
      <thead>
        <tr>
          <th>Rank</th>
          <th>Player</th>
          <th>Score</th>
          <th>Matches (W-L-D)</th>
          <th>Win Rate</th>
          <th>Levels Won</th>
          <th>Speed Bonus</th>
        </tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
  </div>

  <div class="card">
    <div class="card-title">
      <span>⏱ Recent Matches</span>
    </div>
    <table>
      <thead>
        <tr>
          <th>Matchup</th>
          <th>Score</th>
          <th>Winner</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {match_rows}
      </tbody>
    </table>
  </div>

  <div class="card">
    <div class="card-title">
      <span>📐 Scoring Metric Formula</span>
    </div>
    <div class="formula">
      <strong>Total Points = (Exit Wins × 100) + (Distance Wins × 60) + (Match Wins × 150) + (Draws × 50/30) + (Levels Played × 10) + Speed Bonus</strong><br>
      • <strong>Exit Win (+100 pts)</strong>: Reached the exit square before the opponent.<br>
      • <strong>Speed Bonus (+1 pt/sec left)</strong>: Remaining clock seconds awarded when reaching the exit.<br>
      • <strong>Distance Win (+60 pts)</strong>: Closer to exit along true BFS maze distance when timer expires.<br>
      • <strong>Match Win (+150 pts)</strong>: Won best-of-3 levels against the opponent.<br>
      • <strong>Sportsmanship (+10 pts)</strong>: Awarded for every level played to reward participation.
    </div>
  </div>

  <footer>
    MAZE PvP Server &bull; Connect via MAZE.EXE in DOSBox-RS &bull; Designed for Classroom Competition
  </footer>
</div>
</body>
</html>
"#,
            num_players = sorted.len(),
            num_matches = self.match_history.len(),
            top_score = sorted.first().map(|p| p.score).unwrap_or(0),
            rows = rows,
            match_rows = match_rows
        )
    }

    pub fn save_to_files(&self, json_path: &str, txt_path: &str) {
        let json = self.generate_json();
        let _ = fs::write(json_path, json);

        let table = self.generate_terminal_table();
        let _ = fs::write(txt_path, table);
    }

    pub fn load_from_file(&mut self, json_path: &str) {
        let Ok(contents) = fs::read_to_string(json_path) else {
            return;
        };
        // Simple robust JSON parser for standard library without serde
        let Some(players_start) = contents.find("\"players\":") else {
            return;
        };
        let slice = &contents[players_start..];
        let Some(open_bracket) = slice.find('[') else {
            return;
        };
        let Some(close_bracket) = slice.find(']') else {
            return;
        };
        let arr_slice = &slice[open_bracket + 1..close_bracket];

        for block in arr_slice.split('}') {
            let Some(open_brace) = block.find('{') else {
                continue;
            };
            let obj = &block[open_brace + 1..];

            let ip_str = extract_str(obj, "ip");
            let name_str = extract_str(obj, "name");
            let score = extract_u32(obj, "score").unwrap_or(0);
            let matches_played = extract_u32(obj, "matches_played").unwrap_or(0);
            let matches_won = extract_u32(obj, "matches_won").unwrap_or(0);
            let matches_lost = extract_u32(obj, "matches_lost").unwrap_or(0);
            let matches_drawn = extract_u32(obj, "matches_drawn").unwrap_or(0);
            let levels_played = extract_u32(obj, "levels_played").unwrap_or(0);
            let levels_won_exit = extract_u32(obj, "levels_won_exit").unwrap_or(0);
            let levels_won_time = extract_u32(obj, "levels_won_time").unwrap_or(0);
            let levels_drawn = extract_u32(obj, "levels_drawn").unwrap_or(0);
            let total_time_bonus = extract_u32(obj, "total_time_bonus").unwrap_or(0);
            let last_active = extract_u64(obj, "last_active_secs").unwrap_or(0);

            if let Some(ip_text) = ip_str {
                if let Ok(ip) = ip_text.parse::<IpAddr>() {
                    let mut stats = PlayerStats::new(ip);
                    if let Some(n) = name_str {
                        if !n.is_empty() {
                            stats.name = n;
                        }
                    }
                    stats.score = score;
                    stats.matches_played = matches_played;
                    stats.matches_won = matches_won;
                    stats.matches_lost = matches_lost;
                    stats.matches_drawn = matches_drawn;
                    stats.levels_played = levels_played;
                    stats.levels_won_exit = levels_won_exit;
                    stats.levels_won_time = levels_won_time;
                    stats.levels_drawn = levels_drawn;
                    stats.total_time_bonus = total_time_bonus;
                    stats.last_active_secs = last_active;
                    self.players.insert(ip, stats);
                }
            }
        }
    }
}

fn extract_str(obj: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":");
    let pos = obj.find(&pattern)?;
    let rest = &obj[pos + pattern.len()..];
    let first_quote = rest.find('"')?;
    let after_first = &rest[first_quote + 1..];
    let second_quote = after_first.find('"')?;
    Some(after_first[..second_quote].to_string())
}

fn extract_u32(obj: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{key}\":");
    let pos = obj.find(&pattern)?;
    let rest = &obj[pos + pattern.len()..];
    let trimmed = rest.trim_start();
    let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

fn extract_u64(obj: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{key}\":");
    let pos = obj.find(&pattern)?;
    let rest = &obj[pos + pattern.len()..];
    let trimmed = rest.trim_start();
    let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}
