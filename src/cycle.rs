use std::time::{Duration, Instant, SystemTime};

use crate::rng::Rng;

const OPENING_SHARE: (f64, f64) = (0.1, 0.9);
const DRIBBLE_GAP_MS: (u64, u64) = (1_000, 5_000);
const DRIBBLE_GRAINS: usize = 3;
const NUDGE_GRAINS: usize = 5;
const SWAY_STEP: Duration = Duration::from_millis(50);
const SWAY_PATH: [isize; 4] = [0, 1, 0, -1];
const EMPTY_PAUSE: Duration = Duration::from_millis(600);

pub enum Want {
    Sand(usize),
    /// Pull the floor out, and draw the pile this many columns off centre.
    Drain(isize),
    Nothing,
}

#[derive(Clone, Copy)]
pub struct Pile {
    pub filled: usize,
    pub resting: bool,
}

pub enum Cycle {
    Endless(Endless),
    Timer { period: Duration, start: SystemTime },
}

impl Cycle {
    pub fn takes_input(&self) -> bool {
        matches!(self, Self::Endless(_))
    }

    pub fn want(
        &mut self,
        now: Instant,
        wall: SystemTime,
        full: usize,
        pile: Pile,
        rng: &mut Rng,
    ) -> Want {
        match self {
            Self::Endless(endless) => endless.want(now, full, pile, rng),
            Self::Timer { .. } => Want::Sand(self.asked(wall, full)),
        }
    }

    /// The most sand this cycle wants right now, for capping a resized pile.
    pub fn asked(&self, wall: SystemTime, full: usize) -> usize {
        match self {
            Self::Endless(endless) => endless.goal.min(full),
            Self::Timer { period, start } => {
                let progress = wall
                    .duration_since(*start)
                    .map(|spent| spent.as_secs_f64() / period.as_secs_f64())
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                (progress * full as f64) as usize
            }
        }
    }

    pub fn nudge(&mut self, full: usize) {
        if let Self::Endless(endless) = self {
            endless.nudge(full);
        }
    }
}

enum Phase {
    Filling,
    Draining(Instant),
    Waiting(Instant),
}

/// The default cycle: fill to full, sink the pile, open again with a fresh handful.
pub struct Endless {
    goal: usize,
    grew: Instant,
    gap: Duration,
    phase: Phase,
}

impl Endless {
    pub fn opening(now: Instant, full: usize, rng: &mut Rng) -> Self {
        let (low, high) = OPENING_SHARE;
        let span = ((high - low) * full as f64) as usize;
        Self {
            goal: (low * full as f64) as usize + rng.below(span.max(1)),
            grew: now,
            gap: next_gap(rng),
            phase: Phase::Filling,
        }
    }

    fn want(&mut self, now: Instant, full: usize, pile: Pile, rng: &mut Rng) -> Want {
        match self.phase {
            Phase::Filling => {
                if now.saturating_duration_since(self.grew) >= self.gap {
                    self.goal += DRIBBLE_GRAINS;
                    self.grew = now;
                    self.gap = next_gap(rng);
                }
                // Sand still in the air would look like the floor gave way early.
                if pile.filled >= full && pile.resting {
                    self.phase = Phase::Draining(now);
                    return Want::Drain(0);
                }
                Want::Sand(self.goal)
            }
            Phase::Draining(since) => {
                if pile.filled > 0 {
                    return Want::Drain(sway(now.saturating_duration_since(since)));
                }
                self.phase = Phase::Waiting(now);
                Want::Nothing
            }
            Phase::Waiting(since) => {
                if now.saturating_duration_since(since) < EMPTY_PAUSE {
                    return Want::Nothing;
                }
                *self = Self::opening(now, full, rng);
                Want::Sand(self.goal)
            }
        }
    }

    fn nudge(&mut self, full: usize) {
        if matches!(self.phase, Phase::Filling) {
            self.goal = (self.goal + NUDGE_GRAINS).min(full);
        }
    }
}

/// Nothing moves within the sand, so the pile shakes as one body.
fn sway(spent: Duration) -> isize {
    let step = (spent.as_millis() / SWAY_STEP.as_millis()) as usize;
    SWAY_PATH[step % SWAY_PATH.len()]
}

fn next_gap(rng: &mut Rng) -> Duration {
    let (low, high) = DRIBBLE_GAP_MS;
    Duration::from_millis(low + rng.below((high - low + 1) as usize) as u64)
}

/// Reads `--timer <dur>`; without it the cycle is the endless pile.
pub fn timer_from_args(mut args: impl Iterator<Item = String>) -> Result<Option<Duration>, String> {
    let mut period = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--timer" => {
                let text = args.next().ok_or("--timer wants a duration, e.g. 25m")?;
                period = Some(parse_duration(&text)?);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(period)
}

fn parse_duration(text: &str) -> Result<Duration, String> {
    let (digits, unit) = text.split_at(text.len().saturating_sub(1));
    let scale = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        _ => return Err(format!("{text} needs a unit: 90s, 25m or 1h")),
    };
    let count: u64 = digits
        .parse()
        .map_err(|_| format!("{text} needs a number before the unit"))?;
    if count == 0 {
        return Err(format!("{text} is not long enough to watch"));
    }
    Ok(Duration::from_secs(count * scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: usize = 800;

    fn endless(now: Instant) -> Cycle {
        Cycle::Endless(Endless::opening(now, FULL, &mut Rng::new(3)))
    }

    fn grains(want: Want) -> Option<usize> {
        match want {
            Want::Sand(grains) => Some(grains),
            _ => None,
        }
    }

    fn rested(filled: usize) -> Pile {
        Pile {
            filled,
            resting: true,
        }
    }

    #[test]
    fn a_full_pile_waits_for_the_last_grains_to_land_before_it_collapses() {
        let now = Instant::now();
        let mut cycle = endless(now);
        let mut rng = Rng::new(1);
        let wall = SystemTime::now();
        for _ in 0..1_000 {
            cycle.nudge(FULL);
        }

        let falling = Pile {
            filled: FULL,
            resting: false,
        };
        assert!(
            grains(cycle.want(now, wall, FULL, falling, &mut rng)).is_some(),
            "a full pile collapsed with sand still in the air"
        );
        assert!(matches!(
            cycle.want(now, wall, FULL, rested(FULL), &mut rng),
            Want::Drain(_)
        ));
    }

    #[test]
    fn a_round_opens_anywhere_from_a_dusting_to_nearly_full() {
        let mut seen = (FULL, 0);
        for seed in 1..200 {
            let opening = Endless::opening(Instant::now(), FULL, &mut Rng::new(seed)).goal;
            assert!(
                (80..720).contains(&opening),
                "opened at {opening} of {FULL}"
            );
            seen = (seen.0.min(opening), seen.1.max(opening));
        }
        assert!(
            seen.1 - seen.0 > FULL / 2,
            "openings only spanned {seen:?} of {FULL}"
        );
    }

    #[test]
    fn the_dribble_waits_between_one_and_five_seconds() {
        let now = Instant::now();
        let mut rng = Rng::new(1);
        let mut seen = (u64::MAX, 0);
        for _ in 0..200 {
            let gap = next_gap(&mut rng).as_millis() as u64;
            assert!((1_000..=5_000).contains(&gap), "waited {gap}ms");
            seen = (seen.0.min(gap), seen.1.max(gap));
        }
        assert!(
            seen.0 < 1_500 && seen.1 > 4_500,
            "gaps only spanned {seen:?}"
        );

        let mut cycle = endless(now);
        let wall = SystemTime::now();
        let opening = cycle.asked(wall, FULL);
        assert_eq!(
            grains(cycle.want(now, wall, FULL, rested(0), &mut rng)),
            Some(opening)
        );
        let soon = now + Duration::from_millis(900);
        assert_eq!(
            grains(cycle.want(soon, wall, FULL, rested(0), &mut rng)),
            Some(opening),
            "sand arrived before the shortest gap"
        );
        let later = now + Duration::from_millis(5_100);
        assert_eq!(
            grains(cycle.want(later, wall, FULL, rested(0), &mut rng)),
            Some(opening + DRIBBLE_GRAINS)
        );
    }

    #[test]
    fn a_nudge_asks_for_more_but_never_past_full() {
        let now = Instant::now();
        let mut cycle = endless(now);
        let opening = cycle.asked(SystemTime::now(), FULL);
        cycle.nudge(FULL);
        assert_eq!(cycle.asked(SystemTime::now(), FULL), opening + NUDGE_GRAINS);
        for _ in 0..1_000 {
            cycle.nudge(FULL);
        }
        assert_eq!(cycle.asked(SystemTime::now(), FULL), FULL);
    }

    #[test]
    fn a_full_pile_sinks_pauses_and_opens_again() {
        let now = Instant::now();
        let mut cycle = endless(now);
        let mut rng = Rng::new(1);
        let wall = SystemTime::now();
        for _ in 0..1_000 {
            cycle.nudge(FULL);
        }

        assert!(matches!(
            cycle.want(now, wall, FULL, rested(FULL), &mut rng),
            Want::Drain(_)
        ));
        assert!(matches!(
            cycle.want(now, wall, FULL, rested(12), &mut rng),
            Want::Drain(_)
        ));
        assert!(matches!(
            cycle.want(now, wall, FULL, rested(0), &mut rng),
            Want::Nothing
        ));

        let after = now + EMPTY_PAUSE;
        let reopened = grains(cycle.want(after, wall, FULL, rested(0), &mut rng));
        assert!(reopened.is_some_and(|goal| goal < FULL));
    }

    #[test]
    fn a_nudge_does_nothing_once_the_pile_is_sinking() {
        let now = Instant::now();
        let mut cycle = endless(now);
        let mut rng = Rng::new(1);
        for _ in 0..1_000 {
            cycle.nudge(FULL);
        }
        cycle.want(now, SystemTime::now(), FULL, rested(FULL), &mut rng);
        cycle.nudge(FULL);
        assert!(matches!(
            cycle.want(now, SystemTime::now(), FULL, rested(FULL), &mut rng),
            Want::Drain(_)
        ));
    }

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn timer() -> Cycle {
        Cycle::Timer {
            period: Duration::from_secs(1500),
            start: at(10_000),
        }
    }

    #[test]
    fn a_timer_counts_from_its_start_and_stops_when_full() {
        let cycle = timer();
        assert_eq!(cycle.asked(at(10_000), FULL), 0);
        assert_eq!(cycle.asked(at(10_750), FULL), FULL / 2);
        assert_eq!(cycle.asked(at(11_500), FULL), FULL);
        assert_eq!(cycle.asked(at(99_999), FULL), FULL);
    }

    #[test]
    fn a_full_timer_sits_still_instead_of_collapsing() {
        let mut cycle = timer();
        let mut rng = Rng::new(1);
        let now = Instant::now();
        for _ in 0..10 {
            assert_eq!(
                grains(cycle.want(now, at(99_999), FULL, rested(FULL), &mut rng)),
                Some(FULL),
                "a finished timer should stay full, not drain"
            );
        }
    }

    #[test]
    fn a_timer_takes_no_input() {
        let mut cycle = timer();
        assert!(!cycle.takes_input());
        assert!(endless(Instant::now()).takes_input());
        cycle.nudge(FULL);
        assert_eq!(cycle.asked(at(10_000), FULL), 0);
    }

    #[test]
    fn durations_need_a_number_and_a_unit() {
        assert_eq!(parse_duration("25m").unwrap(), Duration::from_secs(1500));
        assert_eq!(parse_duration("90s").unwrap(), Duration::from_secs(90));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert!(parse_duration("25").is_err());
        assert!(parse_duration("m").is_err());
        assert!(parse_duration("0m").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn a_timer_argument_switches_the_cycle() {
        let args = ["--timer".to_string(), "25m".to_string()];
        assert_eq!(
            timer_from_args(args.into_iter()).unwrap(),
            Some(Duration::from_secs(1500))
        );
        assert_eq!(timer_from_args(std::iter::empty()).unwrap(), None);
        assert!(timer_from_args(["--timer".to_string()].into_iter()).is_err());
        assert!(timer_from_args(["--wat".to_string()].into_iter()).is_err());
    }
}
