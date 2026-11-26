//! Smart redeem goals: an account keeps an ordered list like "XP ≥ 10h, then Capture ≥ 5h,
//! then Lure ≥ 3h", and its channel points buy the first goal that isn't met yet. Each
//! platform runs its bonuses on its own clock (Kick points only buy Kick bonuses), so the
//! choice is made per platform from that platform's remaining times.

use crate::config::{Bonus, Goals, WhenMet};

/// What to buy now, given how many hours of each bonus are left on this platform. None = buy
/// nothing (every goal met and the points are saved, or no goals).
pub fn pick(goals: &Goals, hours_left: impl Fn(Bonus) -> f64) -> Option<Bonus> {
    if let Some(goal) = goals.list.iter().find(|g| hours_left(g.bonus) < f64::from(g.hours)) {
        return Some(goal.bonus);
    }
    match goals.when_met {
        WhenMet::Save => None,
        WhenMet::KeepLast => goals.list.last().map(|g| g.bonus),
    }
}

/// Hours left of a bonus that ends at `end` (ms since 1970, 0 = not running) at `now`.
pub fn hours_left(end: i64, now: i64) -> f64 {
    if end <= now {
        0.0
    } else {
        (end - now) as f64 / 3_600_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Goal;
    use Bonus::{Capture, Lure, Xp};

    fn goals(list: &[(Bonus, u16)], when_met: WhenMet) -> Goals {
        Goals { list: list.iter().map(|&(bonus, hours)| Goal { bonus, hours }).collect(), when_met, ..Goals::default() }
    }

    fn left(xp: f64, capture: f64, lure: f64) -> impl Fn(Bonus) -> f64 {
        move |b| match b {
            Xp => xp,
            Capture => capture,
            Lure => lure,
        }
    }

    #[test]
    fn buys_the_first_goal_not_met() {
        let g = goals(&[(Xp, 10), (Capture, 5), (Lure, 3)], WhenMet::Save);
        assert_eq!(pick(&g, left(4.0, 0.0, 0.0)), Some(Xp));
        assert_eq!(pick(&g, left(12.0, 2.0, 0.0)), Some(Capture));
        assert_eq!(pick(&g, left(12.0, 6.0, 1.0)), Some(Lure));
        // Exactly at the goal counts as met.
        assert_eq!(pick(&g, left(10.0, 5.0, 2.9)), Some(Lure));
    }

    #[test]
    fn a_goal_that_runs_low_again_comes_first() {
        let g = goals(&[(Xp, 10), (Capture, 5)], WhenMet::Save);
        // Capture was being bought, then XP fell under its goal.
        assert_eq!(pick(&g, left(9.5, 3.0, 0.0)), Some(Xp));
    }

    #[test]
    fn when_every_goal_is_met() {
        let met = left(12.0, 6.0, 4.0);
        assert_eq!(pick(&goals(&[(Xp, 10), (Capture, 5), (Lure, 3)], WhenMet::Save), &met), None);
        assert_eq!(pick(&goals(&[(Xp, 10), (Capture, 5), (Lure, 3)], WhenMet::KeepLast), &met), Some(Lure));
        assert_eq!(pick(&goals(&[], WhenMet::KeepLast), &met), None);
    }

    #[test]
    fn hours_left_from_the_end_time() {
        let now = 1_000_000_000_000;
        assert_eq!(hours_left(0, now), 0.0);
        assert_eq!(hours_left(now - 5, now), 0.0);
        assert!((hours_left(now + 90 * 60_000, now) - 1.5).abs() < 1e-9);
    }
}
