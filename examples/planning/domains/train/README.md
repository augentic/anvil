# Train Domain

Core domain for realtime train position tracking, passenger counting, and
GTFS feed generation.

## Services
- **common** — Shared domain logic: Block Management API client for vehicle
  allocations, Fleet API client for vehicle metadata
- **dilax-apc-connector** — Ingests Dilax APC payloads on `POST /api/apc`,
  publishes to `realtime-dilax-apc.v2`
- **dilax-adapter** — Enriches Dilax APC events with trip, stop, and vehicle
  data; exposes a scheduled detector for lost APC connections
- **r9k-connector** — Ingests R9K SOAP/XML on `POST /inbound/xml`, validates,
  publishes to `realtime-r9k.v1`
- **r9k-adapter** — Transforms R9K XML into SmarTrak-compatible events,
  publishes to `realtime-r9k-to-smartrak.v1`
- **smartrak-gtfs** — Handles SmarTrak, CAF AVL, Train AVL, and passenger count
  messages; produces GTFS VehiclePosition and dead-reckoning feeds; provides
  vehicle info lookup and god-mode trip overrides

## Shared Concepts
- **Allocation** — Block allocation linking a vehicle to a trip, route, and
  service date (from Block Management API)
- **Vehicle** — Fleet vehicle with id, label, registration, capacity, type, and
  tag (e.g. `caf`, `smartrak`)
- **Identifier** — Label or Id variant for vehicle lookup (AM/AMP/AD/ADL
  patterns)

## Event Contracts
- `realtime-r9k.v1` — Published by r9k-connector, consumed by r9k-adapter
- `realtime-r9k-to-smartrak.v1` — Published by r9k-adapter, consumed by
  smartrak-gtfs
- `realtime-dilax-apc.v2` — Published by dilax-apc-connector, consumed by
  dilax-adapter
- `realtime-dilax-apc-enriched.v2` — Published by dilax-adapter
- `realtime-caf-avl.v1` — Consumed by smartrak-gtfs (CAF vehicle location)
- `realtime-train-avl.v1` — Consumed by smartrak-gtfs (SmarTrak vehicle
  location)
- `realtime-passenger-count.v1` — Consumed by smartrak-gtfs (occupancy to
  StateStore)
- `realtime-gtfs-vp.v1` — Published by smartrak-gtfs (GTFS VehiclePosition
  feed)
- `realtime-dead-reckoning.v1` — Published by smartrak-gtfs (dead-reckoning
  feed)
