# Specification: r9k-xml-ingest

## Purpose

Replace SOAP/XML ingestion with HTTP/JSON endpoint.

## Requirements

### Requirement: HTTP endpoint
+ The system SHALL accept POST requests at `/api/r9k` with a JSON body.
  Source: r9k-http initiative

#### Scenario: Valid JSON payload
- Given: A well-formed JSON payload
- When: POST to `/api/r9k`
- Then: Returns 200 with accepted response

## Provider Capabilities
- HttpRequest: fetch upstream validation service
- Config: API_URL, AZURE_IDENTITY
- Publish: realtime-r9k.v1
