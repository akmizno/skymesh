use anyhow::Result;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag_no_case, take_till, take_until, take_while},
    character,
    character::complete::{alphanumeric1, multispace0, not_line_ending, space0, space1, usize},
    combinator::{all_consuming, consumed, fail, map, not, opt, peek},
    number,
    sequence::{preceded, separated_pair, terminated},
};

use super::mesh::{Face, PlyMeshBuilder, Vertex};

#[derive(Debug, Clone)]
pub(super) struct PlyParser<'a> {
    format: Format<'a>,
    vertex_element: VertexElement<'a>,
    face_element: FaceElement<'a>,
}

impl<'a> PlyParser<'a> {
    pub(super) fn parse(
        input: &'a [u8],
        builder: PlyMeshBuilder,
    ) -> Result<(&'a [u8], PlyMeshBuilder)> {
        let (body_input, header_input) =
            take_until::<&str, &[u8], nom::error::Error<&[u8]>>("end_header")(input)
                .map_err(|e| e.to_owned())?;

        let header_str = str::from_utf8(header_input)?;
        let (_, parser) = Self::parse_header(header_str).map_err(|e| e.to_owned())?;

        let (body_input, builder) = match parser.format {
            Format::Ascii(_) => {
                let body_str = str::from_utf8(body_input)?;
                parser
                    .parse_body_ascii(body_str, builder)
                    .map(|(i, b)| (i.as_bytes(), b))
                    .map_err(|e| e.map_input(|i| i.as_bytes()))
            }
            Format::BinaryLe(_) => parser.parse_body_binary_le(body_input, builder),
            Format::BinaryBe(_) => parser.parse_body_binary_be(body_input, builder),
        }
        .map_err(|e| e.to_owned())?;

        Ok((body_input, builder))
    }

    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, _) = tag_no_case("ply")(input)?;
        let (input, _) = skip_ignored(input)?;
        let (input, format) = Format::parse_header(input)?;
        let (input, _) = skip_ignored(input)?;
        let (input, vertex_element) = VertexElement::parse_header(input)?;
        let (input, _) = skip_ignored(input)?;
        let (input, face_element) = FaceElement::parse_header(input)?;

        let parser = Self {
            format,
            vertex_element,
            face_element,
        };

        Ok((input, parser))
    }

    fn parse_body_ascii(
        &self,
        input: &'a str,
        builder: PlyMeshBuilder,
    ) -> IResult<&'a str, PlyMeshBuilder> {
        self.parse_body_generic(
            input,
            builder,
            |i| self.vertex_element.parse_body_ascii(i),
            |i| self.face_element.parse_body_ascii(i),
        )
    }

    fn parse_body_binary_le(
        &self,
        input: &'a [u8],
        builder: PlyMeshBuilder,
    ) -> IResult<&'a [u8], PlyMeshBuilder> {
        self.parse_body_generic(
            input,
            builder,
            |i| self.vertex_element.parse_body_binary_le(i),
            |i| self.face_element.parse_body_binary_le(i),
        )
    }

    fn parse_body_binary_be(
        &self,
        input: &'a [u8],
        builder: PlyMeshBuilder,
    ) -> IResult<&'a [u8], PlyMeshBuilder> {
        self.parse_body_generic(
            input,
            builder,
            |i| self.vertex_element.parse_body_binary_be(i),
            |i| self.face_element.parse_body_binary_be(i),
        )
    }

    fn parse_body_generic<I, FV, FF>(
        &self,
        mut input: I,
        mut builder: PlyMeshBuilder,
        parse_vert: FV,
        parse_face: FF,
    ) -> IResult<I, PlyMeshBuilder>
    where
        FV: Fn(I) -> IResult<I, Vertex>,
        FF: Fn(I) -> IResult<I, Face>,
    {
        builder.set_num_vertices(self.vertex_element.num_vertices);
        for _ in 0..self.vertex_element.num_vertices {
            let (i, vertex) = parse_vert(input)?;
            input = i;

            builder.add_vertex(vertex);
        }

        builder.set_num_faces(self.face_element.num_faces);
        for _ in 0..self.face_element.num_faces {
            let (i, face) = parse_face(input)?;
            input = i;

            builder.add_face(face);
        }

        Ok((input, builder))
    }
}

fn parse_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag_no_case("comment").parse(input)?;
    let (input, comment) = preceded(space0, not_line_ending).parse(input)?;
    Ok((input, comment))
}

fn skip_ignored(input: &str) -> IResult<&str, Option<&str>> {
    let (input, _) = multispace0(input)?;
    let (input, comment) = opt(parse_comment).parse(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, comment))
}

// Format type (and version)
#[derive(Debug, Clone, PartialEq)]
enum Format<'a> {
    Ascii(&'a str),
    BinaryLe(&'a str),
    BinaryBe(&'a str),
}

impl<'a> Format<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, (_, format_str)) = separated_pair(
            preceded(space0, tag_no_case("format")),
            space1,
            not_line_ending,
        )
        .parse(input)?;

        alt((
            map(
                separated_pair(tag_no_case("ascii"), space1, not_line_ending),
                |(_, version)| Self::Ascii(version),
            ),
            map(
                separated_pair(tag_no_case("binary_little_endian"), space1, not_line_ending),
                |(_, version)| Self::BinaryLe(version),
            ),
            map(
                separated_pair(tag_no_case("binary_big_endian"), space1, not_line_ending),
                |(_, version)| Self::BinaryBe(version),
            ),
        ))
        .parse(format_str)
        .map(|(_, format)| (input, format))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RawElement<'a> {
    name: &'a str,
    num_elements: usize,
    properties: Vec<Property<'a>>,
}

impl<'a> RawElement<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, (_, name_num_str)) = separated_pair(
            tag_no_case::<&'a str, &'a str, nom::error::Error<&'a str>>("element"),
            space1,
            not_line_ending,
        )
        .parse(input)?;

        let (name_num_str, name) =
            take_till(|c: char| c.is_ascii_whitespace()).parse(name_num_str)?;
        let (name_num_str, _) = space1(name_num_str)?;
        let (_, num_elements) = usize(name_num_str)?;

        let mut properties = Vec::with_capacity(num_elements);

        let mut input = input;
        loop {
            let (i, _) = skip_ignored(input)?;
            input = i;

            if let Ok((i, prop)) = Property::parse_header(input) {
                input = i;
                properties.push(prop);
            } else {
                break;
            }
        }

        Ok((
            input,
            Self {
                name,
                num_elements,
                properties,
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VertexElement<'a> {
    num_vertices: usize,
    properties: Vec<Property<'a>>,
}

impl<'a> VertexElement<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, raw_element) = RawElement::parse_header(input)?;

        // test "vertex" keyword
        let _ = tag_no_case("vertex").parse(raw_element.name)?;

        Ok((
            input,
            Self {
                num_vertices: raw_element.num_elements,
                properties: raw_element.properties,
            },
        ))
    }

    fn parse_body_ascii(&self, input: &'a str) -> IResult<&'a str, Vertex> {
        todo!()
    }

    fn parse_body_binary_le(&self, input: &'a [u8]) -> IResult<&'a [u8], Vertex> {
        todo!()
    }

    fn parse_body_binary_be(&self, input: &'a [u8]) -> IResult<&'a [u8], Vertex> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct FaceElement<'a> {
    num_faces: usize,
    properties: Vec<Property<'a>>,
}

impl<'a> FaceElement<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, raw_element) = RawElement::parse_header(input)?;

        // test "face" keyword
        let _ = tag_no_case("face").parse(raw_element.name)?;

        Ok((
            input,
            Self {
                num_faces: raw_element.num_elements,
                properties: raw_element.properties,
            },
        ))
    }

    fn parse_body_ascii(&self, input: &'a str) -> IResult<&'a str, Face> {
        todo!()
    }

    fn parse_body_binary_le(&self, input: &'a [u8]) -> IResult<&'a [u8], Face> {
        todo!()
    }

    fn parse_body_binary_be(&self, input: &'a [u8]) -> IResult<&'a [u8], Face> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Property<'a> {
    Vertex(VertexProperty),
    Face(FaceProperty),
    Unsupported(RawProperty<'a>),
}

impl<'a> Property<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        let (input, raw_property) = RawProperty::parse_header(input)?;

        Ok((
            input,
            if let Ok((_, vert_property)) = VertexProperty::from_raw_property(&raw_property) {
                Property::Vertex(vert_property)
            } else if let Ok((_, face_property)) = FaceProperty::from_raw_property(&raw_property) {
                Property::Face(face_property)
            } else {
                Property::Unsupported(raw_property)
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RawProperty<'a> {
    property_str: &'a str,
}

impl<'a> RawProperty<'a> {
    fn parse_header(input: &'a str) -> IResult<&'a str, Self> {
        map(
            separated_pair(
                tag_no_case::<&'a str, &'a str, nom::error::Error<&'a str>>("property"),
                space1,
                not_line_ending,
            ),
            |(_, property_str)| Self { property_str },
        )
        .parse(input)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VertexProperty {
    value_type: NumericType,
    name: VertexPropertyName,
}

impl VertexProperty {
    fn from_raw_property<'a>(raw_property: &RawProperty<'a>) -> IResult<&'a str, VertexProperty> {
        let property_str = raw_property.property_str;

        let (property_str, value_type) =
            preceded(space0, NumericType::parse_header).parse(property_str)?;
        let (property_str, name) =
            preceded(space1, VertexPropertyName::parse_header).parse(property_str)?;

        Ok((property_str, Self { value_type, name }))
    }

    fn parse_header(input: &str) -> IResult<&str, Self> {
        let (input, raw_property) = RawProperty::parse_header(input)?;

        let (_property_str, vert_property) = Self::from_raw_property(&raw_property)?;

        Ok((input, vert_property))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct FaceProperty {
    num_type: NumericType,
    idx_type: NumericType,
}

impl FaceProperty {
    fn from_raw_property<'a>(raw_property: &RawProperty<'a>) -> IResult<&'a str, FaceProperty> {
        let property_str = raw_property.property_str;

        let (property_str, _) = preceded(space0, tag_no_case("list")).parse(property_str)?;
        let (property_str, num_type) =
            preceded(space1, NumericType::parse_header).parse(property_str)?;
        let (property_str, idx_type) =
            preceded(space1, NumericType::parse_header).parse(property_str)?;
        let (property_str, _) =
            preceded(space1, tag_no_case("vertex_index")).parse(property_str)?;

        Ok((property_str, Self { num_type, idx_type }))
    }

    fn parse_header(input: &str) -> IResult<&str, Self> {
        let (input, raw_property) = RawProperty::parse_header(input)?;

        let (_property_str, face_property) = Self::from_raw_property(&raw_property)?;

        Ok((input, face_property))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NumericType {
    Int8,
    Int16,
    Int32,
    Uint8,
    Uint16,
    Uint32,
    F32,
    F64,
}

impl NumericType {
    fn parse_header(input: &str) -> IResult<&str, Self> {
        let (input, s) = preceded(space0, alphanumeric1).parse(input)?;
        alt((
            map(
                alt((
                    all_consuming(tag_no_case("char")),
                    all_consuming(tag_no_case("int8")),
                )),
                |_| Self::Int8,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("short")),
                    all_consuming(tag_no_case("int16")),
                )),
                |_| Self::Int16,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("int")),
                    all_consuming(tag_no_case("int32")),
                )),
                |_| Self::Int32,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("uchar")),
                    all_consuming(tag_no_case("uint8")),
                )),
                |_| Self::Uint8,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("ushort")),
                    all_consuming(tag_no_case("uint16")),
                )),
                |_| Self::Uint16,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("uint")),
                    all_consuming(tag_no_case("uint32")),
                )),
                |_| Self::Uint32,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("float")),
                    all_consuming(tag_no_case("float32")),
                )),
                |_| Self::F32,
            ),
            map(
                alt((
                    all_consuming(tag_no_case("double")),
                    all_consuming(tag_no_case("float64")),
                )),
                |_| Self::F64,
            ),
        ))
        .parse(s)
        .map(|(_, n)| (input, n))
    }

    fn parse_ascii_integer<'a, 'b>(&'a self, input: &'b str) -> IResult<&'b str, usize> {
        match self {
            Self::Int8 => map(nom::character::complete::i8, |n| n as usize).parse(input),
            Self::Int16 => map(nom::character::complete::i16, |n| n as usize).parse(input),
            Self::Int32 => map(nom::character::complete::i32, |n| n as usize).parse(input),
            Self::Uint8 => map(nom::character::complete::u8, |n| n as usize).parse(input),
            Self::Uint16 => map(nom::character::complete::u16, |n| n as usize).parse(input),
            Self::Uint32 => map(nom::character::complete::u32, |n| n as usize).parse(input),
            Self::F32 => map(nom::number::complete::float, |n| n as usize).parse(input),
            Self::F64 => map(nom::number::complete::double, |n| n as usize).parse(input),
        }
    }

    fn parse_ascii_float<'a, 'b>(&'a self, input: &'b str) -> IResult<&'b str, f32> {
        match self {
            Self::Int8 => map(nom::character::complete::i8, |n| n as f32).parse(input),
            Self::Int16 => map(nom::character::complete::i16, |n| n as f32).parse(input),
            Self::Int32 => map(nom::character::complete::i32, |n| n as f32).parse(input),
            Self::Uint8 => map(nom::character::complete::u8, |n| n as f32).parse(input),
            Self::Uint16 => map(nom::character::complete::u16, |n| n as f32).parse(input),
            Self::Uint32 => map(nom::character::complete::u32, |n| n as f32).parse(input),
            Self::F32 => nom::number::complete::float(input),
            Self::F64 => map(nom::number::complete::double, |n| n as f32).parse(input),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum VertexPropertyName {
    X,     // x
    Y,     // y
    Z,     // z
    Nx,    // nx
    Ny,    // ny
    Nz,    // nz
    Red,   // red
    Green, // green
    Blue,  // blue
    Alpha, // alpha
    S,     // s
    T,     // t
}

impl VertexPropertyName {
    fn parse_header(input: &str) -> IResult<&str, Self> {
        let (input, s) = preceded(space0, not_line_ending).parse(input)?;
        alt((
            map(all_consuming(tag_no_case("x")), |_| Self::X),
            map(all_consuming(tag_no_case("y")), |_| Self::Y),
            map(all_consuming(tag_no_case("z")), |_| Self::Z),
            map(all_consuming(tag_no_case("nx")), |_| Self::Nx),
            map(all_consuming(tag_no_case("ny")), |_| Self::Ny),
            map(all_consuming(tag_no_case("nz")), |_| Self::Nz),
            map(all_consuming(tag_no_case("red")), |_| Self::Red),
            map(all_consuming(tag_no_case("green")), |_| Self::Green),
            map(all_consuming(tag_no_case("blue")), |_| Self::Blue),
            map(all_consuming(tag_no_case("alpha")), |_| Self::Alpha),
            map(all_consuming(tag_no_case("s")), |_| Self::S),
            map(all_consuming(tag_no_case("t")), |_| Self::T),
        ))
        .parse(s)
        .map(|(_, n)| (input, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_comment() {
        assert!(parse_comment("abc").is_err());
        assert_eq!(parse_comment("comment"), Ok(("", "")));
        assert_eq!(
            parse_comment("comment This is a comment!"),
            Ok(("", "This is a comment!"))
        );
    }

    #[test]
    fn parse_header_format() {
        assert!(Format::parse_header("abc").is_err());
        assert!(matches!(
            Format::parse_header("format ascii 1.0").unwrap().1,
            Format::Ascii("1.0")
        ));
        assert!(matches!(
            Format::parse_header("format binary_little_endian 1.0")
                .unwrap()
                .1,
            Format::BinaryLe("1.0")
        ));
        assert!(matches!(
            Format::parse_header("format binary_big_endian 1.0")
                .unwrap()
                .1,
            Format::BinaryBe("1.0")
        ));
    }

    const VERTEX_ELEMENT: &str = "element vertex 12
property float x
property float y
property float z
";
    const FACE_ELEMENT: &str = "element face 10
property list uchar int vertex_index";

    #[test]
    fn parse_header_raw_element() {
        assert!(RawElement::parse_header("abc").is_err());

        assert_eq!(
            RawElement::parse_header(VERTEX_ELEMENT).unwrap().1,
            RawElement {
                name: "vertex",
                num_elements: 12,
                properties: vec![
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::X
                    }),
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::Y
                    }),
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::Z
                    }),
                ]
            }
        );

        assert_eq!(
            RawElement::parse_header(FACE_ELEMENT).unwrap().1,
            RawElement {
                name: "face",
                num_elements: 10,
                properties: vec![Property::Face(FaceProperty {
                    num_type: NumericType::Uint8,
                    idx_type: NumericType::Int32
                })]
            }
        );
    }

    #[test]
    fn parse_header_vertex_element() {
        assert!(VertexElement::parse_header("abc").is_err());

        assert!(VertexElement::parse_header(FACE_ELEMENT).is_err());

        assert_eq!(
            VertexElement::parse_header(VERTEX_ELEMENT).unwrap().1,
            VertexElement {
                num_vertices: 12,
                properties: vec![
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::X
                    }),
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::Y
                    }),
                    Property::Vertex(VertexProperty {
                        value_type: NumericType::F32,
                        name: VertexPropertyName::Z
                    }),
                ]
            }
        );
    }

    #[test]
    fn parse_header_face_element() {
        assert!(FaceElement::parse_header("abc").is_err());

        assert!(FaceElement::parse_header(VERTEX_ELEMENT).is_err());

        assert_eq!(
            FaceElement::parse_header(FACE_ELEMENT).unwrap().1,
            FaceElement {
                num_faces: 10,
                properties: vec![Property::Face(FaceProperty {
                    num_type: NumericType::Uint8,
                    idx_type: NumericType::Int32
                })]
            }
        );
    }

    #[test]
    fn parse_header_property() {
        assert_eq!(
            Property::parse_header("property test").unwrap().1,
            Property::Unsupported(RawProperty {
                property_str: "test"
            }),
        );

        assert_eq!(
            Property::parse_header("property float x").unwrap().1,
            Property::Vertex(VertexProperty {
                value_type: NumericType::F32,
                name: VertexPropertyName::X
            })
        );

        assert_eq!(
            Property::parse_header("property list uchar int vertex_index")
                .unwrap()
                .1,
            Property::Face(FaceProperty {
                num_type: NumericType::Uint8,
                idx_type: NumericType::Int32
            })
        );
    }

    #[test]
    fn parse_header_vertex_property() {
        assert!(
            VertexProperty::from_raw_property(
                &RawProperty::parse_header("property list uchar int vertex_index")
                    .unwrap()
                    .1
            )
            .is_err()
        );

        assert_eq!(
            VertexProperty::from_raw_property(
                &RawProperty::parse_header("property float x").unwrap().1
            )
            .unwrap()
            .1,
            VertexProperty {
                value_type: NumericType::F32,
                name: VertexPropertyName::X
            }
        );

        assert_eq!(
            VertexProperty::from_raw_property(
                &RawProperty::parse_header("property float64 red").unwrap().1
            )
            .unwrap()
            .1,
            VertexProperty {
                value_type: NumericType::F64,
                name: VertexPropertyName::Red
            }
        );
    }

    #[test]
    fn parse_header_face_property() {
        assert!(
            FaceProperty::from_raw_property(
                &RawProperty::parse_header("property float x").unwrap().1
            )
            .is_err()
        );

        assert_eq!(
            FaceProperty::from_raw_property(
                &RawProperty::parse_header("property list uchar int vertex_index")
                    .unwrap()
                    .1
            )
            .unwrap()
            .1,
            FaceProperty {
                num_type: NumericType::Uint8,
                idx_type: NumericType::Int32
            }
        );
    }

    #[test]
    fn parse_header_vertex_property_name() {
        assert!(VertexPropertyName::parse_header("abc").is_err());
        assert!(matches!(
            VertexPropertyName::parse_header("x").unwrap().1,
            VertexPropertyName::X
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("y").unwrap().1,
            VertexPropertyName::Y
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("z").unwrap().1,
            VertexPropertyName::Z
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("nx").unwrap().1,
            VertexPropertyName::Nx
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("ny").unwrap().1,
            VertexPropertyName::Ny
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("nz").unwrap().1,
            VertexPropertyName::Nz
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("red").unwrap().1,
            VertexPropertyName::Red
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("blue").unwrap().1,
            VertexPropertyName::Blue
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("green").unwrap().1,
            VertexPropertyName::Green
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("alpha").unwrap().1,
            VertexPropertyName::Alpha
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("s").unwrap().1,
            VertexPropertyName::S
        ));
        assert!(matches!(
            VertexPropertyName::parse_header("t").unwrap().1,
            VertexPropertyName::T
        ));
    }

    #[test]
    fn parse_header_numeric_type() {
        assert!(NumericType::parse_header("abc").is_err());

        assert!(matches!(
            NumericType::parse_header("char").unwrap().1,
            NumericType::Int8
        ));
        assert!(matches!(
            NumericType::parse_header("int8").unwrap().1,
            NumericType::Int8
        ));

        assert!(matches!(
            NumericType::parse_header("short").unwrap().1,
            NumericType::Int16
        ));
        assert!(matches!(
            NumericType::parse_header("int16").unwrap().1,
            NumericType::Int16
        ));

        assert!(matches!(
            NumericType::parse_header("int").unwrap().1,
            NumericType::Int32
        ));
        assert!(matches!(
            NumericType::parse_header("int32").unwrap().1,
            NumericType::Int32
        ));

        assert!(matches!(
            NumericType::parse_header("uchar").unwrap().1,
            NumericType::Uint8
        ));
        assert!(matches!(
            NumericType::parse_header("uint8").unwrap().1,
            NumericType::Uint8
        ));

        assert!(matches!(
            NumericType::parse_header("ushort").unwrap().1,
            NumericType::Uint16
        ));
        assert!(matches!(
            NumericType::parse_header("uint16").unwrap().1,
            NumericType::Uint16
        ));

        assert!(matches!(
            NumericType::parse_header("uint").unwrap().1,
            NumericType::Uint32
        ));
        assert!(matches!(
            NumericType::parse_header("uint32").unwrap().1,
            NumericType::Uint32
        ));

        assert!(matches!(
            NumericType::parse_header("float").unwrap().1,
            NumericType::F32
        ));
        assert!(matches!(
            NumericType::parse_header("float32").unwrap().1,
            NumericType::F32
        ));

        assert!(matches!(
            NumericType::parse_header("double").unwrap().1,
            NumericType::F64
        ));
        assert!(matches!(
            NumericType::parse_header("float64").unwrap().1,
            NumericType::F64
        ));
    }
}
