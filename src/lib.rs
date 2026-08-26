mod element;

pub use element::HighlightInput;

use std::ops::Range;

use gpui::{Bounds, Context, Entity, EventEmitter, HighlightStyle, Pixels, Point};
use gpui_component::input::{InputState, TextDecoration};

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightSpan<P> {
    pub range: Range<usize>,
    pub payload: P,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HighlightSpanError {
    EmptyRange {
        range: Range<usize>,
    },
    OutOfBounds {
        range: Range<usize>,
        text_len: usize,
    },
    InvalidCharBoundary {
        offset: usize,
    },
    Overlap {
        previous: Range<usize>,
        current: Range<usize>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightHit<P> {
    pub payload: P,
    pub range: Range<usize>,
    pub bounds: Bounds<Pixels>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HighlightInputEvent<P> {
    HoverChanged(Option<HighlightHit<P>>),
    Clicked(HighlightHit<P>),
}

#[derive(Clone, Debug, PartialEq)]
struct HoverTarget {
    index: usize,
    range: Range<usize>,
    bounds: Bounds<Pixels>,
}

pub struct HighlightInputState<P> {
    input: Entity<InputState>,
    spans: Vec<HighlightSpan<P>>,
    style: HighlightStyle,
    hovered: Option<HoverTarget>,
    #[cfg(test)]
    decoration_epoch: usize,
}

fn validate_spans<P>(
    text: &str,
    mut spans: Vec<HighlightSpan<P>>,
) -> Result<Vec<HighlightSpan<P>>, HighlightSpanError> {
    spans.sort_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then(left.range.end.cmp(&right.range.end))
    });

    let mut previous: Option<Range<usize>> = None;
    for span in &spans {
        let range = span.range.clone();
        if range.is_empty() {
            return Err(HighlightSpanError::EmptyRange { range });
        }
        if range.start > text.len() || range.end > text.len() {
            return Err(HighlightSpanError::OutOfBounds {
                range,
                text_len: text.len(),
            });
        }
        for offset in [range.start, range.end] {
            if !text.is_char_boundary(offset) {
                return Err(HighlightSpanError::InvalidCharBoundary { offset });
            }
        }
        if let Some(previous) = &previous
            && range.start < previous.end
        {
            return Err(HighlightSpanError::Overlap {
                previous: previous.clone(),
                current: range,
            });
        }
        previous = Some(range);
    }

    Ok(spans)
}

impl<P: Clone + PartialEq + 'static> HighlightInputState<P> {
    pub fn new(input: Entity<InputState>, _cx: &mut Context<Self>) -> Self {
        Self {
            input,
            spans: Vec::new(),
            style: HighlightStyle::default(),
            hovered: None,
            #[cfg(test)]
            decoration_epoch: 0,
        }
    }

    pub fn input_state(&self) -> &Entity<InputState> {
        &self.input
    }

    fn candidate_at(&self, position: Point<Pixels>, cx: &gpui::App) -> Option<HoverTarget> {
        let input = self.input.read(cx);
        let offset = input.text_offset_at_position(position)?;
        let split = self
            .spans
            .partition_point(|span| span.range.start <= offset);
        let primary = split.checked_sub(1);
        let previous = primary.and_then(|index| index.checked_sub(1));

        for index in [primary, previous].into_iter().flatten() {
            let span = &self.spans[index];
            let Some(bounds) = input.range_to_bounds(&span.range) else {
                continue;
            };
            if bounds.contains(&position) {
                return Some(HoverTarget {
                    index,
                    range: span.range.clone(),
                    bounds,
                });
            }
        }
        None
    }

    fn hit_for_target(&self, target: &HoverTarget) -> HighlightHit<P> {
        let span = &self.spans[target.index];
        HighlightHit {
            payload: span.payload.clone(),
            range: span.range.clone(),
            bounds: target.bounds,
        }
    }

    fn update_hover(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let candidate = self.candidate_at(position, cx);
        if candidate.as_ref() == self.hovered.as_ref() {
            return;
        }

        let hit = candidate.as_ref().map(|target| self.hit_for_target(target));
        self.hovered = candidate;
        cx.emit(HighlightInputEvent::HoverChanged(hit));
    }

    fn click(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(target) = self.candidate_at(position, cx) else {
            return;
        };
        let hit = self.hit_for_target(&target);
        cx.emit(HighlightInputEvent::Clicked(hit));
    }

    pub fn set_spans(
        &mut self,
        spans: Vec<HighlightSpan<P>>,
        cx: &mut Context<Self>,
    ) -> Result<(), HighlightSpanError> {
        let text = self.input.read(cx).value().to_string();
        let spans = match validate_spans(&text, spans) {
            Ok(spans) => spans,
            Err(error) => {
                self.clear_spans_and_hover(cx);
                return Err(error);
            }
        };

        let hover_is_stale = self.hovered.as_ref().is_some_and(|hovered| {
            let old = self.spans.get(hovered.index);
            let new = spans.get(hovered.index);
            match (old, new) {
                (Some(old), Some(new)) => {
                    old.range != new.range
                        || old.payload != new.payload
                        || new.range != hovered.range
                }
                _ => true,
            }
        });
        self.spans = spans;
        if hover_is_stale {
            self.clear_hover(cx);
        }
        self.rebuild_decorations(cx);
        Ok(())
    }

    fn clear_spans_and_hover(&mut self, cx: &mut Context<Self>) {
        self.spans.clear();
        self.input.update(cx, |input, cx| {
            input.set_text_decorations(Vec::new(), cx);
        });
        self.clear_hover(cx);
    }

    fn clear_hover(&mut self, cx: &mut Context<Self>) {
        if self.hovered.take().is_some() {
            cx.emit(HighlightInputEvent::HoverChanged(None));
        }
    }

    fn sync_style(&mut self, style: HighlightStyle, cx: &mut Context<Self>) {
        if self.style == style {
            return;
        }
        self.style = style;
        self.rebuild_decorations(cx);
    }

    fn rebuild_decorations(&mut self, cx: &mut Context<Self>) {
        let decorations = self
            .spans
            .iter()
            .map(|span| TextDecoration::new(span.range.clone(), self.style))
            .collect();
        self.input.update(cx, |input, cx| {
            input.set_text_decorations(decorations, cx);
        });
        #[cfg(test)]
        {
            self.decoration_epoch += 1;
        }
    }
}

impl<P: Clone + PartialEq + 'static> EventEmitter<HighlightInputEvent<P>>
    for HighlightInputState<P>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, Bounds};
    use gpui_component::input::InputState;

    #[test]
    fn spans_are_sorted_and_adjacency_is_allowed() {
        let spans = validate_spans(
            "aé{{host}}{{port}}",
            vec![
                HighlightSpan {
                    range: 11..19,
                    payload: "port",
                },
                HighlightSpan {
                    range: 3..11,
                    payload: "host",
                },
            ],
        )
        .unwrap();

        assert_eq!(
            spans
                .iter()
                .map(|span| span.range.clone())
                .collect::<Vec<_>>(),
            [3..11, 11..19]
        );
    }

    #[test]
    fn invalid_ranges_report_the_exact_reason() {
        assert!(matches!(
            validate_spans(
                "abc",
                vec![HighlightSpan {
                    range: 1..1,
                    payload: ()
                }]
            ),
            Err(HighlightSpanError::EmptyRange { range }) if range == (1..1)
        ));
        assert!(matches!(
            validate_spans(
                "abc",
                vec![HighlightSpan {
                    range: 0..4,
                    payload: ()
                }]
            ),
            Err(HighlightSpanError::OutOfBounds { range, text_len: 3 }) if range == (0..4)
        ));
        assert!(matches!(
            validate_spans(
                "é",
                vec![HighlightSpan {
                    range: 1..2,
                    payload: ()
                }]
            ),
            Err(HighlightSpanError::InvalidCharBoundary { offset: 1 })
        ));
        assert!(matches!(
            validate_spans(
                "abcd",
                vec![
                    HighlightSpan {
                        range: 0..3,
                        payload: ()
                    },
                    HighlightSpan {
                        range: 2..4,
                        payload: ()
                    },
                ]
            ),
            Err(HighlightSpanError::Overlap { previous, current })
                if previous == (0..3) && current == (2..4)
        ));
    }

    #[gpui::test]
    fn invalid_replacement_clears_decorations_and_hover_once(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();
        let input = cx.update(|window, cx| {
            cx.new(|cx| InputState::new(window, cx).default_value("aé{{host}}{{port}}"))
        });
        let state = cx.update(|_, cx| cx.new(|cx| HighlightInputState::new(input, cx)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured_events = events.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&state, move |_, event, _| {
                captured_events.borrow_mut().push(event.clone());
            })
        });

        let result = cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_spans(
                        vec![HighlightSpan {
                            range: 3..11,
                            payload: "host",
                        }],
                        cx,
                    )
                    .unwrap();
                state.hovered = Some(HoverTarget {
                    index: 0,
                    range: 3..11,
                    bounds: Bounds::default(),
                });
                state.set_spans(
                    vec![HighlightSpan {
                        range: 3..20,
                        payload: "host",
                    }],
                    cx,
                )
            })
        });

        let spans_are_empty = cx.update(|_, cx| state.read(cx).spans.is_empty());
        assert!(spans_are_empty);
        assert_eq!(
            events.borrow().as_slice(),
            [HighlightInputEvent::HoverChanged(None)]
        );
        assert!(matches!(
            result,
            Err(HighlightSpanError::OutOfBounds { .. })
        ));
    }

    #[gpui::test]
    fn sync_style_rebuilds_only_when_the_style_changes(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let cx = cx.add_empty_window();
        let input = cx.update(|window, cx| {
            cx.new(|cx| InputState::new(window, cx).default_value("aé{{host}}{{port}}"))
        });
        let state = cx.update(|_, cx| cx.new(|cx| HighlightInputState::new(input, cx)));

        let before = cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_spans(
                        vec![HighlightSpan {
                            range: 3..11,
                            payload: "host",
                        }],
                        cx,
                    )
                    .unwrap();
                state.decoration_epoch
            })
        });
        let after = cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                let first = HighlightStyle {
                    background_color: Some(gpui::red()),
                    ..Default::default()
                };
                let changed = HighlightStyle {
                    background_color: Some(gpui::blue()),
                    ..Default::default()
                };
                state.sync_style(first, cx);
                state.sync_style(first, cx);
                state.sync_style(changed, cx);
                state.decoration_epoch
            })
        });

        assert_eq!(after - before, 2);
    }
}
