// this_file: crates/core/src/collections.rs

//! Collections of SVG-related constants and mappings

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

pub static COLORS_NAMES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("aliceblue", "#f0f8ff"),
        ("antiquewhite", "#faebd7"),
        ("aqua", "#00ffff"),
        ("aquamarine", "#7fffd4"),
        ("azure", "#f0ffff"),
        ("beige", "#f5f5dc"),
        ("bisque", "#ffe4c4"),
        ("black", "#000000"),
        ("blanchedalmond", "#ffebcd"),
        ("blue", "#0000ff"),
        ("blueviolet", "#8a2be2"),
        ("brown", "#a52a2a"),
        ("burlywood", "#deb887"),
        ("cadetblue", "#5f9ea0"),
        ("chartreuse", "#7fff00"),
        ("chocolate", "#d2691e"),
        ("coral", "#ff7f50"),
        ("cornflowerblue", "#6495ed"),
        ("cornsilk", "#fff8dc"),
        ("crimson", "#dc143c"),
        ("cyan", "#00ffff"),
        ("darkblue", "#00008b"),
        ("darkcyan", "#008b8b"),
        ("darkgoldenrod", "#b8860b"),
        ("darkgray", "#a9a9a9"),
        ("darkgreen", "#006400"),
        ("darkgrey", "#a9a9a9"),
        ("darkkhaki", "#bdb76b"),
        ("darkmagenta", "#8b008b"),
        ("darkolivegreen", "#556b2f"),
        ("darkorange", "#ff8c00"),
        ("darkorchid", "#9932cc"),
        ("darkred", "#8b0000"),
        ("darksalmon", "#e9967a"),
        ("darkseagreen", "#8fbc8f"),
        ("darkslateblue", "#483d8b"),
        ("darkslategray", "#2f4f4f"),
        ("darkslategrey", "#2f4f4f"),
        ("darkturquoise", "#00ced1"),
        ("darkviolet", "#9400d3"),
        ("deeppink", "#ff1493"),
        ("deepskyblue", "#00bfff"),
        ("dimgray", "#696969"),
        ("dimgrey", "#696969"),
        ("dodgerblue", "#1e90ff"),
        ("firebrick", "#b22222"),
        ("floralwhite", "#fffaf0"),
        ("forestgreen", "#228b22"),
        ("fuchsia", "#ff00ff"),
        ("gainsboro", "#dcdcdc"),
        ("ghostwhite", "#f8f8ff"),
        ("gold", "#ffd700"),
        ("goldenrod", "#daa520"),
        ("gray", "#808080"),
        ("green", "#008000"),
        ("greenyellow", "#adff2f"),
        ("grey", "#808080"),
        ("honeydew", "#f0fff0"),
        ("hotpink", "#ff69b4"),
        ("indianred", "#cd5c5c"),
        ("indigo", "#4b0082"),
        ("ivory", "#fffff0"),
        ("khaki", "#f0e68c"),
        ("lavender", "#e6e6fa"),
        ("lavenderblush", "#fff0f5"),
        ("lawngreen", "#7cfc00"),
        ("lemonchiffon", "#fffacd"),
        ("lightblue", "#add8e6"),
        ("lightcoral", "#f08080"),
        ("lightcyan", "#e0ffff"),
        ("lightgoldenrodyellow", "#fafad2"),
        ("lightgray", "#d3d3d3"),
        ("lightgreen", "#90ee90"),
        ("lightgrey", "#d3d3d3"),
        ("lightpink", "#ffb6c1"),
        ("lightsalmon", "#ffa07a"),
        ("lightseagreen", "#20b2aa"),
        ("lightskyblue", "#87cefa"),
        ("lightslategray", "#778899"),
        ("lightslategrey", "#778899"),
        ("lightsteelblue", "#b0c4de"),
        ("lightyellow", "#ffffe0"),
        ("lime", "#00ff00"),
        ("limegreen", "#32cd32"),
        ("linen", "#faf0e6"),
        ("magenta", "#ff00ff"),
        ("maroon", "#800000"),
        ("mediumaquamarine", "#66cdaa"),
        ("mediumblue", "#0000cd"),
        ("mediumorchid", "#ba55d3"),
        ("mediumpurple", "#9370db"),
        ("mediumseagreen", "#3cb371"),
        ("mediumslateblue", "#7b68ee"),
        ("mediumspringgreen", "#00fa9a"),
        ("mediumturquoise", "#48d1cc"),
        ("mediumvioletred", "#c71585"),
        ("midnightblue", "#191970"),
        ("mintcream", "#f5fffa"),
        ("mistyrose", "#ffe4e1"),
        ("moccasin", "#ffe4b5"),
        ("navajowhite", "#ffdead"),
        ("navy", "#000080"),
        ("oldlace", "#fdf5e6"),
        ("olive", "#808000"),
        ("olivedrab", "#6b8e23"),
        ("orange", "#ffa500"),
        ("orangered", "#ff4500"),
        ("orchid", "#da70d6"),
        ("palegoldenrod", "#eee8aa"),
        ("palegreen", "#98fb98"),
        ("paleturquoise", "#afeeee"),
        ("palevioletred", "#db7093"),
        ("papayawhip", "#ffefd5"),
        ("peachpuff", "#ffdab9"),
        ("peru", "#cd853f"),
        ("pink", "#ffc0cb"),
        ("plum", "#dda0dd"),
        ("powderblue", "#b0e0e6"),
        ("purple", "#800080"),
        ("rebeccapurple", "#663399"),
        ("red", "#ff0000"),
        ("rosybrown", "#bc8f8f"),
        ("royalblue", "#4169e1"),
        ("saddlebrown", "#8b4513"),
        ("salmon", "#fa8072"),
        ("sandybrown", "#f4a460"),
        ("seagreen", "#2e8b57"),
        ("seashell", "#fff5ee"),
        ("sienna", "#a0522d"),
        ("silver", "#c0c0c0"),
        ("skyblue", "#87ceeb"),
        ("slateblue", "#6a5acd"),
        ("slategray", "#708090"),
        ("slategrey", "#708090"),
        ("snow", "#fffafa"),
        ("springgreen", "#00ff7f"),
        ("steelblue", "#4682b4"),
        ("tan", "#d2b48c"),
        ("teal", "#008080"),
        ("thistle", "#d8bfd8"),
        ("tomato", "#ff6347"),
        ("turquoise", "#40e0d0"),
        ("violet", "#ee82ee"),
        ("wheat", "#f5deb3"),
        ("white", "#ffffff"),
        ("whitesmoke", "#f5f5f5"),
        ("yellow", "#ffff00"),
        ("yellowgreen", "#9acd32"),
    ])
});

pub static COLORS_SHORT_NAMES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("#f0f8ff", "aliceblue"),
        ("#faebd7", "antiquewhite"),
        ("#00ffff", "aqua"),
        ("#7fffd4", "aquamarine"),
        ("#f0ffff", "azure"),
        ("#f5f5dc", "beige"),
        ("#ffe4c4", "bisque"),
        ("#000000", "black"),
        ("#ffebcd", "blanchedalmond"),
        ("#0000ff", "blue"),
        ("#8a2be2", "blueviolet"),
        ("#a52a2a", "brown"),
        ("#deb887", "burlywood"),
        ("#5f9ea0", "cadetblue"),
        ("#7fff00", "chartreuse"),
        ("#d2691e", "chocolate"),
        ("#ff7f50", "coral"),
        ("#6495ed", "cornflowerblue"),
        ("#fff8dc", "cornsilk"),
        ("#dc143c", "crimson"),
        ("#00ffff", "cyan"),
        ("#00008b", "darkblue"),
        ("#008b8b", "darkcyan"),
        ("#b8860b", "darkgoldenrod"),
        ("#a9a9a9", "darkgray"),
        ("#006400", "darkgreen"),
        ("#a9a9a9", "darkgrey"),
        ("#bdb76b", "darkkhaki"),
        ("#8b008b", "darkmagenta"),
        ("#556b2f", "darkolivegreen"),
        ("#ff8c00", "darkorange"),
        ("#9932cc", "darkorchid"),
        ("#8b0000", "darkred"),
        ("#e9967a", "darksalmon"),
        ("#8fbc8f", "darkseagreen"),
        ("#483d8b", "darkslateblue"),
        ("#2f4f4f", "darkslategray"),
        ("#2f4f4f", "darkslategrey"),
        ("#00ced1", "darkturquoise"),
        ("#9400d3", "darkviolet"),
        ("#ff1493", "deeppink"),
        ("#00bfff", "deepskyblue"),
        ("#696969", "dimgray"),
        ("#696969", "dimgrey"),
        ("#1e90ff", "dodgerblue"),
        ("#b22222", "firebrick"),
        ("#fffaf0", "floralwhite"),
        ("#228b22", "forestgreen"),
        ("#ff00ff", "fuchsia"),
        ("#dcdcdc", "gainsboro"),
        ("#f8f8ff", "ghostwhite"),
        ("#ffd700", "gold"),
        ("#daa520", "goldenrod"),
        ("#808080", "gray"),
        ("#008000", "green"),
        ("#adff2f", "greenyellow"),
        ("#808080", "grey"),
        ("#f0fff0", "honeydew"),
        ("#ff69b4", "hotpink"),
        ("#cd5c5c", "indianred"),
        ("#4b0082", "indigo"),
        ("#fffff0", "ivory"),
        ("#f0e68c", "khaki"),
        ("#e6e6fa", "lavender"),
        ("#fff0f5", "lavenderblush"),
        ("#7cfc00", "lawngreen"),
        ("#fffacd", "lemonchiffon"),
        ("#add8e6", "lightblue"),
        ("#f08080", "lightcoral"),
        ("#e0ffff", "lightcyan"),
        ("#fafad2", "lightgoldenrodyellow"),
        ("#d3d3d3", "lightgray"),
        ("#90ee90", "lightgreen"),
        ("#d3d3d3", "lightgrey"),
        ("#ffb6c1", "lightpink"),
        ("#ffa07a", "lightsalmon"),
        ("#20b2aa", "lightseagreen"),
        ("#87cefa", "lightskyblue"),
        ("#778899", "lightslategray"),
        ("#778899", "lightslategrey"),
        ("#b0c4de", "lightsteelblue"),
        ("#ffffe0", "lightyellow"),
        ("#00ff00", "lime"),
        ("#32cd32", "limegreen"),
        ("#faf0e6", "linen"),
        ("#ff00ff", "magenta"),
        ("#800000", "maroon"),
        ("#66cdaa", "mediumaquamarine"),
        ("#0000cd", "mediumblue"),
        ("#ba55d3", "mediumorchid"),
        ("#9370db", "mediumpurple"),
        ("#3cb371", "mediumseagreen"),
        ("#7b68ee", "mediumslateblue"),
        ("#00fa9a", "mediumspringgreen"),
        ("#48d1cc", "mediumturquoise"),
        ("#c71585", "mediumvioletred"),
        ("#191970", "midnightblue"),
        ("f5fffa", "mintcream"),
        ("#ffe4e1", "mistyrose"),
        ("#ffe4b5", "moccasin"),
        ("#ffdead", "navajowhite"),
        ("#000080", "navy"),
        ("#fdf5e6", "oldlace"),
        ("#808000", "olive"),
        ("#6b8e23", "olivedrab"),
        ("#ffa500", "orange"),
        ("#ff4500", "orangered"),
        ("#da70d6", "orchid"),
        ("#eee8aa", "palegoldenrod"),
        ("#98fb98", "palegreen"),
        ("#afeeee", "paleturquoise"),
        ("#db7093", "palevioletred"),
        ("#ffefd5", "papayawhip"),
        ("#ffdab9", "peachpuff"),
        ("#cd853f", "peru"),
        ("#ffc0cb", "pink"),
        ("#dda0dd", "plum"),
        ("#b0e0e6", "powderblue"),
        ("#800080", "purple"),
        ("#663399", "rebeccapurple"),
        ("#ff0000", "red"),
        ("#bc8f8f", "rosybrown"),
        ("#4169e1", "royalblue"),
        ("#8b4513", "saddlebrown"),
        ("#fa8072", "salmon"),
        ("#f4a460", "sandybrown"),
        ("#2e8b57", "seagreen"),
        ("#fff5ee", "seashell"),
        ("#a0522d", "sienna"),
        ("#c0c0c0", "silver"),
        ("#87ceeb", "skyblue"),
        ("#6a5acd", "slateblue"),
        ("#708090", "slategray"),
        ("#708090", "slategrey"),
        ("#fffafa", "snow"),
        ("#00ff7f", "springgreen"),
        ("#4682b4", "steelblue"),
        ("#d2b48c", "tan"),
        ("#008080", "teal"),
        ("#d8bfd8", "thistle"),
        ("#ff6347", "tomato"),
        ("#40e0d0", "turquoise"),
        ("#ee82ee", "violet"),
        ("#f5deb3", "wheat"),
        ("#ffffff", "white"),
        ("#f5f5f5", "whitesmoke"),
        ("#ffff00", "yellow"),
        ("#9acd32", "yellowgreen"),
    ])
});

pub static COLORS_PROPS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "color",
        "fill",
        "stroke",
        "stop-color",
        "flood-color",
        "lighting-color",
        "outline-color",
        "text-decoration-color",
        "column-rule-color",
        "background-color",
    ])
});

/// Animation elements in SVG
pub static ANIMATION_ELEMS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "animate",
        "animateColor",
        "animateMotion",
        "animateTransform",
        "set",
    ])
});

/// Inheritable attributes in SVG
pub static INHERITABLE_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "clip-rule",
        "color",
        "color-interpolation",
        "color-interpolation-filters",
        "color-profile",
        "color-rendering",
        "cursor",
        "direction",
        "fill",
        "fill-opacity",
        "fill-rule",
        "font-family",
        "font-size",
        "font-size-adjust",
        "font-stretch",
        "font-style",
        "font-variant",
        "font-weight",
        "glyph-orientation-horizontal",
        "glyph-orientation-vertical",
        "image-rendering",
        "kerning",
        "letter-spacing",
        "marker",
        "marker-end",
        "marker-mid",
        "marker-start",
        "opacity",
        "pointer-events",
        "shape-rendering",
        "stroke",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-opacity",
        "stroke-width",
        "text-anchor",
        "text-decoration",
        "text-rendering",
        "unicode-bidi",
        "visibility",
        "word-spacing",
        "writing-mode",
    ])
});

/// SVG presentation attributes that can be set as attributes
pub static PRESENTATION_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "alignment-baseline",
        "baseline-shift",
        "clip-path",
        "clip-rule",
        "clip",
        "color-interpolation-filters",
        "color-interpolation",
        "color-profile",
        "color-rendering",
        "color",
        "cursor",
        "direction",
        "display",
        "dominant-baseline",
        "enable-background",
        "fill-opacity",
        "fill-rule",
        "fill",
        "filter",
        "flood-color",
        "flood-opacity",
        "font-family",
        "font-size-adjust",
        "font-size",
        "font-stretch",
        "font-style",
        "font-variant",
        "font-weight",
        "glyph-orientation-horizontal",
        "glyph-orientation-vertical",
        "image-rendering",
        "letter-spacing",
        "lighting-color",
        "marker-end",
        "marker-mid",
        "marker-start",
        "mask",
        "opacity",
        "overflow",
        "paint-order",
        "pointer-events",
        "shape-rendering",
        "stop-color",
        "stop-opacity",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-opacity",
        "stroke-width",
        "stroke",
        "text-anchor",
        "text-decoration",
        "text-overflow",
        "text-rendering",
        "transform-origin",
        "transform",
        "unicode-bidi",
        "vector-effect",
        "visibility",
        "word-spacing",
        "writing-mode",
    ])
});

/// Animation event attributes in SVG
pub static ANIMATION_EVENT_ATTRS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["onbegin", "onend", "onrepeat", "onload"]));

/// Document event attributes in SVG
pub static DOCUMENT_EVENT_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "onabort", "onerror", "onresize", "onscroll", "onunload", "onzoom",
    ])
});

/// Document element event attributes in SVG
pub static DOCUMENT_ELEMENT_EVENT_ATTRS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["oncopy", "oncut", "onpaste"]));

/// Global event attributes in SVG
pub static GLOBAL_EVENT_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "oncancel",
        "oncanplay",
        "oncanplaythrough",
        "onchange",
        "onclick",
        "onclose",
        "oncuechange",
        "ondblclick",
        "ondrag",
        "ondragend",
        "ondragenter",
        "ondragleave",
        "ondragover",
        "ondragstart",
        "ondrop",
        "ondurationchange",
        "onemptied",
        "onended",
        "onerror",
        "onfocus",
        "oninput",
        "oninvalid",
        "onkeydown",
        "onkeypress",
        "onkeyup",
        "onload",
        "onloadeddata",
        "onloadedmetadata",
        "onloadstart",
        "onmousedown",
        "onmouseenter",
        "onmouseleave",
        "onmousemove",
        "onmouseout",
        "onmouseover",
        "onmouseup",
        "onmousewheel",
        "onpause",
        "onplay",
        "onplaying",
        "onprogress",
        "onratechange",
        "onreset",
        "onresize",
        "onscroll",
        "onseeked",
        "onseeking",
        "onselect",
        "onshow",
        "onstalled",
        "onsubmit",
        "onsuspend",
        "ontimeupdate",
        "ontoggle",
        "onvolumechange",
        "onwaiting",
    ])
});

/// Graphical event attributes in SVG
pub static GRAPHICAL_EVENT_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "onfocusin",
        "onfocusout",
        "onactivate",
        "onclick",
        "onmousedown",
        "onmouseup",
        "onmouseover",
        "onmousemove",
        "onmouseout",
        "onload",
    ])
});

/// Properties that can contain URL references in SVG
pub static REFERENCES_PROPS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "clip-path",
        "color-profile",
        "fill",
        "filter",
        "marker-end",
        "marker-mid",
        "marker-start",
        "mask",
        "stroke",
        "style",
    ])
});

/// Editor-specific namespaces in SVG
pub static EDITOR_NAMESPACES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "http://creativecommons.org/ns#",
        "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd",
        "http://krita.org/namespaces/svg/krita",
        "http://ns.adobe.com/AdobeIllustrator/10.0/",
        "http://ns.adobe.com/AdobeSVGViewerExtensions/3.0/",
        "http://ns.adobe.com/Extensibility/1.0/",
        "http://ns.adobe.com/Flows/1.0/",
        "http://ns.adobe.com/GenericCustomNamespace/1.0/",
        "http://ns.adobe.com/Graphs/1.0/",
        "http://ns.adobe.com/ImageReplacement/1.0/",
        "http://ns.adobe.com/SaveForWeb/1.0/",
        "http://ns.adobe.com/Variables/1.0/",
        "http://ns.adobe.com/XPath/1.0/",
        "http://purl.org/dc/elements/1.1/",
        "http://schemas.microsoft.com/visio/2003/SVGExtensions/",
        "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
        "http://taptrix.com/vectorillustrator/svg_extensions",
        "http://www.bohemiancoding.com/sketch/ns",
        "http://www.figma.com/figma/ns",
        "http://www.inkscape.org/namespaces/inkscape",
        "http://www.serif.com/",
        "http://www.vector.evaxdesign.sk",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
        "https://boxy-svg.com",
    ])
});
