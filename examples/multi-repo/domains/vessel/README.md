# Vessel Domain

Core domain for vessel tracking, identification, and fleet management.

## Services
- **ais-connector** — Ingests AIS position reports, validates, publishes
- **position-tracker** — Maintains position history, geofence alerting
- **fleet-manager** — Fleet composition, vessel-to-fleet assignment

## Shared Concepts
- **MMSI** — Maritime Mobile Service Identity (9-digit vessel identifier)
- **IMO** — International Maritime Organization number (7-digit)
- **Position** — { latitude, longitude, heading, speed, timestamp }

## Event Contracts
- `vessel-position` — Published by ais-connector, consumed by
  position-tracker, event-gateway
- `geofence-alert` — Published by position-tracker, consumed by
  notification-router