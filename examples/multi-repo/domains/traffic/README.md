# Traffic Domain

Core domain for realtime traffic flow, incidents, route monitoring, and roadworks.

## Services
- **api** — Realtime traffic API: flows, incidents, route monitoring,
  directions (proxies TomTom data from cache)
- **cars** — CARs/TMP integration with MyWorkSites and ArcGIS for roadworks
  features, worksites, layouts, and deployments
- **tomtom** — TomTom traffic feed ingestion: flows (with OpenLR decoding),
  incidents (XML), route monitoring; caches results to StateStore

## Shared Concepts
- **TrafficFlow** — Speed and freeflow data per OpenLR-decoded road segment
- **TrafficIncident** — Incident with type, validity period, and geometry
- **RouteSummary / RouteDetails** — Monitored route with segments and conditions
- **GeoIndex** — Spatial index for flow lookup by lat/lon/radius
- **Worksite / Tmp / Layout** — CARs roadworks hierarchy (worksite contains
  TMPs, each TMP has layouts and deployments)

## Event Contracts
None — this domain is HTTP-only with no messaging topics.
