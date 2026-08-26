mod variables;

use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Input, InputContentType, InputEvent, InputState},
    *,
};
use gpui_highlight_input::{
    HighlightInput, HighlightInputEvent, HighlightInputState, HighlightSpan,
};

const INITIAL_URL: &str = "https://{{host}}/{{version}}/health";

struct Playground {
    url: Entity<InputState>,
    highlighted: Entity<HighlightInputState<SharedString>>,
    last_hover: Option<SharedString>,
    last_click: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl Playground {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url = cx.new(|cx| InputState::new(window, cx).default_value(INITIAL_URL));
        let highlighted = cx.new(|cx| HighlightInputState::new(url.clone(), cx));

        highlighted
            .update(cx, |state, cx| {
                state.set_spans(highlight_spans(INITIAL_URL), cx)
            })
            .expect("the demo parser returns valid UTF-8 ranges");

        let highlighted_on_change = highlighted.clone();
        let input_subscription = cx.subscribe(&url, move |_, input, event, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }

            let text = input.read(cx).value().to_string();
            highlighted_on_change
                .update(cx, |state, cx| state.set_spans(highlight_spans(&text), cx))
                .expect("the demo parser returns valid UTF-8 ranges");
        });

        let highlight_subscription = cx.subscribe(
            &highlighted,
            |this, _, event: &HighlightInputEvent<SharedString>, cx| {
                match event {
                    HighlightInputEvent::HoverChanged(hit) => {
                        this.last_hover = hit.as_ref().map(|hit| hit.payload.clone());
                    }
                    HighlightInputEvent::Clicked(hit) => {
                        this.last_click = Some(hit.payload.clone());
                    }
                }
                cx.notify();
            },
        );

        Self {
            url,
            highlighted,
            last_hover: None,
            last_click: None,
            _subscriptions: vec![input_subscription, highlight_subscription],
        }
    }
}

impl Render for Playground {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let last_hover = self.last_hover.clone().unwrap_or_else(|| "none".into());
        let last_click = self.last_click.clone().unwrap_or_else(|| "none".into());

        div()
            .v_flex()
            .gap_2()
            .size_full()
            .p_4()
            .bg(cx.theme().background)
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(Button::new("method").small().ghost().label("GET"))
                    .child(
                        div().flex_1().min_w_0().child(
                            HighlightInput::new(&self.highlighted).content(
                                Input::new(&self.url)
                                    .small()
                                    .appearance(false)
                                    .content_type(InputContentType::Url),
                            ),
                        ),
                    )
                    .child(Button::new("send").small().primary().label("Send")),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .px_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Hover: {last_hover}"))
                    .child(format!("Click: {last_click}")),
            )
    }
}

fn highlight_spans(text: &str) -> Vec<HighlightSpan<SharedString>> {
    variables::spans(text)
        .into_iter()
        .map(|span| HighlightSpan {
            range: span.range,
            payload: span.name.into(),
        })
        .collect()
}

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| Playground::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open demo window");
        })
        .detach();
    });
}
