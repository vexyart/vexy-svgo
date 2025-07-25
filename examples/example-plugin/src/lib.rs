use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::error::VexyError;
use vexy_svgo_core::plugin_registry::Plugin;

pub struct RemoveGElementPlugin;

impl Plugin for RemoveGElementPlugin {
    fn name(&self) -> &'static str {
        "removeGElement"
    }

    fn description(&self) -> &'static str {
        "Removes all <g> elements from an SVG."
    }

    fn apply(&self, document: &mut Document) -> Result<(), VexyError> {
        document.root.children.retain(|node| match node {
            Node::Element(element) => element.name != "g",
            _ => true,
        });
        Ok(())
    }
}
