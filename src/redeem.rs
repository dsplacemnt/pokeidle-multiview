//! Redeeming PokeIdle bonuses with channel points: the part every platform shares. Each account
//! chooses per platform what its points buy there (a fixed bonus, or its goals; or nothing), and
//! each platform module implements `Redeemer` to build the script that buys it in its pages.
//! When and how those scripts run stays with each platform (Kick: every check, in its live
//! pages; Twitch: during the ~30 s visit of the full page), so changing or adding a platform
//! never touches the others.

use crate::config::{Bonus, Goals, Platform, SlotConfig};
use crate::goals;

/// Loaded before every platform's redeem script: the rewards' names and how points read.
pub const COMMON: &str = include_str!("../scripts/redeem-common.js");

/// What a platform provides to redeem bonuses in its pages.
pub trait Redeemer: Sync {
    /// The script to run in one of this platform's pages to buy `bonus` (None = nothing to buy;
    /// a platform may still have something to do then, like Twitch's bonus chest). None = no
    /// script at all.
    fn script(&self, bonus: Option<Bonus>) -> Option<String>;
}

/// The redeemer of `platform`.
pub fn redeemer(platform: Platform) -> &'static dyn Redeemer {
    match platform {
        Platform::Twitch => &crate::twitch::Redeem,
        Platform::Kick => &crate::kick::Redeem,
    }
}

/// The name the page scripts give `bonus`.
pub fn kind(bonus: Bonus) -> &'static str {
    match bonus {
        Bonus::Xp => "xp",
        Bonus::Capture => "capture",
        Bonus::Lure => "lure",
    }
}

/// The account's goals, when they are what it redeems (streams-only accounts use a fixed bonus:
/// goals need the bonus times, which only the game knows).
fn goals_of(slot: &SlotConfig) -> Option<&Goals> {
    slot.goals.as_ref().filter(|_| !slot.streams_only)
}

/// Whether the account spends its points on `platform` at all: a fixed bonus there, or goals
/// that are on there.
pub fn redeems_on(slot: &SlotConfig, platform: Platform) -> bool {
    match goals_of(slot) {
        Some(goals) => !goals.off.get(platform),
        None => slot.bonus.get(platform).is_some(),
    }
}

/// What the account's points buy on `platform` now: its fixed bonus there, or in goals mode the
/// first goal not met there. `hours_left` gives the hours left of each bonus on this platform;
/// None while they're unknown (the game hasn't reported them, or a redeem is still being
/// counted), which buys nothing in goals mode.
pub fn bonus_now(slot: &SlotConfig, platform: Platform, hours_left: Option<&dyn Fn(Bonus) -> f64>) -> Option<Bonus> {
    let Some(goals) = goals_of(slot) else { return slot.bonus.get(platform) };
    if goals.off.get(platform) {
        return None;
    }
    goals::pick(goals, hours_left?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Goal, PerPlatform};
    use Bonus::{Capture, Xp};
    use Platform::{Kick, Twitch};

    fn with_goals(list: &[(Bonus, u16)]) -> SlotConfig {
        let mut slot = SlotConfig::new(1);
        slot.goals = Some(Goals { list: list.iter().map(|&(bonus, hours)| Goal { bonus, hours }).collect(), ..Goals::default() });
        slot
    }

    #[test]
    fn fixed_bonus_per_platform() {
        let mut slot = SlotConfig::new(1);
        slot.bonus = PerPlatform { twitch: Some(Capture), kick: Some(Xp) };
        assert_eq!(bonus_now(&slot, Twitch, None), Some(Capture));
        assert_eq!(bonus_now(&slot, Kick, None), Some(Xp));
        slot.bonus.twitch = None;
        assert!(!redeems_on(&slot, Twitch) && redeems_on(&slot, Kick));
        assert_eq!(bonus_now(&slot, Twitch, None), None);
    }

    #[test]
    fn goals_follow_each_platform_and_can_be_off() {
        let mut slot = with_goals(&[(Xp, 48), (Capture, 5)]);
        // Twitch has its 48 h of XP, Kick doesn't.
        let twitch = |b: Bonus| if b == Xp { 50.0 } else { 0.0 };
        let kick = |b: Bonus| if b == Xp { 10.0 } else { 0.0 };
        assert_eq!(bonus_now(&slot, Twitch, Some(&twitch)), Some(Capture));
        assert_eq!(bonus_now(&slot, Kick, Some(&kick)), Some(Xp));
        // Times not known yet: nothing.
        assert_eq!(bonus_now(&slot, Kick, None), None);
        slot.goals.as_mut().unwrap().off.set(Twitch, true);
        assert!(!redeems_on(&slot, Twitch) && redeems_on(&slot, Kick));
        assert_eq!(bonus_now(&slot, Twitch, Some(&twitch)), None);
        assert_eq!(bonus_now(&slot, Kick, Some(&kick)), Some(Xp));
    }

    #[test]
    fn streams_only_uses_the_fixed_bonus() {
        let mut slot = with_goals(&[(Xp, 48)]);
        slot.streams_only = true;
        slot.bonus.kick = Some(Capture);
        assert_eq!(bonus_now(&slot, Kick, Some(&|_| 0.0)), Some(Capture));
        assert!(!redeems_on(&slot, Twitch));
    }

    #[test]
    fn each_platform_builds_its_own_script() {
        let kick = redeemer(Kick).script(Some(Capture)).unwrap();
        assert!(kick.contains("__multiviewRewards") && kick.ends_with("__multiviewRedeem(\"capture\");"));
        // Kick has nothing to do without a bonus; Twitch still claims the chest.
        assert!(redeemer(Kick).script(None).is_none());
        assert!(redeemer(Twitch).script(None).unwrap().ends_with("__multiviewTwitchVisit(null);"));
        assert!(redeemer(Twitch).script(Some(Xp)).unwrap().ends_with("__multiviewTwitchVisit(\"xp\");"));
    }
}
