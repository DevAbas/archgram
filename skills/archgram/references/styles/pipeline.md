# Pipeline (pipes and filters)

- **Recognise:** stages that each take data in and pass it on: ETL jobs,
  `extract`/`transform`/`load`, DAGs (Airflow, Dagster, dbt), stream
  processors, a scheduler.
- **Parts:** sources, stages, the stores between them, the destination,
  where bad data goes, what schedules it.
- **Questions:** where does data enter, change and land? Batch or
  stream, on what schedule? Where is it checked?
- **Draw:** `direction: right`, one node per stage the reader must tell
  apart; a run of stages with nothing between them is one node. The flow
  is one record's way through.
