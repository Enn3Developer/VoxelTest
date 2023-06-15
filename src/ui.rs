use std::rc::Rc;

use wgpu::{util::StagingBelt, CommandEncoder, Device, TextureFormat, TextureView};
use wgpu_glyph::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder, Section, Text};

pub trait Component {
    fn render(&self, glyph_brush: &mut GlyphBrush<()>);
}

pub struct Label {
    position: (f32, f32),
    bounds: (f32, f32),
    text: String,
    color: [f32; 4],
}

impl Label {
    #[inline]
    pub fn new(position: (f32, f32), bounds: (f32, f32), text: String, color: [f32; 4]) -> Self {
        Self {
            position,
            bounds,
            text,
            color,
        }
    }

    #[inline]
    pub fn with_position(mut self, position: (f32, f32)) -> Self {
        self.position = position;
        self
    }

    #[inline]
    pub fn with_bounds(mut self, bounds: (f32, f32)) -> Self {
        self.bounds = bounds;
        self
    }

    #[inline]
    pub fn with_text<S: Into<String>>(mut self, str: S) -> Self {
        self.text = str.into();
        self
    }

    #[inline]
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn set_text<S: Into<String>>(&mut self, str: S) {
        self.text = str.into();
    }
}

impl Default for Label {
    #[inline]
    fn default() -> Self {
        Self::new(
            (0.0, 0.0),
            (1920.0, 1080.0),
            String::new(),
            [1.0, 1.0, 1.0, 1.0],
        )
    }
}

impl Component for Label {
    #[inline]
    fn render(&self, glyph_brush: &mut GlyphBrush<()>) {
        glyph_brush.queue(Section {
            screen_position: self.position,
            bounds: self.bounds,
            text: vec![Text::new(&self.text).with_color(self.color)],
            ..Default::default()
        });
    }
}

pub struct UI {
    glyph_brush: GlyphBrush<()>,
    staging_belt: StagingBelt,
    device: Rc<Device>,
}

impl UI {
    pub fn new(font_data: &'static [u8], device: Rc<Device>, format: TextureFormat) -> Self {
        let font = FontArc::try_from_slice(font_data).expect("Can't load font");

        let glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, format);
        let staging_belt = StagingBelt::new(1024);

        Self {
            glyph_brush,
            staging_belt,
            device,
        }
    }

    #[inline]
    pub fn render<C: Component>(&mut self, component: &C) {
        component.render(&mut self.glyph_brush);
    }

    #[inline]
    pub fn draw(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.glyph_brush.draw_queued(
            &self.device,
            &mut self.staging_belt,
            encoder,
            view,
            width,
            height,
        )?;
        self.staging_belt.finish();

        Ok(())
    }

    #[inline]
    pub fn recall(&mut self) {
        self.staging_belt.recall();
    }
}
