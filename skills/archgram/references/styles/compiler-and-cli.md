# Compiler, CLI and build tool

- **Recognise:** a command that reads input, runs it through stages and
  writes output: parse, validate, transform, render or emit; a `bin`
  entry; subcommands.
- **Parts:** the input, each stage, the output, and the tools or files
  each stage reads.
- **Questions:** what does it read and write? In which stage is each
  decision made? What is generated and must not be edited?
- **Draw:** `direction: right`, one node per stage; subcommands that run
  the same stages are one entry node, their names in its note.
