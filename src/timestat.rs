use std::time::{Duration, Instant};

use chess::Color;

#[derive(Debug, Clone)]
pub struct TimeStat {
    pub wtime: Option<Duration>,
    pub btime: Option<Duration>,
    pub winc: Option<Duration>,
    pub binc: Option<Duration>,
    pub movestogo: Option<i64>,
    pub movetime: Option<Duration>,
    pub search_depth: Option<i64>,
    total_nodes: i64,
    nodes_since_last: i64,
    nps: i64,
    depth: i32,
    best_score: i32,
    start: Option<Instant>,
    timer: Option<Instant>,
    soft_deadline: Option<Instant>,
    hard_deadline: Option<Instant>,
}

impl TimeStat {
    pub fn new() -> Self {
        Self {
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movestogo: None,
            movetime: None,
            search_depth: None,
            total_nodes: 0,
            nodes_since_last: 0,
            nps: 0,
            depth: 0,
            best_score: 0,
            start: None,
            timer: None,
            soft_deadline: None,
            hard_deadline: None,
        }
    }

    pub fn add_wtime(&mut self, time: &str) -> Result<(), ()> {
        let millis = match u64::from_str_radix(time, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.wtime = Some(Duration::from_millis(millis));
        Ok(())
    }
    pub fn add_btime(&mut self, time: &str) -> Result<(), ()> {
        let millis = match u64::from_str_radix(time, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.btime = Some(Duration::from_millis(millis));
        Ok(())
    }
    pub fn add_winc(&mut self, time: &str) -> Result<(), ()> {
        let millis = match u64::from_str_radix(time, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.winc = Some(Duration::from_millis(millis));
        Ok(())
    }
    pub fn add_binc(&mut self, time: &str) -> Result<(), ()> {
        let millis = match u64::from_str_radix(time, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.binc = Some(Duration::from_millis(millis));
        Ok(())
    }
    pub fn add_movestogo(&mut self, moves: &str) -> Result<(), ()> {
        let moves = match i64::from_str_radix(moves, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.movestogo = Some(moves);
        Ok(())
    }
    pub fn add_depth(&mut self, depth: &str) -> Result<(), ()> {
        let depth = match i64::from_str_radix(depth, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.search_depth = Some(depth);
        Ok(())
    }
    pub fn add_movetime(&mut self, movetime: &str) -> Result<(), ()> {
        let millis = match u64::from_str_radix(movetime, 10) {
            Ok(s) => s,
            Err(_) => return Err(()),
        };
        self.movetime = Some(Duration::from_millis(millis));
        Ok(())
    }

    pub fn calc_deadlines(&mut self, player: Color) {
        self.start = Some(Instant::now());
        self.timer = Some(Instant::now());

        let (stime, sinc);
        match player {
            Color::White => {
                stime = self.wtime;
                sinc = self.winc;
            }
            Color::Black => {
                stime = self.btime;
                sinc = self.binc;
            }
        }
        let mut search_time = Duration::new(0, 0);

        if stime.is_none() && self.movetime.is_none() && self.search_depth.is_none() {
            self.search_depth = Some(6);
        }

        if stime.is_some() && self.movetime.is_none() {
            if self.movestogo.is_some() {
                search_time = stime
                    .unwrap()
                    .div_f64(2.max(self.movestogo.unwrap() + 1) as f64)
                    - Duration::from_millis(50);
                if search_time.as_millis() < 20 {
                    search_time = Duration::from_millis(20);
                }
            } else {
                search_time = stime.unwrap_or(Duration::new(0, 0)).div_f64(20.)
                    + sinc.unwrap_or(Duration::new(0, 0)).div_f64(3.0);
            }
        }

        if search_time.as_millis() != 0 {
            self.soft_deadline = Some(self.start.unwrap() + search_time.div_f64(3.0));
            self.hard_deadline = Some(self.start.unwrap() + search_time);
        }
        if self.movetime.is_some() {
            self.soft_deadline = Some(self.start.unwrap() + self.movetime.unwrap());
            self.hard_deadline = Some(self.start.unwrap() + self.movetime.unwrap());
        }
    }

    pub fn format_score(score: i32) -> String {
        let mut ret = String::new();
        // if score < CHECKMATE + 200 {
        //     ret += "mate -M";
        //     ret += i32::abs(score - CHECKMATE).to_string().as_str();
        // } else if score > -CHECKMATE - 200 {
        //     ret += "mate +M";
        //     ret += i32::abs(score + CHECKMATE).to_string().as_str();
        // } else {

        ret += "cp ";
        ret += score.to_string().as_str();
        // }
        ret
    }

    pub fn update(&mut self) {
        self.timer = Some(Instant::now());

        self.nps = (self.total_nodes as f64
            / (self.timer.unwrap() - self.start.unwrap()).as_secs_f64()) as i64;
    }

    pub fn soft_deadline_passed(&self) -> bool {
        match self.soft_deadline {
            Some(s) => s < self.timer.unwrap(),
            None => false,
        }
    }

    pub fn hard_deadline_passed(&self) -> bool {
        match self.hard_deadline {
            Some(s) => s < self.timer.unwrap(),
            None => false,
        }
    }

    pub fn passed_depth(&mut self, depth: i64) -> bool {
        match self.search_depth {
            Some(s) => s < depth,
            None => false,
        }
    }

    pub fn increment_nodes(&mut self) {
        self.total_nodes += 1;
        self.nodes_since_last += 1;
    }

    pub fn set_depth(&mut self, depth: i32) {
        self.depth = depth;
    }

    pub fn set_best_score(&mut self, score: i32) {
        self.best_score = score;
    }

    pub fn node_probe(&mut self) -> bool {
        // tune these constants based on how fast search is
        const PROBE_INTERVAL: i64 = 1 << 16;
        const PRINT_INTERVAL: i64 = 1 << 20;
        if self.total_nodes % PROBE_INTERVAL == 0 {
            self.update();

            if self.total_nodes % PRINT_INTERVAL == 0 && self.total_nodes > 0 {
                self.print_status();
            }
        }
        self.hard_deadline_passed()
    }

    pub fn print_status(&self) {
        let score_str = Self::format_score(self.best_score);
        let out = format!(
            "info score {} depth {} time {} nodes {} nps {}",
            score_str,
            self.depth,
            (self.timer.unwrap() - self.start.unwrap()).as_millis(),
            self.total_nodes,
            self.nps
        );
        println!("{}", out);
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}
