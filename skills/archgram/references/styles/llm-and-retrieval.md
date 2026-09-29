# LLM, retrieval and agents

- **Recognise:** calls to a model API, embeddings, a vector store,
  prompt files, tool definitions, an agent loop.
- **Parts:** what is prepared ahead (ingest, chunk, embed, index), what
  happens at question time (retrieve, prompt, call, check), the tools the
  model may call, what the answer is checked against.
- **Questions:** what does the model see, and never see? Which steps are
  model calls, and which plain code? Exact or semantic retrieval? What
  stops it inventing facts?
- **Draw:** a frame for ahead of time and one for question time, the
  stores between them; every model call its own node, so a reader can
  count them. Tools with the same relations are one node.
