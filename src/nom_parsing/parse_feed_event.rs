use nom::{branch::alt, bytes::complete::tag, character::complete::{i16, u8}, combinator::opt, error::context, multi::many1, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use tracing::error;

use crate::{enums::FeedEventType, feed_event::{AttributeChange, AttributeEqual, FeedEvent, FeedEventParseError, ParsedFeedEventText}, nom_parsing::shared::{emoji_team_eof, emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word}};

use super::shared::Error;

trait FeedEventParser<'output>: Parser<&'output str, Output = ParsedFeedEventText<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedFeedEventText<&'output str>, Error = Error<'output>>> FeedEventParser<'output> for T {}


pub fn parse_feed_event<'output>(event: &'output FeedEvent) -> ParsedFeedEventText<&'output str> {
    let event_type = match &event.event_type {
        Ok(event_type) => event_type,
        Err(e) => {
            let error = FeedEventParseError::EventTypeNotRecognized(e.clone());
            return ParsedFeedEventText::ParseError { error, text: &event.text };
        }
    };

    let result = match event_type {
        FeedEventType::Game => game().parse(&event.text),
        FeedEventType::Augment => augment().parse(&event.text),
    };
    match result.finish() {
        Ok(("", output)) => output,
        Ok((leftover, _)) => {
            error!("{event_type} feed event parsed had leftover: {leftover} from {}", &event.text);
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            ParsedFeedEventText::ParseError { error, text: &event.text }
        }
        Err(_) => {
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            tracing::error!("Parse error: {}", error);
            ParsedFeedEventText::ParseError { error, text: &event.text }
        }
    }
}

fn game<'output>() -> impl FeedEventParser<'output> {
    context("Game Feed Event", alt((
        game_result(),
        feed_delivery("Delivery").map(|delivery| ParsedFeedEventText::Delivery { delivery } ),
        feed_delivery("Shipment").map(|delivery| ParsedFeedEventText::Shipment { delivery } ),
        feed_delivery("Special Delivery").map(|delivery| ParsedFeedEventText::SpecialDelivery { delivery } ),
        hit_by_falling_star()
    )))
}

fn hit_by_falling_star<'output>() -> impl FeedEventParser<'output> {
    parse_terminated(" was hit by a Falling Star!").map(|player| ParsedFeedEventText::HitByFallingStar { player })
}

fn game_result<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" vs. ").and_then(emoji_team_eof),
        parse_terminated(" - ").and_then(emoji_team_eof),
        preceded(tag("FINAL "), separated_pair(u8, tag("-"), u8))
    ).map(|(away_team, home_team, (away_score, home_score))| 
        ParsedFeedEventText::GameResult { home_team, away_team, home_score, away_score }
    )
}

fn augment<'output>() -> impl FeedEventParser<'output> {
    context("Augment Feed Event", alt((
        attribute_gain(),
        enchantment_s1a(),
        enchantment_s1b(),
        enchantment_s2(),
        enchantment_compensatory(),
        robo(),
        take_the_mound(),
        take_the_plate(),
        attribute_equal_1(),
        attribute_equal_2(),
        attribute_equal_3(),
        swap_places()
    )))
}

fn attribute_gain<'output>() -> impl FeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated(" gained +")),
            i16,
            delimited(tag(" "), try_from_word, tag("."))
        ).map(|(player_name, amount, attribute)| AttributeChange { player_name, amount, attribute })
    ).map(|changes| ParsedFeedEventText::AttributeChanges { changes })
}

fn attribute_equal_1<'output>() -> impl FeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated("'s ")),
            try_from_word,
            delimited(tag(" was set to their "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| AttributeEqual { player_name, changing_attribute, value_attribute })
    ).map(|equals| ParsedFeedEventText::AttributeEquals { equals })
}

fn attribute_equal_2<'output>() -> impl FeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated("'s ")),
            try_from_word,
            delimited(tag(" became equal to their base "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| AttributeEqual { player_name, changing_attribute, value_attribute })
    ).map(|equals| ParsedFeedEventText::AttributeEquals { equals })
}

fn attribute_equal_3<'output>() -> impl FeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated("'s ")),
            try_from_word,
            delimited(tag(" became equal to their current base "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| AttributeEqual { player_name, changing_attribute, value_attribute })
    ).map(|equals| ParsedFeedEventText::AttributeEquals { equals })
}

fn enchantment_s1a<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated("'s "),
        emojiless_item,
        preceded(tag(" was enchanted with +"), u8),
        delimited(tag(" to "), try_from_word, tag("."))
    ).map(|(player_name, item, amount, attribute)| ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute })
}

fn enchantment_s1b<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        delimited(tag(" gained a +"), u8, tag(" ")),
        terminated(try_from_word, tag(" bonus."))
    ).map(|(player_name, item, amount, attribute)| ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute })
}

fn enchantment_s2<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
        delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag(".")),
    ).map(|(player_name, item, (amount, attribute), enchant_two)| ParsedFeedEventText::S2Enchantment { player_name, item, amount, attribute, enchant_two: Some(enchant_two), compensatory: false })
}

fn enchantment_compensatory<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("The Compensatory Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        alt((
            (
                preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
                delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag("."))
            ).map(|((amount, attribute), second)| (amount, attribute, Some(second))),
            (
                delimited(tag(" gained a +"), separated_pair(u8, tag(" "), try_from_word), tag(" bonus."))
                .map(|(amount, attribute)| (amount, attribute, None))
            )
        ))
    ).map(|(player_name, item, (amount, attribute, enchant_two))| ParsedFeedEventText::S2Enchantment { player_name, item, amount, attribute, enchant_two, compensatory: true })
}

fn robo<'output>() -> impl FeedEventParser<'output> {
    parse_terminated(" gained the ROBO Modification.")
        .map(|player_name| ParsedFeedEventText::ROBO { player_name })
}

fn take_the_mound<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" was moved to the mound. "),
        parse_terminated(" was sent to the lineup."),
    )
        .map(|(to_mound_player, to_lineup_player)| ParsedFeedEventText::TakeTheMound { to_mound_player, to_lineup_player })
}

fn take_the_plate<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" was sent to the plate. "),
        parse_terminated(" was pulled from the lineup."),
    )
        .map(|(to_plate_player, from_lineup_player)| ParsedFeedEventText::TakeThePlate { to_plate_player, from_lineup_player })
}

fn swap_places<'output>() -> impl FeedEventParser<'output> {
    sentence_eof((
        parse_terminated(" swapped places with "),
        name_eof
    ))
    .map(|(player_one, player_two)| ParsedFeedEventText::SwapPlaces { player_one, player_two })
}

#[cfg(test)]
mod test {
    use nom::Parser;

    use crate::{enums::Attribute, feed_event::{AttributeChange, ParsedFeedEventText}, nom_parsing::parse_feed_event::{attribute_gain, game_result}, parsed_event::EmojiTeam};

    #[test]
    fn test_attribute_gain() {
        assert_eq!(
            Ok(ParsedFeedEventText::AttributeChanges { changes: vec![AttributeChange { player_name: "Nancy Bright", amount: 50, attribute: Attribute::Awareness}] }),
            attribute_gain().parse("Nancy Bright gained +50 Awareness.").map(|(_, o)| o).map_err(|e| e.to_string())
        );
    }

    #[test]
    fn test_game_result() {
        let s = "🦖 Peoria Monster Monster Monster vs. 📮 Akron Anteaters Pace Stick - FINAL 2-4";
        assert_eq!(
            Ok(ParsedFeedEventText::GameResult { away_team: EmojiTeam {emoji: "🦖", name: "Peoria Monster Monster Monster"}, home_team: EmojiTeam { emoji: "📮", name: "Akron Anteaters Pace Stick" }, away_score: 2, home_score: 4 }),
            game_result().parse(s).map(|(_, o)| o).map_err(|e| e.to_string())
        );
    }
}