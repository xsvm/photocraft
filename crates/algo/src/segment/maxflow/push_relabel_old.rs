//! FIFO push/relabel with periodic exact distances to both terminals.
//!
//! Implemented from Goldberg & Tarjan, "A New Approach to the Maximum-Flow
//! Problem", JACM 35(4), 1988, with FIFO scheduling and global relabelling.
//! The existing SoA residual graph is reused. Terminal source arcs are saturated
//! into a preflow; `back` keeps their reverse capacity, `excess` uses f64 so local
//! f32 capacity updates do not accumulate excess-rounding drift.
use super::{CANCEL_EVERY, EPS, Graph, NONE, TERMINAL};
use std::collections::VecDeque;

impl Graph {
    /// Push/relabel; `None` when `cancel` stopped it.
    pub(super) fn push_relabel_old(&mut self, cancel: &dyn Fn() -> bool) -> Option<f64> {
        let n = self.node_count();
        let Some(source_height) = u32::try_from(n).ok().filter(|&n| n < u32::MAX / 2).map(|n| n + 2) else {
            return self.bk(cancel);
        };
        let mut back = vec![0.0f32; n];
        let mut excess = vec![0.0f64; n];
        for i in 0..n {
            self.active[i] = false;
            if self.tr_cap[i] > EPS {
                back[i] = self.tr_cap[i];
                excess[i] = f64::from(self.tr_cap[i]);
                self.tr_cap[i] = 0.0;
            }
        }
        self.global_relabel_old(source_height, &back);
        let mut queue = VecDeque::new();
        for (i, &e) in excess.iter().enumerate() {
            if e > f64::from(EPS) {
                self.active[i] = true;
                queue.push_back(i)
            }
        }
        let mut work = 0usize;
        let frequency = self.r_cap.len().saturating_mul(2).saturating_add(n);
        let mut steps = 0u32;
        while let Some(i) = queue.pop_front() {
            steps = steps.wrapping_add(1);
            if steps.is_multiple_of(CANCEL_EVERY) && cancel() {
                return None;
            }
            self.active[i] = false;
            while excess[i] > f64::from(EPS) {
                if self.dist[i] == 1 && self.tr_cap[i] < -EPS {
                    let delta = (excess[i] as f32).min(-self.tr_cap[i]);
                    self.tr_cap[i] += delta;
                    excess[i] -= f64::from(delta);
                    self.flow += f64::from(delta);
                    if excess[i] <= f64::from(EPS) {
                        break;
                    }
                }
                if self.dist[i] == source_height + 1 && back[i] > EPS {
                    let delta = (excess[i] as f32).min(back[i]);
                    back[i] -= delta;
                    self.tr_cap[i] += delta;
                    excess[i] -= f64::from(delta);
                    if excess[i] <= f64::from(EPS) {
                        break;
                    }
                }
                let mut a = self.parent[i];
                while a != NONE {
                    let ai = a as usize;
                    let j = self.head[ai] as usize;
                    work += 1;
                    // `checked_add`: two nodes a global relabel left unreachable (`u32::MAX`) must
                    // not count as admissible for each other, or float-stranded excess ping-pongs
                    // between them forever.
                    if self.r_cap[ai] > EPS && self.dist[j].checked_add(1) == Some(self.dist[i]) {
                        let delta = (excess[i] as f32).min(self.r_cap[ai]);
                        self.r_cap[ai] -= delta;
                        self.r_cap[ai ^ 1] += delta;
                        excess[i] -= f64::from(delta);
                        excess[j] += f64::from(delta);
                        if !self.active[j] && excess[j] > f64::from(EPS) {
                            self.active[j] = true;
                            queue.push_back(j);
                        }
                        if excess[i] <= f64::from(EPS) {
                            break;
                        }
                    }
                    a = self.next[ai];
                    self.parent[i] = a;
                }
                if excess[i] <= f64::from(EPS) {
                    break;
                }
                let mut height = if self.tr_cap[i] < -EPS { 0 } else { u32::MAX };
                if back[i] > EPS {
                    height = height.min(source_height)
                }
                let mut a = self.first[i];
                while a != NONE {
                    let ai = a as usize;
                    work += 1;
                    if self.r_cap[ai] > EPS {
                        height = height.min(self.dist[self.head[ai] as usize])
                    }
                    a = self.next[ai];
                }
                if height == u32::MAX {
                    break;
                }
                self.dist[i] = height.saturating_add(1);
                self.parent[i] = self.first[i];
            }
            if work >= frequency {
                self.global_relabel_old(source_height, &back);
                work = 0;
            }
        }
        // Report the smallest source-side cut, independently of discharge order.
        self.parent.fill(NONE);
        self.is_sink.fill(true);
        for i in 0..n {
            if self.tr_cap[i] > EPS {
                self.parent[i] = TERMINAL;
                self.is_sink[i] = false;
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            let mut a = self.first[i];
            while a != NONE {
                let ai = a as usize;
                let j = self.head[ai] as usize;
                if self.r_cap[ai] > EPS && self.parent[j] == NONE {
                    self.parent[j] = TERMINAL;
                    self.is_sink[j] = false;
                    queue.push_back(j);
                }
                a = self.next[ai];
            }
        }
        Some(self.flow)
    }

    fn global_relabel_old(&mut self, source_height: u32, back: &[f32]) {
        self.dist.fill(u32::MAX);
        self.parent.clone_from(&self.first);
        let mut queue = VecDeque::new();
        for i in 0..self.node_count() {
            if self.tr_cap[i] < -EPS {
                self.dist[i] = 1;
                queue.push_back(i)
            }
        }
        while let Some(i) = queue.pop_front() {
            let mut a = self.first[i];
            while a != NONE {
                let ai = a as usize;
                let j = self.head[ai] as usize;
                if self.r_cap[ai ^ 1] > EPS && self.dist[j] == u32::MAX {
                    self.dist[j] = self.dist[i].saturating_add(1);
                    queue.push_back(j);
                }
                a = self.next[ai];
            }
        }
        // Give sink-unreachable vertices exact distances back to the source.
        // Resetting all of them to the same height can undo discharge progress.
        for (i, &capacity) in back.iter().enumerate() {
            if capacity > EPS && self.dist[i] == u32::MAX {
                self.dist[i] = source_height + 1;
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            let mut a = self.first[i];
            while a != NONE {
                let ai = a as usize;
                let j = self.head[ai] as usize;
                if self.r_cap[ai ^ 1] > EPS && self.dist[j] == u32::MAX {
                    self.dist[j] = self.dist[i].saturating_add(1);
                    queue.push_back(j);
                }
                a = self.next[ai];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Graph;
    use crate::segment::Rng;

    /// Old (linked-list) and new (per-node blocks) push/relabel on the same graphs: the same
    /// flow, residuals, terminal residuals and cut, bit for bit.
    #[test]
    #[ignore = "analysis: old vs new push/relabel"]
    fn per_node_blocks_match_the_linked_list_solver_bit_for_bit() {
        let mut rng = Rng::new(5);
        let mut checked = 0usize;
        for trial in 0..240usize {
            let (w, h) = (64 + (trial % 7) * 23, 64 + (trial % 5) * 19);
            let n = w * h;
            let mut g = Graph::with_capacity(n, n * 4);
            let kind = trial % 4;
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    // Data terms like GrabCut's: mostly one side, some ties, some large.
                    let (s, t) = match kind {
                        0 => (rng.f32() * 4.0, rng.f32() * 4.0),
                        1 => (if x < w / 3 { 5.0 + rng.f32() } else { 0.0 }, if x > 2 * w / 3 { 5.0 + rng.f32() } else { rng.f32() * 0.01 }),
                        2 => ((rng.f32() * 8.0).round(), (rng.f32() * 8.0).round()),
                        _ => (rng.f32() * 40.0 * (rng.f32() < 0.1) as u8 as f32, rng.f32() * 3.0),
                    };
                    g.add_tweights(i, s, t);
                    // 8-connected forward neighbours, as grid_cut adds them.
                    for (dx, dy, len) in [(1i32, 0i32, 1.0f32), (0, 1, 1.0), (1, 1, std::f32::consts::SQRT_2), (-1, 1, std::f32::consts::SQRT_2)] {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let wt = if kind == 2 { (rng.f32() * 5.0).round() } else { 50.0 * (-rng.f32() * 6.0).exp() / len };
                        g.add_edge(i, ny as usize * w + nx as usize, wt, wt);
                    }
                }
            }
            let (mut old, mut new) = (g.clone(), g);
            let (fo, fnew) = (old.push_relabel_old(&|| false), new.push_relabel(&|| false));
            assert_eq!(fo.map(f64::to_bits), fnew.map(f64::to_bits), "trial {trial}: flow");
            assert!(old.r_cap.iter().zip(&new.r_cap).all(|(a, b)| a.to_bits() == b.to_bits()), "trial {trial}: residuals");
            assert!(old.tr_cap.iter().zip(&new.tr_cap).all(|(a, b)| a.to_bits() == b.to_bits()), "trial {trial}: terminal residuals");
            assert!((0..n).all(|i| old.in_source(i) == new.in_source(i)), "trial {trial}: cut");
            assert_eq!(old.dist, new.dist, "trial {trial}: heights");
            checked += 1;
        }
        eprintln!("old vs new push/relabel: {checked} graphs identical");
    }
}
