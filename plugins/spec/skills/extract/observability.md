# Metrics and observability (Step 6)

For each metric emission in the source code (counters, gauges, histograms, log-structured events):

- Metric name and type (counter, gauge, histogram)
- When it is emitted (which step in the algorithm)
- Dimensions/labels attached
- Purpose (operational visibility, alerting, debugging)

Example artifacts:

```markdown
- **Metrics**:
  - `events_published` — type: monotonic counter; emitted: after each successful publish; labels: none
  - `irrelevant_station` — type: monotonic counter; emitted: when station is filtered out; labels: station ID
  - `r9k_delay` — type: gauge; emitted: during validation; labels: none; value: message delay in seconds
```
