//! Campaigns (prompt 189): an order of missions with texts between them.
//!
//! A campaign is a TOML file `data/campaigns/<id>.toml` (format in
//! `docs/MISSIONS.md`). It only orders mission files and names the texts
//! shown before and after each one; unlocking and progress live in the
//! menu (`user://missions.cfg`). Nothing carries over between missions
//! (no core force), so the simulation never reads a campaign.

use crate::mission::MissionError;
use serde::Deserialize;

/// Folder of the campaign files, in `data/`.
pub const CAMPAIGNS_DIR: &str = "campaigns";

/// A campaign file.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub campaign: CampaignHeader,
    #[serde(default, rename = "mission")]
    pub missions: Vec<CampaignMission>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignHeader {
    pub title: String,
    #[serde(default)]
    pub briefing: String,
    /// Position in the campaign list.
    #[serde(default)]
    pub order: u32,
}

/// One mission of a campaign, in play order.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignMission {
    /// The mission file's id (file name without `.toml`).
    pub id: String,
    /// Its map (folder name), where the file lies.
    pub map: String,
    /// Shown before the mission starts.
    #[serde(default)]
    pub intro: String,
    /// Shown once it was won.
    #[serde(default)]
    pub outro: String,
    /// Optional short season/date label for the campaign timeline and chapter card (prompt 237).
    #[serde(default)]
    pub date_label: String,
    /// Optional short chapter tagline shown on the chapter card and details (prompt 237).
    #[serde(default)]
    pub tagline: String,
}

impl Campaign {
    /// Parses a campaign file: at least one mission, no mission twice.
    pub fn parse(text: &str) -> Result<Self, MissionError> {
        let c: Self = toml::from_str(text).map_err(|e| MissionError(e.to_string()))?;
        if c.missions.is_empty() {
            return Err(MissionError("a campaign needs a [[mission]]".into()));
        }
        for (i, m) in c.missions.iter().enumerate() {
            if m.id.is_empty() || m.map.is_empty() {
                return Err(MissionError(format!("mission {i} needs `id` and `map`")));
            }
            if c.missions[..i].iter().any(|o| o.id == m.id) {
                return Err(MissionError(format!("mission '{}' is listed twice", m.id)));
            }
        }
        Ok(c)
    }

    /// Every text the player reads, in file order without repeats (the
    /// keys of its translations, like `mission::mission_texts`).
    pub fn texts(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let all = [&self.campaign.title, &self.campaign.briefing]
            .into_iter()
            .chain(
                self.missions
                    .iter()
                    .flat_map(|m| [&m.date_label, &m.tagline, &m.intro, &m.outro]),
            );
        for t in all {
            if !t.is_empty() && !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaigns_parse_and_bad_ones_fail() {
        let text = r#"
[campaign]
title = "T"
briefing = "B"
[[mission]]
id = "a"
map = "m"
date_label = "Spring"
tagline = "First thrust"
intro = "I"
outro = "O"
[[mission]]
id = "b"
map = "m"
intro = "I"
"#;
        let c = Campaign::parse(text).unwrap();
        assert_eq!(c.missions.len(), 2);
        assert_eq!(c.missions[0].date_label, "Spring");
        assert_eq!(c.missions[0].tagline, "First thrust");
        assert_eq!(c.missions[1].date_label, "");
        assert_eq!(c.texts(), ["T", "B", "Spring", "First thrust", "I", "O"]);
        assert!(Campaign::parse("[campaign]\ntitle = \"T\"\n").is_err());
        let twice = text.replace("id = \"b\"", "id = \"a\"");
        assert!(Campaign::parse(&twice).is_err());
        assert!(Campaign::parse(&format!("{text}\nextra = 1\n")).is_err());
    }
}
