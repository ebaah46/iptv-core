# iptv-core

A Rust library that fetches, models, and serves IPTV channel, stream, and EPG data
sourced from the [iptv-org](https://github.com/iptv-org/api) API. Other providers could be
integrated by extending the provided interfaces for the provider of your choice.

This is **library** contains no UI and no video rendering.
It exists so multiple UI applications (starting with a Desktop app, with TV
platforms as a future target) can share one implementation of catalog browsing/search,
stream resolution with automatic fallback, EPG lookup, and favorites, without
duplicating that logic per platform.

## Status

🚧 Implementation — current work is being done on core services.

## Architecture

Hexagonal (Ports & Adapters). The core crate contains domain logic only and has **no
I/O dependencies** — networking, file access, and persistence are implemented in
separate adapter crates and injected at startup by whatever application consumes this
library (the "composition root").

```
core/                          domain models, services, port traits (the hexagon)
adapters/
  iptv-adapter-rest/           implements ChannelDataSource — talks to the iptv-org API
  iptv-adapter-cache/          implements CacheStore — persists the catalog snapshot
  iptv-adapter-persistence/    implements FavoritesStore — persists favorite channel ids
tests/                         integration tests against mocked ports
```

## Using this library

Add it as a Cargo dependency (In the future, C ABIs may be provided):

```toml
[dependencies]
iptv-core = { git = "https://github.com/ebaah46/iptv-core", tag = "v0.1.0" }
iptv-adapter-rest = { git = "https://github.com/ebaah46/iptv-core", tag = "v0.1.0" }
iptv-adapter-cache = { git = "https://github.com/ebaah46/iptv-core", tag = "v0.1.0" }
iptv-adapter-persistence = { git = "https://github.com/ebaah46/iptv-core", tag = "v0.1.0" }
```

Your application acts as the composition root: construct the concrete adapters above,
plus your own `PlayerController` implementation (playback is platform-specific and not
provided by this library), and wire them into `IptvFacade`:

```rust
use core::facade::IptvFacade;
use core::services::{
    CatalogService, IptvCatalogService, IptvFavoritesService, IpTvPlaybackController,
};

let facade = IptvFacade::new(
    IptvCatalogService::new(repository.clone()),
    IptvFavoritesService::new(persistence, repository.clone()),
    IpTvPlaybackController::new(player, resolver),
);

let channels = facade.get_catalog_service().search("news");
facade.get_playback_controller().play(&channel_id);
```

The facade exposes service handles rather than mirroring their methods, so it
stays small as new services are added.

### Combined catalog queries

`CatalogQuery` combines search, country, language, category, and channel id
filters into a single pass over the catalog. Category matching supports all,
any, or none of the selected categories:

```rust
use core::{CatalogQuery, CategoryMatch};

let channels = facade.get_catalog_service().query(
    &CatalogQuery::new()
        .search("news")
        .country("US")
        .language("en")
        .categories(["news", "sports"])
        .category_match(CategoryMatch::Any),
);
```

- `CategoryMatch::All` — a channel must have every selected category.
- `CategoryMatch::Any` — a channel must have at least one selected category.
- `CategoryMatch::None` — a channel must have none of the selected categories.

Use `.channels([...])` to restrict results to specific channel ids (exact
match), which is useful for querying favorites. An empty selection applies no
channel filter.

An empty category selection applies no category filter. Queries return only
active channels by default; use `.active_only(false)` to include channels
without streams.

### Favorites

Favorites are stored as channel ids and resolved against the catalog on demand,
so they survive catalog refreshes and channel metadata changes. The core service
owns the add/remove/toggle logic; the persistence adapter supplies a
`FavoritesStore`:

```rust
use core::FavoritesService;

let favorites = facade.get_favorites_service();

favorites.add("BBCNews.uk")?;
favorites.toggle("BBCNews.uk")?;
let channels = favorites.get_favorites();
```

`get_favorite_ids()` returns the ids even before the catalog is loaded, while
`get_favorites()` resolves them to `Channel`s and skips ids that are no longer
in the catalog. Favorites are ordered most-recent-first.

## Building

```
cargo build
cargo test
```

## What this library does *not* do

- No video rendering or player implementation — you supply a `PlayerController`
  adapter for your platform (e.g. GStreamer or mpv on desktop).
- No bundled UI of any kind.

