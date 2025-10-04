use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use crate::{enums::{EventType, Inning}, game::{EventBatterVersions, EventPitcherVersions, Pitch}, utils::{extra_fields_deserialize, MaybeRecognizedResult, NonStringOrEmptyString}};
use crate::utils::MaybeRecognizedHelper;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RawEvent {
    /// 0 is before the game has started
    pub inning: u8,
    
    /// 2 when the game is over
    pub inning_side: u8,

    pub away_score: u8,
    pub home_score: u8,

    pub balls: Option<u8>,
    pub strikes: Option<u8>,
    pub outs: Option<u8>,

    pub on_1b: bool,
    pub on_2b: bool,
    pub on_3b: bool,
    
    /// Empty string between innings, null before game
    pub on_deck: EventBatterVersions<String>,
    /// Empty string between innings, null before game
    pub batter: EventBatterVersions<String>,
    /// Empty string between innings, null before game
    pub pitcher: EventPitcherVersions<String>,

    /// Empty if none
    pub pitch_info: String,

    #[serde_as(as = "NonStringOrEmptyString")]
    pub zone: Option<u8>,

    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub event: MaybeRecognizedResult<EventType>,
    pub message: String,

    #[serde_as(as = "NonStringOrEmptyString")]
    pub index: Option<u16>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(from = "RawEvent", into = "RawEvent")]
pub struct Event {
    pub inning: Inning,

    pub away_score: u8,
    pub home_score: u8,

    pub balls: Option<u8>,
    pub strikes: Option<u8>,
    pub outs: Option<u8>,

    pub on_1b: bool,
    pub on_2b: bool,
    pub on_3b: bool,
    
    pub on_deck: EventBatterVersions<String>,
    pub batter: EventBatterVersions<String>,
    pub pitcher: EventPitcherVersions<String>,

    pub pitch: Option<Pitch>,

    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub event: MaybeRecognizedResult<EventType>,
    pub message: String,

    pub index: Option<u16>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
impl From<RawEvent> for Event {
    fn from(value: RawEvent) -> Self {
        let inning = match (value.inning, value.inning_side) {
            (0, 1) => Inning::BeforeGame,
            (0, 2) => Inning::AfterGame { final_inning_number: 1 },
            (number, 2) => Inning::AfterGame { final_inning_number: number - 1 },
            (number, side) => Inning::DuringGame { number, batting_side: side.try_into().unwrap() }
        };
        let pitch_info = (!value.pitch_info.is_empty()).then_some(value.pitch_info);

        let pitch = pitch_info.zip(value.zone).map(|(pitch_info, zone)| Pitch::new(pitch_info, zone));

        Self {inning, pitch, batter: value.batter, pitcher: value.pitcher, on_deck: value.on_deck, event: value.event, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message, extra_fields: value.extra_fields, index: value.index }
    }
}
impl From<Event> for RawEvent {
    fn from(value: Event) -> Self {
        let (inning, inning_side) = match value.inning {
            Inning::BeforeGame => (0, 1),
            Inning::DuringGame { number, batting_side: side } => (number, side.into()),
            Inning::AfterGame { final_inning_number: 1 } => (0, 2),
            Inning::AfterGame { final_inning_number } => (final_inning_number + 1, 2)
        };
        let (pitch_info, zone) = value.pitch.map(Pitch::unparse).map(|(pitch, zone)| (pitch, Some(zone))).unwrap_or(("".to_string(), None));

        Self {inning, inning_side, pitch_info, zone, event: value.event, batter: value.batter, on_deck: value.on_deck, pitcher: value.pitcher, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message, extra_fields: value.extra_fields, index: value.index.into() }
    }
}
