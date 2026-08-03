use crate::dto::{CategoryDTO, ChannelDTO, CountryDTO, FeedDTO, LanguageDTO, StreamDTO};
use chrono::NaiveDate;
use core::domain::{Category, Channel, Country, Feed, Language, Stream};
use std::str::FromStr;
/**
* This mapper converts the DTOs from the various APIs into the domain
* models that are supported by this library.
*/

const DATE_FORMAT: &str = "%Y-%m-%d";
pub fn map_channel_dto(dto: ChannelDTO) -> Option<Channel> {
    Some(Channel {
        id: dto.id,
        name: dto.name,
        alt_names: dto.alt_names,
        category_ids: dto.categories,
        country_code: dto.country,
        is_nsfw: dto.is_nsfw,
        launched: dto.launched.and_then(|d| NaiveDate::from_str(&d).ok()),
        closed: dto.closed.and_then(|d| NaiveDate::from_str(&d).ok()),
        website: dto.website,
        network: dto.network.unwrap_or_default(),
    })
}

pub fn map_stream_dto(dto: StreamDTO) -> Option<Stream> {
    Some(Stream {
        channel_id: dto.channel.unwrap_or_default(),
        feed_id: dto.feed.unwrap_or_default(),
        url: Some(dto.url).filter(|s| !s.is_empty())?,
        quality: dto.quality.unwrap_or_default(),
        referrer: dto.referrer.unwrap_or_default(),
        title: Some(dto.title).filter(|t| !t.is_empty())?,
        user_agent: dto.user_agent.unwrap_or_default(),
    })
}

pub fn map_feed_dto(dto: FeedDTO) -> Option<Feed> {
    Some(Feed {
        id: Some(dto.id).filter(|id| id.is_empty())?,
        channel_id: Some(dto.channel).filter(|id| id.is_empty())?,
        name: Some(dto.name).filter(|id| id.is_empty())?,
        broadcast_codes: dto.broadcast_areas,
        language_codes: dto.languages,
        is_main: false,
    })
}

pub fn map_language_dto(dto: LanguageDTO) -> Option<Language> {
    Some(Language {
        code: dto.code,
        name: dto.name,
    })
}
pub fn map_country_dto(dto: CountryDTO) -> Option<Country> {
    Some(Country {
        code: dto.code,
        name: dto.name,
        languages: dto.languages,
        flag_url: dto.flag,
    })
}

pub fn map_category_dto(dto: CategoryDTO) -> Option<Category> {
    Some(Category {
        id: dto.id,
        name: dto.name,
        description: dto.description,
    })
}
