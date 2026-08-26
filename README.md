# gpui-highlight-input

A lightweight wrapper around gpui-component's single-line InputState. The
application supplies validated UTF-8 byte ranges and semantic payloads; the
wrapper decorates them and emits hover/click events from one input-wide handler.

It does not parse templates, own popovers, reveal secrets, persist values, use
EditorState, or allocate while the pointer moves.

The crate currently uses immutable git revisions and is not yet published to
crates.io.
