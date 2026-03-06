# Specification: r9k-xml-to-smartrak-gtfs

## Purpose

Consume new JSON event format from r9k-connector (replaces XML transform).

## Requirements

### Requirement: JSON event consumption
+ The system SHALL consume events from `realtime-r9k.v2` (JSON format).
- The system SHALL consume events from `realtime-r9k.v1` (XML format).
  Source: r9k-http initiative

#### Scenario: JSON event processing
- Given: A JSON event on `realtime-r9k.v2`
- When: Event is received
- Then: Transforms to SmarTrak-compatible format and publishes to `realtime-r9k-to-smartrak.v1`

## Provider Capabilities
- Config: API_URL
- Publish: realtime-r9k-to-smartrak.v1
