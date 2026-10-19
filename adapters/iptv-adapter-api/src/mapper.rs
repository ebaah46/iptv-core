use crate::dto::{CategoryDTO, ChannelDTO, CountryDTO, FeedDTO, LanguageDTO, LogoDTO, StreamDTO};
use chrono::NaiveDate;
use core::domain::{Category, Channel, Country, Feed, Language, Logo, Stream};
use std::str::FromStr;
use std::sync::Arc;

/**
* This mapper converts the DTOs from the various APIs into the domain
* models that are supported by this library.
*/

pub fn map_channel_dto(dto: ChannelDTO) -> Option<Arc<Channel>> {
    Some(Arc::new(Channel {
        id: dto.id.to_ascii_lowercase(),
        name: dto.name.to_ascii_lowercase(),
        alt_names: dto
            .alt_names
            .into_iter()
            .map(|name| name.to_ascii_lowercase())
            .collect(),
        category_ids: dto.categories,
        country_code: dto.country,
        is_nsfw: dto.is_nsfw,
        launched: dto.launched.and_then(|d| NaiveDate::from_str(&d).ok()),
        closed: dto.closed.and_then(|d| NaiveDate::from_str(&d).ok()),
        website: dto.website.unwrap_or_default(),
        network: dto.network.unwrap_or_default(),
    }))
}

pub fn map_stream_dto(dto: StreamDTO) -> Option<Arc<Stream>> {
    Some(Arc::new(Stream {
        channel_id: dto.channel.unwrap_or_default().to_ascii_lowercase(),
        feed_id: dto.feed.unwrap_or_default().to_ascii_lowercase(),
        url: Some(dto.url).filter(|s| !s.is_empty())?,
        quality: dto.quality.unwrap_or_default(),
        referrer: dto.referrer.unwrap_or_default(),
        title: Some(dto.title).filter(|t| !t.is_empty())?,
        user_agent: dto.user_agent.unwrap_or_default(),
    }))
}

pub fn map_feed_dto(dto: FeedDTO) -> Option<Arc<Feed>> {
    Some(Arc::new(Feed {
        id: Some(dto.id.to_ascii_lowercase()).filter(|id| !id.is_empty())?,
        channel_id: Some(dto.channel.to_ascii_lowercase()).filter(|id| !id.is_empty())?,
        name: Some(dto.name.to_ascii_lowercase()).filter(|id| !id.is_empty())?,
        broadcast_codes: dto.broadcast_area,
        language_codes: dto.languages,
        is_main: false,
    }))
}

pub fn map_language_dto(dto: LanguageDTO) -> Option<Arc<Language>> {
    Some(Arc::new(Language {
        code: dto.code.to_ascii_lowercase(),
        name: dto.name.to_ascii_lowercase(),
    }))
}
pub fn map_country_dto(dto: CountryDTO) -> Option<Arc<Country>> {
    Some(Arc::new(Country {
        code: dto.code.to_ascii_lowercase(),
        name: dto.name.to_ascii_lowercase(),
        languages: dto.languages,
        flag_url: dto.flag,
    }))
}

pub fn map_category_dto(dto: CategoryDTO) -> Option<Arc<Category>> {
    Some(Arc::new(Category {
        id: dto.id.to_ascii_lowercase(),
        name: dto.name.to_ascii_lowercase(),
        description: dto.description,
    }))
}

pub fn map_logo_dto(dto: LogoDTO) -> Option<Arc<Logo>> {
    Some(Arc::new(Logo {
        channel: dto.channel.to_ascii_lowercase(),
        format: dto
            .format
            .map(|f| f.to_ascii_lowercase())
            .unwrap_or_default(),
        url: dto.url,
        width: dto.width,
        height: dto.height,
    }))
}
