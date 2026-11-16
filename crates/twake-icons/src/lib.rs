//! The twake-icons SVG set as static data and lookup; GPUI-free and dependency-free.
//! The icons come from `docs/spec/icons.json` through `cargo xtask icons` (`generated.rs`).

// The generator writes its own layout (no trailing commas): rustfmt leaves the file alone.
#[rustfmt::skip]
mod generated;

pub use generated::*;

/// The box an icon is drawn in, as the four numbers of an SVG `viewBox`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    /// Left edge.
    pub min_x: f32,
    /// Top edge.
    pub min_y: f32,
    /// Width, above zero.
    pub width: f32,
    /// Height, above zero.
    pub height: f32,
}

/// One icon: its kebab-case id, its box and the SVG elements that draw it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Icon {
    /// Kebab-case id, as in `docs/spec/icons.json`.
    pub id: &'static str,
    /// The box the body is drawn in.
    pub view_box: ViewBox,
    /// SVG elements, without the `<svg>` wrapper.
    pub body: &'static str,
}

impl Icon {
    /// A complete SVG document, tinted by the text colour (`currentColor`).
    pub fn to_svg(&self) -> String {
        let ViewBox {
            min_x,
            min_y,
            width,
            height,
        } = self.view_box;
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x} {min_y} {width} {height}\" fill=\"currentColor\">{}</svg>",
            self.body
        )
    }
}

/// The icon with this id, if the set holds it (exact, case-sensitive).
pub fn by_id(id: &str) -> Option<&'static Icon> {
    ALL.binary_search_by(|icon| icon.id.cmp(id))
        .ok()
        .map(|at| &ALL[at])
}
