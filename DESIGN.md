# F1 design boundary

This is schema and package scaffolding only. Runtime behavior, transport,
process handling, persistence implementation, and generated signal/meta crate
ownership remain subsequent work.

The F1 definition set is the atomic unit of configuration. Replacement must
validate the whole candidate before publication; the old set remains intact
on refusal. Recovery fences an uncertain mutation before its name can be
reused. Startup binds one canonical UTF-8-hex filename to the selected
configuration and refuses malformed, duplicate, or incomplete definitions.

The Ethos files are the checked contract source. Generated files are not
committed in this scaffold.
