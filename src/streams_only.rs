//! "Streams only" accounts: the account's game runs on another computer, and this app runs
//! only its Twitch and Kick streams here (points, chests, redeems). The bonuses follow the
//! Twitch/Kick login linked in the game, not the computer the game runs on, so they still
//! reach the account. The game itself can't open here too: the game keeps one connection per
//! account and would log the other computer out.
//!
//! Without the game, nobody tells the app how long the bonuses have left, so these accounts
//! show no bonus times (the game on the other computer has them) and redeem a fixed bonus.

use crate::config::SlotConfig;

/// Streams-only accounts redeem a fixed bonus: goals need the bonus times, which only the game
/// knows. An account with goals keeps its first goal's bonus.
pub fn fixed_only(slot: &mut SlotConfig) {
    if !slot.streams_only {
        return;
    }
    if let Some(goals) = slot.goals.take() {
        slot.bonus = goals.list.first().map(|g| g.bonus).or(slot.bonus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Goal, Goals, KickBonus};

    #[test]
    fn goals_become_their_first_bonus() {
        let mut slot = SlotConfig::new(1);
        slot.goals = Some(Goals { list: vec![Goal { bonus: KickBonus::Capture, hours: 5 }], ..Goals::default() });
        // Other slots keep their goals.
        fixed_only(&mut slot);
        assert!(slot.goals.is_some() && slot.bonus.is_none());
        let mut slot = SlotConfig { streams_only: true, ..slot };
        fixed_only(&mut slot);
        assert_eq!((slot.goals, slot.bonus), (None, Some(KickBonus::Capture)));
    }
}
