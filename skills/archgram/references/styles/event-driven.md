# Event-driven

- **Recognise:** producers and consumers of named events or topics; a
  broker (Kafka, RabbitMQ, SQS, NATS, Redis streams); handlers registered
  by event name; sometimes an event store.
- **Parts:** producers, the broker or topics, consumers, what consumers
  write.
- **Questions:** who reacts to what? What happens if a consumer fails,
  and in what order do things happen?
- **Draw:** the broker or each topic that matters as a queue node between
  producers and consumers; a flow per event the reader asks about.
  Consumers of one event with the same relations are one node.
