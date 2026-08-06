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
  iptv-adapter-persistence/    implements PersistencePort — favorites & settings
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
provided by this library), and wire them into `IptvLibraryFacade`:

```rust
let facade = IptvLibraryFacade::new(
IptvCatalogService::new(repository.clone()),
IptvEpgService::new(repository.clone(), clock.clone()),
IptvFavoritesService::new(persistence),
IptvPlaybackController::new(player, resolver),
);

let channels = facade.get_catalog().search("news");
facade.play( & channel_id);
```

## Building

```
cargo build
cargo test
```

## What this library does *not* do

- No video rendering or player implementation — you supply a `PlayerController`
  adapter for your platform (e.g. GStreamer or mpv on desktop).
- No bundled UI of any kind.

