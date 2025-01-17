use std::io::Read;

use xml::{attribute::OwnedAttribute, EventReader};

use crate::{
    error::TiledError,
    properties::{Color, Properties},
    tile::Gid,
    util::{get_attrs, parse_tag},
};

#[derive(Debug, PartialEq, Clone)]
pub struct ObjectGroup {
    /// The object group's name.
    pub name: String,
    /// The opacity with which this layer is drawn.
    pub opacity: f32,
    /// Whether this layer is visible or not.
    pub visible: bool,
    /// The collection of objects in this object group.
    pub objects: Vec<Object>,
    /// The color property of this layer.
    pub color: Option<Color>,
    /**
     * Layer index is not preset for tile collision boxes
     */
    pub layer_index: Option<u32>,
    /// The custom properties of this layer.
    pub properties: Properties,
}

impl ObjectGroup {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        layer_index: Option<u32>,
    ) -> Result<ObjectGroup, TiledError> {
        let ((o, v, c, n), ()) = get_attrs!(
            attrs,
            optionals: [
                ("opacity", opacity, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                ("color", color, |v:String| v.parse().ok()),
                ("name", name, |v:String| v.into()),
            ],
            required: [],
            TiledError::MalformedAttributes("object groups must have a name".to_string())
        );
        let mut objects = Vec::new();
        let mut properties = Properties::default();
        parse_tag!(parser, "objectgroup", {
            "object" => |attrs| {
                objects.push(Object::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = Properties::parse_xml(parser)?;
                Ok(())
            },
        });
        Ok(ObjectGroup {
            name: n.unwrap_or(String::new()),
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            objects,
            color: c,
            layer_index,
            properties,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectShape {
    Rect {
        width: f32,
        height: f32,
    },
    Ellipse {
        width: f32,
        height: f32,
    },
    Polyline {
        points: Vec<(f32, f32)>,
    },
    Polygon {
        points: Vec<(f32, f32)>,
    },
    Point(f32, f32),
    Text {
        font_family: String,
        pixel_size: usize,
        wrap: bool,
        color: Color,
        bold: bool,
        italic: bool,
        underline: bool,
        strikeout: bool,
        kerning: bool,
        halign: HorizontalAlignment,
        valign: VerticalAlignment,
        contents: String,
    },
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(missing_docs)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(missing_docs)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    /// The object's ID. Unique for every object from within a map.
    pub id: u32,
    /// The object's tile GID. If the object is not a tile, this is set to [`Gid::EMPTY`].
    pub gid: Gid,
    /// The object's name.
    pub name: String,
    /// The object's type.
    pub obj_type: String,
    /// The object's width, in pixels.
    pub width: f32,
    /// The object's height, in pixels.
    pub height: f32,
    /// The object's X position, in pixels.
    pub x: f32,
    /// The object's Y position, in pixels.
    pub y: f32,
    /// The object's angle of rotation, in degrees.
    pub rotation: f32,
    /// Whether this object is visible or not.
    pub visible: bool,
    /// The object's shape.
    pub shape: ObjectShape,
    /// The custom properties associated to this object.
    pub properties: Properties,
}

impl Object {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Object, TiledError> {
        let ((id, gid, n, t, w, h, v, r), (x, y)) = get_attrs!(
            attrs,
            optionals: [
                ("id", id, |v:String| v.parse().ok()),
                ("gid", gid, |v:String| v.parse().ok().and_then(|i| Some(Gid(i)))),
                ("name", name, |v:String| v.parse().ok()),
                ("type", obj_type, |v:String| v.parse().ok()),
                ("width", width, |v:String| v.parse().ok()),
                ("height", height, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                ("rotation", rotation, |v:String| v.parse().ok()),
            ],
            required: [
                ("x", x, |v:String| v.parse().ok()),
                ("y", y, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("objects must have an x and a y number".to_string())
        );
        let v = v.unwrap_or(true);
        let w = w.unwrap_or(0f32);
        let h = h.unwrap_or(0f32);
        let r = r.unwrap_or(0f32);
        let id = id.unwrap_or(0u32);
        let gid = gid.unwrap_or(Gid::EMPTY);
        let n = n.unwrap_or(String::new());
        let t = t.unwrap_or(String::new());
        let mut shape = None;
        let mut properties = Properties::default();

        parse_tag!(parser, "object", {
            "ellipse" => |_| {
                shape = Some(ObjectShape::Ellipse {
                    width: w,
                    height: h,
                });
                Ok(())
            },
            "polyline" => |attrs| {
                shape = Some(Object::new_polyline(attrs)?);
                Ok(())
            },
            "polygon" => |attrs| {
                shape = Some(Object::new_polygon(attrs)?);
                Ok(())
            },
            "point" => |_| {
                shape = Some(Object::new_point(x, y)?);
                Ok(())
            },
            "text" => |attrs| {
                shape = Some(Object::new_text(attrs, parser)?);
                Ok(())
            },
            "properties" => |_| {
                properties = Properties::parse_xml(parser)?;
                Ok(())
            },
        });

        let shape = shape.unwrap_or(ObjectShape::Rect {
            width: w,
            height: h,
        });

        Ok(Object {
            id,
            gid,
            name: n.clone(),
            obj_type: t.clone(),
            width: w,
            height: h,
            x,
            y,
            rotation: r,
            visible: v,
            shape: shape,
            properties,
        })
    }

    fn new_polyline(attrs: Vec<OwnedAttribute>) -> Result<ObjectShape, TiledError> {
        let ((), s) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("points", points, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("A polyline must have points".to_string())
        );
        let points = Object::parse_points(s)?;
        Ok(ObjectShape::Polyline { points: points })
    }

    fn new_polygon(attrs: Vec<OwnedAttribute>) -> Result<ObjectShape, TiledError> {
        let ((), s) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("points", points, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("A polygon must have points".to_string())
        );
        let points = Object::parse_points(s)?;
        Ok(ObjectShape::Polygon { points: points })
    }

    fn new_point(x: f32, y: f32) -> Result<ObjectShape, TiledError> {
        Ok(ObjectShape::Point(x, y))
    }

    fn new_text<R: Read>(
        attrs: Vec<OwnedAttribute>,
        parser: &mut EventReader<R>,
    ) -> Result<ObjectShape, TiledError> {
        let (
            (
                font_family,
                pixel_size,
                wrap,
                color,
                bold,
                italic,
                underline,
                strikeout,
                kerning,
                halign,
                valign,
            ),
            (),
        ) = get_attrs!(
            attrs,
            optionals: [
                ("fontfamily", font_family, Some),
                ("pixelsize", pixel_size, |v:String| v.parse().ok()),
                ("wrap", wrap, |v:String| v.parse().ok()),
                ("color", color, |v:String| v.parse().ok()),
                ("bold", bold, |v:String| v.parse().ok()),
                ("italic", italic, |v:String| v.parse().ok()),
                ("underline", underline, |v:String| v.parse().ok()),
                ("strikeout", strikeout, |v:String| v.parse().ok()),
                ("kerning", kerning, |v:String| v.parse().ok()),
                ("halign", halign, Some),
                ("valign", valign, Some),
            ],
            required: [],
            TiledError::Other("could not parse text".to_string())
        );
        let font_family = font_family.unwrap_or_else(|| "sans-serif".to_string());
        let pixel_size = pixel_size.unwrap_or(16);
        let wrap = if wrap == Some(1) { true } else { false };
        let color = color.unwrap_or(Color {
            red: 0,
            green: 0,
            blue: 0,
        });
        let bold = if bold == Some(1) { true } else { false };
        let italic = if italic == Some(1) { true } else { false };
        let underline = if underline == Some(1) { true } else { false };
        let strikeout = if strikeout == Some(1) { true } else { false };
        let kerning = if kerning == Some(0) { false } else { true };
        let halign = match halign.as_deref() {
            Some("left") | None => HorizontalAlignment::Left,
            Some("center") => HorizontalAlignment::Center,
            Some("right") => HorizontalAlignment::Right,
            Some("justify") => HorizontalAlignment::Justify,
            _ => panic!("Unknown halign"),
        };
        let valign = match valign.as_deref() {
            Some("top") | None => VerticalAlignment::Top,
            Some("center") => VerticalAlignment::Center,
            Some("bottom") => VerticalAlignment::Bottom,
            _ => panic!("Unknown halign"),
        };

        let contents = match parser.next().map_err(TiledError::XmlDecodingError)? {
            xml::reader::XmlEvent::Characters(contents) => contents,
            _ => panic!(),
        };

        Ok(ObjectShape::Text {
            font_family,
            pixel_size,
            wrap,
            color,
            bold,
            italic,
            underline,
            strikeout,
            kerning,
            halign,
            valign,
            contents,
        })
    }

    fn parse_points(s: String) -> Result<Vec<(f32, f32)>, TiledError> {
        let pairs = s.split(' ');
        let mut points = Vec::new();
        for v in pairs.map(|p| p.split(',')) {
            let v: Vec<&str> = v.collect();
            if v.len() != 2 {
                return Err(TiledError::MalformedAttributes(
                    "one of a polyline's points does not have an x and y coordinate".to_string(),
                ));
            }
            let (x, y) = (v[0].parse().ok(), v[1].parse().ok());
            if x.is_none() || y.is_none() {
                return Err(TiledError::MalformedAttributes(
                    "one of polyline's points does not have i32eger coordinates".to_string(),
                ));
            }
            points.push((x.unwrap(), y.unwrap()));
        }
        Ok(points)
    }
}
