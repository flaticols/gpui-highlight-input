# gpui-highlight-input

A lightweight wrapper around gpui-component's single-line InputState. The
application supplies validated UTF-8 byte ranges and semantic payloads; the
wrapper decorates them and emits hover/click events from one input-wide handler.

It does not parse templates, own popovers, reveal secrets, persist values, use
EditorState, or clone payloads while the pointer remains within one span.

The crate currently uses immutable git revisions and is not yet published to
crates.io.

## Example

The application owns parsing and decides what each payload means. It gives the
wrapper validated UTF-8 byte ranges, subscribes to semantic events, and keeps
using the same `InputState` for editing and rendering:

```rust
use gpui::{Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_component::input::InputState;
use gpui_highlight_input::{
    HighlightInput, HighlightInputEvent, HighlightInputState, HighlightSpan,
};

struct RequestInput {
    highlighted: Entity<HighlightInputState<&'static str>>,
    _subscription: Subscription,
}

impl RequestInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx).default_value("GET https://{{host}}/health")
        });
        let highlighted =
            cx.new(|cx| HighlightInputState::new(input, cx));

        // The application's parser produced this byte range and payload.
        highlighted
            .update(cx, |state, cx| {
                state.set_spans(
                    vec![HighlightSpan {
                        range: 12..20,
                        payload: "host",
                    }],
                    cx,
                )
            })
            .expect("the parser returned valid ranges");

        let subscription = cx.subscribe(&highlighted, |_, _, event, _| match event {
            HighlightInputEvent::HoverChanged(hit) => {
                // Show or hide application-owned help for hit.payload.
                let _ = hit;
            }
            HighlightInputEvent::Clicked(hit) => {
                // Open the application-owned editor for hit.payload.
                let _ = hit;
            }
        });

        Self {
            highlighted,
            _subscription: subscription,
        }
    }
}

impl Render for RequestInput {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        HighlightInput::new(&self.highlighted)
    }
}
```
