use gpui::{Context, Empty, IntoElement, Render, Window};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Track {
    FrontLevel,
    FrontCct,
    RearLevel,
    RearSpeed,
    Hue,
    SatVal,
}

#[derive(Clone, Copy)]
pub struct TrackDrag(pub Track);

impl Render for TrackDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
