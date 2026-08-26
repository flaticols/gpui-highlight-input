use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Entity, GlobalElementId, HighlightStyle,
    InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, Pixels, RenderOnce, Window,
};
use gpui_component::{ActiveTheme as _, Sizable as _, input::Input};

use crate::HighlightInputState;

#[derive(IntoElement)]
pub struct HighlightInput<P: Clone + PartialEq + 'static> {
    state: Entity<HighlightInputState<P>>,
    style: Option<HighlightStyle>,
    content: Option<AnyElement>,
}

impl<P: Clone + PartialEq + 'static> HighlightInput<P> {
    pub fn new(state: &Entity<HighlightInputState<P>>) -> Self {
        Self {
            state: state.clone(),
            style: None,
            content: None,
        }
    }

    pub fn highlight_style(mut self, style: HighlightStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Replace the stock presentation. The supplied element must render the
    /// same InputState returned by HighlightInputState::input_state.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
}

impl<P: Clone + PartialEq + 'static> RenderOnce for HighlightInput<P> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.read(cx).input_state().clone();
        let child = self
            .content
            .unwrap_or_else(|| Input::new(&input).small().into_any_element());
        let style = self.style.unwrap_or_else(|| HighlightStyle {
            background_color: Some(cx.theme().primary.opacity(0.1)),
            ..Default::default()
        });

        HighlightInputElement {
            state: self.state,
            style,
            child: Some(child),
        }
    }
}

struct HighlightInputElement<P: Clone + PartialEq + 'static> {
    state: Entity<HighlightInputState<P>>,
    style: HighlightStyle,
    child: Option<AnyElement>,
}

impl<P: Clone + PartialEq + 'static> IntoElement for HighlightInputElement<P> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<P: Clone + PartialEq + 'static> Element for HighlightInputElement<P> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("highlight input rendered once");
        let layout_id = child.request_layout(window, cx);
        (layout_id, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        let style = self.style.clone();
        self.state
            .update(cx, |state, cx| state.sync_style(style, cx));
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);

        let state = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, _, _, cx| {
            state.update(cx, |state, cx| state.update_hover(event.position, cx));
        });

        let state = self.state.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
            if phase.bubble() && event.button == MouseButton::Left {
                state.update(cx, |state, cx| state.click(event.position, cx));
            }
        });

        let state = self.state.clone();
        window.on_mouse_event(move |_: &MouseExitEvent, _, _, cx| {
            state.update(cx, |state, cx| state.clear_hover(cx));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::HighlightInput;
    use crate::{HighlightInputEvent, HighlightInputState, HighlightSpan};
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Bounds, Context, Entity, InteractiveElement as _, IntoElement, Modifiers,
        MouseButton, ParentElement as _, Pixels, Render, Styled as _, Subscription, TestAppContext,
        VisualTestContext, Window, div, point, px,
    };
    use gpui_component::{
        Sizable as _,
        input::{Input, InputState},
    };

    type Events = Rc<RefCell<Vec<HighlightInputEvent<String>>>>;

    struct HighlightHarness {
        input: Entity<InputState>,
        highlighted: Entity<HighlightInputState<String>>,
        _subscription: Subscription,
    }

    impl Render for HighlightHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            HighlightInput::new(&self.highlighted)
        }
    }

    fn show_highlighted<'a>(
        cx: &'a mut TestAppContext,
        value: String,
        spans: Vec<HighlightSpan<String>>,
        events: Events,
    ) -> (Entity<HighlightHarness>, &'a mut VisualTestContext) {
        cx.update(gpui_component::init);
        let captured_events = events.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
            let highlighted = cx.new(|cx| HighlightInputState::new(input.clone(), cx));
            highlighted.update(cx, |highlighted, cx| {
                highlighted.set_spans(spans, cx).unwrap();
            });
            let subscription = cx.subscribe(&highlighted, move |_, _, event, _| {
                captured_events.borrow_mut().push(event.clone());
            });
            HighlightHarness {
                input,
                highlighted,
                _subscription: subscription,
            }
        });
        cx.run_until_parked();
        (view, cx)
    }

    fn input_entity(
        view: &Entity<HighlightHarness>,
        cx: &mut VisualTestContext,
    ) -> Entity<InputState> {
        cx.update(|_, cx| view.read(cx).input.clone())
    }

    fn range_bounds(
        input: &Entity<InputState>,
        range: std::ops::Range<usize>,
        cx: &mut VisualTestContext,
    ) -> Bounds<Pixels> {
        cx.update(|_, cx| {
            input
                .read(cx)
                .range_to_bounds(&range)
                .expect("highlight range is laid out")
        })
    }

    fn hover_event_count(events: &Events) -> usize {
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, HighlightInputEvent::HoverChanged(Some(_))))
            .count()
    }

    #[gpui::test]
    fn rendered_pointer_events_are_deduplicated_cleared_and_clicked(cx: &mut TestAppContext) {
        let events = Events::default();
        let (view, cx) = show_highlighted(
            cx,
            "aa{{host}}:{{port}}zz".to_string(),
            vec![
                HighlightSpan {
                    range: 2..10,
                    payload: "host".to_string(),
                },
                HighlightSpan {
                    range: 11..19,
                    payload: "port".to_string(),
                },
            ],
            events.clone(),
        );
        let input = input_entity(&view, cx);
        let first = range_bounds(&input, 2..10, cx);
        let second = range_bounds(&input, 11..19, cx);

        cx.simulate_mouse_move(first.center(), None, Modifiers::default());
        cx.simulate_mouse_move(
            point(first.center().x + px(1.), first.center().y),
            None,
            Modifiers::default(),
        );
        assert_eq!(
            hover_event_count(&events),
            1,
            "movement inside one span is deduplicated"
        );

        let outside = point(first.origin.x - px(2.), first.center().y);
        cx.simulate_mouse_move(outside, None, Modifiers::default());
        assert_eq!(
            events.borrow().last(),
            Some(&HighlightInputEvent::HoverChanged(None))
        );

        cx.simulate_mouse_down(second.center(), MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(second.center(), MouseButton::Left, Modifiers::default());
        assert!(matches!(
            events.borrow().last(),
            Some(HighlightInputEvent::Clicked(hit)) if hit.payload == "port"
        ));
    }

    #[gpui::test]
    fn input_padding_does_not_hit_the_nearest_text_range(cx: &mut TestAppContext) {
        let events = Events::default();
        let (view, cx) = show_highlighted(
            cx,
            " {{host}} ".to_string(),
            vec![HighlightSpan {
                range: 1..9,
                payload: "host".to_string(),
            }],
            events.clone(),
        );
        let input = input_entity(&view, cx);
        let highlighted = range_bounds(&input, 1..9, cx);
        let input_bounds = cx.update(|_, cx| input.read(cx).input_bounds());
        let before = point(highlighted.left() - px(1.), highlighted.center().y);
        let after = point(highlighted.right() + px(1.), highlighted.center().y);

        cx.update(|_, cx| {
            let input = input.read(cx);
            assert!(input_bounds.contains(&before));
            assert!(input_bounds.contains(&after));
            assert!(input.text_offset_at_position(before).is_some());
            assert!(input.text_offset_at_position(after).is_some());
        });

        cx.simulate_mouse_move(before, None, Modifiers::default());
        cx.simulate_mouse_move(after, None, Modifiers::default());
        assert!(events.borrow().is_empty());
    }

    #[gpui::test]
    fn adjacent_ranges_check_the_previous_candidate_at_the_shared_boundary(
        cx: &mut TestAppContext,
    ) {
        let events = Events::default();
        let (view, cx) = show_highlighted(
            cx,
            "abcd".to_string(),
            vec![
                HighlightSpan {
                    range: 0..2,
                    payload: "left".to_string(),
                },
                HighlightSpan {
                    range: 2..4,
                    payload: "right".to_string(),
                },
            ],
            events.clone(),
        );
        let input = input_entity(&view, cx);
        let left = range_bounds(&input, 0..2, cx);
        let right = range_bounds(&input, 2..4, cx);
        assert_eq!(left.right(), right.left());
        let just_inside_left = point(right.left() - px(0.25), left.center().y);
        assert_eq!(
            cx.update(|_, cx| input.read(cx).text_offset_at_position(just_inside_left)),
            Some(2),
            "the nearest caret must select the current candidate for this regression"
        );

        cx.simulate_mouse_move(just_inside_left, None, Modifiers::default());
        assert!(matches!(
            events.borrow().last(),
            Some(HighlightInputEvent::HoverChanged(Some(hit))) if hit.payload == "left"
        ));
    }

    struct HeightHarness {
        input: Entity<InputState>,
        highlighted: Entity<HighlightInputState<String>>,
        stock: Entity<InputState>,
    }

    impl Render for HeightHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "highlight-custom-content".to_string())
                        .child(
                            HighlightInput::new(&self.highlighted)
                                .content(Input::new(&self.input).small().appearance(false)),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "stock-small-input".to_string())
                        .child(Input::new(&self.stock).small()),
                )
        }
    }

    #[gpui::test]
    fn custom_content_preserves_the_stock_inputs_intrinsic_height(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).default_value("custom"));
            let highlighted = cx.new(|cx| HighlightInputState::new(input.clone(), cx));
            let stock = cx.new(|cx| InputState::new(window, cx).default_value("stock"));
            HeightHarness {
                input,
                highlighted,
                stock,
            }
        });
        cx.run_until_parked();

        let custom = cx
            .debug_bounds("highlight-custom-content")
            .expect("custom highlighted input is rendered");
        let stock = cx
            .debug_bounds("stock-small-input")
            .expect("stock small input is rendered");
        assert!(
            custom.size.height > px(0.),
            "the height test must be non-vacuous"
        );
        assert_eq!(custom.size.height, stock.size.height);
    }

    #[gpui::test]
    fn a_visible_range_at_the_end_of_a_scrolled_value_emits_its_payload(cx: &mut TestAppContext) {
        let value = format!("{}END", "long-prefix/".repeat(160));
        let end = value.len();
        let events = Events::default();
        let (view, cx) = show_highlighted(
            cx,
            value,
            vec![HighlightSpan {
                range: end - 3..end,
                payload: "end".to_string(),
            }],
            events.clone(),
        );
        let input = input_entity(&view, cx);

        cx.update(|_, cx| {
            input.update(cx, |input, cx| {
                input.set_scroll_offset(point(px(-100_000.), px(0.)), cx);
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();

        let visible_end = range_bounds(&input, end - 3..end, cx);
        cx.update(|_, cx| {
            let input = input.read(cx);
            assert!(input.scroll_offset().x < px(0.));
            assert!(input.input_bounds().contains(&visible_end.center()));
        });
        cx.simulate_mouse_move(visible_end.center(), None, Modifiers::default());
        assert!(matches!(
            events.borrow().last(),
            Some(HighlightInputEvent::HoverChanged(Some(hit))) if hit.payload == "end"
        ));
    }
}
