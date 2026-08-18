use crate::rng::Rng;

pub const POUR_ROW: usize = 1;
const MOUTH_HALF_WIDTH: usize = 2;
const STRIDE_FLOOR: usize = 7;

pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<u8>,
    filled: usize,
    stride: usize,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![0; cols * rows],
            filled: 0,
            stride: sweep_stride(cols),
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn filled(&self) -> usize {
        self.filled
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.cells[y * self.cols + x]
    }

    /// Carries the pile over to a new size, anchored to the floor and the centre.
    pub fn resized(&self, cols: usize, rows: usize) -> Self {
        let mut next = Self::new(cols, rows);
        let dx = cols as isize / 2 - self.cols as isize / 2;
        let dy = rows as isize - self.rows as isize;
        for y in 0..self.rows {
            for x in 0..self.cols {
                let kind = self.at(x, y);
                if kind == 0 {
                    continue;
                }
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < cols && (ny as usize) < rows {
                    next.cells[ny as usize * cols + nx as usize] = kind;
                    next.filled += 1;
                }
            }
        }
        next
    }

    /// Empties the floor so the existing fall rules sink the pile instead of vanishing it.
    pub fn drain_floor(&mut self) {
        if self.rows == 0 {
            return;
        }
        let floor = (self.rows - 1) * self.cols;
        for i in floor..floor + self.cols {
            if self.cells[i] != 0 {
                self.cells[i] = 0;
                self.filled -= 1;
            }
        }
    }

    /// Takes grains from under the pile, so the shape sinks instead of eroding.
    pub fn drain_to(&mut self, grains: usize) {
        for i in (0..self.cells.len()).rev() {
            if self.filled <= grains {
                return;
            }
            if self.cells[i] != 0 {
                self.cells[i] = 0;
                self.filled -= 1;
            }
        }
    }

    pub fn pour(&mut self, rng: &mut Rng, grains: usize) {
        if self.cols == 0 || self.rows <= POUR_ROW {
            return;
        }
        let mouth = self.cols / 2;
        let left = mouth.saturating_sub(MOUTH_HALF_WIDTH);
        let width = (mouth + MOUTH_HALF_WIDTH).min(self.cols - 1) - left + 1;
        for _ in 0..grains {
            let x = left + rng.below(width);
            if self.is_empty(x, POUR_ROW) {
                let kind = grain_kind(rng);
                self.cells[POUR_ROW * self.cols + x] = kind;
                self.filled += 1;
            }
        }
    }

    /// Runs one pass of the fall rules, reporting whether any grain is still moving.
    pub fn settle(&mut self, rng: &mut Rng) -> bool {
        let mut moved = false;
        for y in (0..self.rows.saturating_sub(1)).rev() {
            for k in 0..self.cols {
                let x = (k * self.stride + y) % self.cols;
                if self.is_empty(x, y) {
                    continue;
                }
                if self.is_empty(x, y + 1) {
                    self.slide(x, y, x, y + 1);
                    moved = true;
                    continue;
                }
                if let Some(nx) = self.neighbour(x, rng)
                    && self.is_empty(nx, y + 1)
                    && self.is_empty(nx, y)
                {
                    self.slide(x, y, nx, y + 1);
                    moved = true;
                }
            }
        }
        moved
    }

    fn neighbour(&self, x: usize, rng: &mut Rng) -> Option<usize> {
        if rng.below(2) == 0 {
            x.checked_sub(1)
        } else {
            Some(x + 1).filter(|&nx| nx < self.cols)
        }
    }

    fn is_empty(&self, x: usize, y: usize) -> bool {
        self.at(x, y) == 0
    }

    fn slide(&mut self, x: usize, y: usize, nx: usize, ny: usize) {
        self.cells[ny * self.cols + nx] = self.cells[y * self.cols + x];
        self.cells[y * self.cols + x] = 0;
    }
}

const FULL_PILE_HEIGHT: f64 = 0.7;

/// How much sand a 45° pile this tall covers, once the slopes hit the walls.
fn pile_area(cols: usize, height: usize) -> usize {
    if 2 * height <= cols {
        height * height
    } else {
        height * cols - cols * cols / 4
    }
}

/// As much sand as a grid this shape should ever hold.
pub fn pile_full(cols: usize, rows: usize) -> usize {
    pile_area(cols, (rows as f64 * FULL_PILE_HEIGHT) as usize)
}

fn grain_kind(rng: &mut Rng) -> u8 {
    match rng.below(100) {
        0..4 => 4,
        4..11 => 3,
        11..24 => 2,
        _ => 1,
    }
}

fn sweep_stride(cols: usize) -> usize {
    if cols < 2 {
        return 1;
    }
    (STRIDE_FLOOR..STRIDE_FLOOR + cols)
        .find(|&s| gcd(s, cols) == 1)
        .unwrap_or(1)
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apex(grid: &Grid) -> usize {
        for y in 0..grid.rows() {
            for x in 0..grid.cols() {
                if grid.at(x, y) != 0 {
                    return y;
                }
            }
        }
        grid.rows()
    }

    #[test]
    fn the_sweep_visits_every_column_when_cols_is_a_multiple_of_the_stride_floor() {
        for cols in [70, 84, 98, 102, 140] {
            let stride = sweep_stride(cols);
            let mut seen = vec![false; cols];
            for k in 0..cols {
                seen[(k * stride) % cols] = true;
            }
            assert!(seen.iter().all(|&hit| hit), "cols {cols} stride {stride}");
        }
    }

    #[test]
    fn a_grain_falls_to_the_floor() {
        let mut grid = Grid::new(3, 8);
        let mut rng = Rng::new(1);
        grid.pour(&mut rng, 1);
        for _ in 0..16 {
            grid.settle(&mut rng);
        }
        assert!(grid.at(1, 7) != 0);
        assert_eq!(grid.filled(), 1);
    }

    #[test]
    fn a_grain_slides_off_a_peak() {
        let mut grid = Grid::new(3, 3);
        let mut rng = Rng::new(1);
        grid.cells = vec![0, 0, 0, 0, 1, 0, 0, 1, 0];
        grid.filled = 2;
        grid.settle(&mut rng);
        assert_eq!(grid.at(1, 1), 0);
        assert!(grid.at(0, 2) != 0 || grid.at(2, 2) != 0);
    }

    #[test]
    fn a_blocked_mouth_places_nothing() {
        let mut grid = Grid::new(9, 4);
        let mut rng = Rng::new(1);
        for x in 0..9 {
            grid.cells[POUR_ROW * 9 + x] = 1;
        }
        grid.filled = 9;
        grid.pour(&mut rng, 20);
        assert_eq!(grid.filled(), 9);
    }

    #[test]
    fn settling_never_loses_or_duplicates_a_grain() {
        let mut grid = Grid::new(40, 20);
        let mut rng = Rng::new(9);
        for _ in 0..200 {
            grid.pour(&mut rng, 3);
            grid.settle(&mut rng);
            let counted = (0..grid.rows())
                .flat_map(|y| (0..grid.cols()).map(move |x| (x, y)))
                .filter(|&(x, y)| grid.at(x, y) != 0)
                .count();
            assert_eq!(counted, grid.filled());
        }
    }

    fn full_pile(cols: usize, rows: usize, rng: &mut Rng) -> Grid {
        let mut grid = Grid::new(cols, rows);
        while grid.filled() < pile_full(cols, rows) {
            grid.pour(rng, 40);
            grid.settle(rng);
            grid.settle(rng);
        }
        for _ in 0..2_000 {
            grid.settle(rng);
        }
        grid
    }

    #[test]
    fn draining_the_floor_lets_the_pile_sink_into_it() {
        let mut grid = Grid::new(3, 3);
        grid.cells = vec![0, 1, 0, 1, 1, 1, 1, 1, 1];
        grid.filled = 7;
        grid.drain_floor();
        assert_eq!(grid.filled(), 4);
        assert_eq!(grid.at(1, 0), 1, "the crest should be left alone");
        assert_eq!(grid.at(1, 2), 0, "the floor should have given way");
    }

    #[test]
    fn a_sinking_pile_keeps_its_shape_until_it_is_gone() {
        for (cols, rows) in [(80, 48), (102, 47), (200, 50), (300, 48)] {
            let mut rng = Rng::new(1);
            let mut grid = full_pile(cols, rows, &mut rng);
            let piled = grid.filled();
            let tall = rows - apex(&grid);
            let mut ticks = 0;
            let mut widest = 0;
            while grid.filled() > 0 && ticks < 2_000 {
                grid.drain_floor();
                grid.settle(&mut rng);
                grid.settle(&mut rng);
                ticks += 1;
                let span = (0..cols)
                    .filter(|&x| (0..rows).any(|y| grid.at(x, y) != 0))
                    .count();
                widest = widest.max(span);
            }
            println!(
                "{cols:4}x{rows:<4} {piled:5} grains, {tall:2} rows tall: gone in {ticks} ticks ({}ms), spread over {widest} of {cols} columns",
                ticks * 33
            );
            assert_eq!(grid.filled(), 0, "{cols}x{rows}: never emptied");
        }
    }

    #[test]
    fn a_full_pile_still_fits_under_the_geometric_ceiling() {
        for (cols, rows) in [
            (60, 80),
            (80, 80),
            (80, 48),
            (102, 47),
            (120, 40),
            (160, 44),
            (200, 50),
            (240, 60),
            (300, 48),
        ] {
            let full = pile_full(cols, rows);
            let ceiling = pile_area(cols, rows);
            assert!(
                full < ceiling,
                "{cols}x{rows}: a full pile asks {full} of the {ceiling} the grid can hold"
            );
        }
    }

    #[test]
    fn resizing_wider_keeps_every_grain_and_recentres_it() {
        let mut grid = Grid::new(5, 3);
        grid.cells = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 3, 4, 0];
        grid.filled = 4;
        let wider = grid.resized(11, 3);
        assert_eq!(wider.filled(), 4);
        assert_eq!(wider.at(5, 1), 1);
        assert_eq!(wider.at(4, 2), 2);
        assert_eq!(wider.at(6, 2), 4);
    }

    #[test]
    fn resizing_taller_keeps_the_pile_on_the_new_floor() {
        let mut grid = Grid::new(5, 3);
        grid.cells = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
        grid.filled = 1;
        let taller = grid.resized(5, 7);
        assert_eq!(taller.filled(), 1);
        assert_eq!(taller.at(2, 6), 1);
    }

    #[test]
    fn resizing_narrower_drops_what_no_longer_fits() {
        let mut grid = Grid::new(9, 2);
        for x in 0..9 {
            grid.cells[9 + x] = 1;
        }
        grid.filled = 9;
        assert_eq!(grid.resized(3, 2).filled(), 3);
    }

    #[test]
    fn draining_leaves_the_crest_alone_and_pulls_the_floor_out() {
        let mut grid = Grid::new(3, 3);
        grid.cells = vec![0, 1, 0, 1, 1, 1, 1, 1, 1];
        grid.filled = 7;
        grid.drain_to(5);
        assert_eq!(grid.filled(), 5);
        assert_eq!(grid.at(1, 0), 1);
        assert_eq!(grid.at(2, 2), 0);
    }

    fn pile_until_stalled(cols: usize, rows: usize) -> Grid {
        let mut grid = Grid::new(cols, rows);
        let mut rng = Rng::new(1);
        let mut stalled = 0;
        let mut last = 0;
        for _ in 0..20_000 {
            grid.pour(&mut rng, 40);
            grid.settle(&mut rng);
            grid.settle(&mut rng);
            stalled = if grid.filled() == last {
                stalled + 1
            } else {
                0
            };
            last = grid.filled();
            if stalled > 400 {
                break;
            }
        }
        grid
    }

    #[test]
    fn a_full_cycle_leaves_the_pile_at_the_intended_height() {
        for (cols, rows) in [
            (60, 80),
            (80, 80),
            (80, 48),
            (102, 47),
            (120, 40),
            (160, 44),
            (200, 50),
            (240, 60),
            (300, 48),
        ] {
            let mut grid = Grid::new(cols, rows);
            let mut rng = Rng::new(1);
            let target = pile_full(cols, rows);
            let mut ticks = 0;
            while grid.filled() < target && ticks < 20_000 {
                grid.pour(&mut rng, 40);
                grid.settle(&mut rng);
                grid.settle(&mut rng);
                ticks += 1;
            }
            assert!(
                grid.filled() >= target,
                "{cols}x{rows}: stopped at {} of {target}",
                grid.filled()
            );
            for _ in 0..4_000 {
                grid.settle(&mut rng);
            }
            let height = rows - apex(&grid);
            let share = height as f64 / rows as f64;
            println!(
                "{cols:4}x{rows:<4} target {target:6} height {height:3} of {rows} = {share:.2}"
            );
            assert!(
                (share - FULL_PILE_HEIGHT).abs() < 0.1,
                "{cols}x{rows}: the pile stands {share:.2} of the way up, wanted {FULL_PILE_HEIGHT}"
            );
        }
    }

    #[test]
    fn the_pile_reaches_the_predicted_ceiling_at_every_aspect_ratio() {
        for (cols, rows) in [
            (60, 80),
            (80, 80),
            (80, 48),
            (102, 47),
            (120, 40),
            (160, 44),
            (200, 50),
            (240, 60),
            (300, 48),
        ] {
            let grid = pile_until_stalled(cols, rows);
            let ceiling = pile_area(cols, rows);
            println!(
                "{cols:4}x{rows:<4} piled {:6} ceiling {ceiling:6} apex {:3}",
                grid.filled(),
                apex(&grid)
            );
            assert!(
                grid.filled() >= ceiling,
                "{cols}x{rows}: piled {} but the ceiling promises {ceiling}",
                grid.filled()
            );
        }
    }
}
