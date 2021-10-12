use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use xml::{attribute::OwnedAttribute, reader::XmlEvent, EventReader};

use crate::{
    error::{ParseTileError, TiledError},
    layers::{ImageLayer, Layer},
    objects::ObjectGroup,
    properties::{parse_properties, Color, Properties},
    tile::Gid,
    tileset::Tileset,
    util::{get_attrs, parse_tag},
};

/// All Tiled files will be parsed into this. Holds all the layers and tilesets
#[derive(Debug, PartialEq, Clone)]
pub struct Map {
    /// The TMX format version this map was saved to.
    pub version: String,
    /// The orientation of this map.
    pub orientation: Orientation,
    /// Width of the map, in tiles.
    pub width: u32,
    /// Height of the map, in tiles.
    pub height: u32,
    /// Tile width, in pixels.
    pub tile_width: u32,
    /// Tile height, in pixels.
    pub tile_height: u32,
    /// The tilesets present in this map.
    pub tilesets: Vec<Tileset>,
    /// The tile layers present in this map.
    pub layers: Vec<Layer>,
    /// The image layers present in this map.
    pub image_layers: Vec<ImageLayer>,
    /// The object groups present in this map.
    pub object_groups: Vec<ObjectGroup>,
    /// The custom properties of this map.
    pub properties: Properties,
    /// The background color of this map, if any.
    pub background_color: Option<Color>,
    /// Whether this map is infinite or not.
    pub infinite: bool,
    /// Where this map was loaded from.
    /// If fully embedded (loaded with path = `None`), this will return `None`.
    pub source: Option<PathBuf>,
}

impl Map {
    /// Parse a buffer hopefully containing the contents of a Tiled file and try to
    /// parse it. This augments `parse` with a file location: some engines
    /// (e.g. Amethyst) simply hand over a byte stream (and file location) for parsing,
    /// in which case this function may be required.
    /// The path may be skipped if the map is fully embedded (Doesn't refer to external files).
    pub fn parse_reader<R: Read>(reader: R, path: Option<&Path>) -> Result<Self, TiledError> {
        let mut parser = EventReader::new(reader);
        loop {
            match parser.next().map_err(TiledError::XmlDecodingError)? {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    if name.local_name == "map" {
                        return Self::parse_xml(&mut parser, attributes, path);
                    }
                }
                XmlEvent::EndDocument => {
                    return Err(TiledError::PrematureEnd(
                        "Document ended before map was parsed".to_string(),
                    ))
                }
                _ => {}
            }
        }
    }

    /// Parse a file hopefully containing a Tiled map and try to parse it.  If the
    /// file has an external tileset, the tileset file will be loaded using a path
    /// relative to the map file's path.
    pub fn parse_file(path: &Path) -> Result<Self, TiledError> {
        let file = File::open(path)
            .map_err(|_| TiledError::Other(format!("Map file not found: {:?}", path)))?;
        Self::parse_reader(file, Some(path))
    }

    fn parse_xml<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        map_path: Option<&Path>,
    ) -> Result<Map, TiledError> {
        let ((c, infinite), (v, o, w, h, tw, th)) = get_attrs!(
            attrs,
            optionals: [
                ("backgroundcolor", colour, |v:String| v.parse().ok()),
                ("infinite", infinite, |v:String| Some(v == "1")),
            ],
            required: [
                ("version", version, |v| Some(v)),
                ("orientation", orientation, |v:String| v.parse().ok()),
                ("width", width, |v:String| v.parse().ok()),
                ("height", height, |v:String| v.parse().ok()),
                ("tilewidth", tile_width, |v:String| v.parse().ok()),
                ("tileheight", tile_height, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("map must have a version, width and height with correct types".to_string())
        );

        let mut tilesets = Vec::new();
        let mut layers = Vec::new();
        let mut image_layers = Vec::new();
        let mut properties = HashMap::new();
        let mut object_groups = Vec::new();
        let mut layer_index = 0;
        parse_tag!(parser, "map", {
            "tileset" => |attrs| {
                tilesets.push(Tileset::parse_xml(parser, attrs, map_path)?);
                Ok(())
            },
            "layer" => |attrs| {
                layers.push(Layer::new(parser, attrs, w, layer_index, infinite.unwrap_or(false))?);
                layer_index += 1;
                Ok(())
            },
            "imagelayer" => |attrs| {
                image_layers.push(ImageLayer::new(parser, attrs, layer_index)?);
                layer_index += 1;
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
            "objectgroup" => |attrs| {
                object_groups.push(ObjectGroup::new(parser, attrs, Some(layer_index))?);
                layer_index += 1;
                Ok(())
            },
        });
        Ok(Map {
            version: v,
            orientation: o,
            width: w,
            height: h,
            tile_width: tw,
            tile_height: th,
            tilesets,
            layers,
            image_layers,
            object_groups,
            properties,
            background_color: c,
            infinite: infinite.unwrap_or(false),
            source: map_path.and_then(|p| Some(p.to_owned())),
        })
    }

    /// Returns the tileset that contains the tile with the given GID, if any.
    pub fn tileset_by_gid(&self, gid: Gid) -> Option<&Tileset> {
        self.tilesets.iter().find(|t| t.contains_tile(gid))
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Orientation {
    Orthogonal,
    Isometric,
    Staggered,
    Hexagonal,
}

impl FromStr for Orientation {
    type Err = ParseTileError;

    fn from_str(s: &str) -> Result<Orientation, ParseTileError> {
        match s {
            "orthogonal" => Ok(Orientation::Orthogonal),
            "isometric" => Ok(Orientation::Isometric),
            "staggered" => Ok(Orientation::Staggered),
            "hexagonal" => Ok(Orientation::Hexagonal),
            _ => Err(ParseTileError::OrientationError),
        }
    }
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Orientation::Orthogonal => write!(f, "orthogonal"),
            Orientation::Isometric => write!(f, "isometric"),
            Orientation::Staggered => write!(f, "staggered"),
            Orientation::Hexagonal => write!(f, "hexagonal"),
        }
    }
}
