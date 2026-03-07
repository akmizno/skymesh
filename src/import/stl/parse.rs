pub(super) mod ascii {
    use nom::{
        IResult, Parser,
        bytes::complete::tag_no_case,
        character::complete::{multispace1, not_line_ending, space1},
        combinator::{map, opt, peek},
        multi::many0_count,
        number::complete::float,
        sequence::preceded,
    };

    use super::super::mesh::{Face, StlMeshBuilder, Vertex};
    use crate::model::Vec3;

    pub(crate) fn parse(
        mut input: &str,
        mut builder: StlMeshBuilder,
    ) -> IResult<&str, StlMeshBuilder> {
        let (i, _header) = parse_header(input)?;
        input = i;

        while peek(preceded(skip_ignored, parse_endsolid))
            .parse(input)
            .is_err()
        {
            let (i, face) = preceded(skip_ignored, parse_face).parse(input)?;
            input = i;

            builder.add_face(face);
        }

        Ok((input, builder))
    }

    // consume spaces, tabs, newlines
    fn skip_ignored(input: &str) -> IResult<&str, ()> {
        map(many0_count(multispace1), |_| ()).parse(input)
    }

    struct Header<'a> {
        _name: &'a str,
    }

    fn parse_header(input: &str) -> IResult<&str, Header<'_>> {
        let (input, _) = tag_no_case("solid").parse(input)?;
        let (input, name) = preceded(space1, not_line_ending).parse(input)?;

        Ok((input, Header { _name: name }))
    }

    fn parse_endsolid(input: &str) -> IResult<&str, ()> {
        let (input, _) = tag_no_case("endsolid").parse(input)?;
        let (input, _) = opt(preceded(space1, not_line_ending)).parse(input)?;
        Ok((input, ()))
    }

    fn parse_vector(input: &str) -> IResult<&str, Vec3> {
        let (input, x) = float(input)?;
        let (input, y) = preceded(space1, float).parse(input)?;
        let (input, z) = preceded(space1, float).parse(input)?;

        Ok((input, Vec3::new(x, y, z)))
    }

    // vertex
    fn parse_vertex(input: &str) -> IResult<&str, Vertex> {
        let (input, _) = tag_no_case("vertex").parse(input)?;
        let (input, v) = preceded(space1, parse_vector).parse(input)?;
        Ok((input, Vertex::new(v)))
    }

    // face
    fn parse_face(input: &str) -> IResult<&str, Face> {
        let (input, _) = tag_no_case("facet").parse(input)?;
        let (input, _) = preceded(space1, tag_no_case("normal")).parse(input)?;
        let (input, normal) = preceded(space1, parse_vector).parse(input)?;
        let (input, _) = preceded(skip_ignored, tag_no_case("outer")).parse(input)?;
        let (input, _) = preceded(space1, tag_no_case("loop")).parse(input)?;
        let (input, vert0) = preceded(skip_ignored, parse_vertex).parse(input)?;
        let (input, vert1) = preceded(skip_ignored, parse_vertex).parse(input)?;
        let (input, vert2) = preceded(skip_ignored, parse_vertex).parse(input)?;
        let (input, _) = preceded(skip_ignored, tag_no_case("endloop")).parse(input)?;
        let (input, _) = preceded(skip_ignored, tag_no_case("endfacet")).parse(input)?;

        Ok((input, Face::new(normal, [vert0, vert1, vert2])))
    }
}

pub(super) mod binary {
    use nom::{
        IResult,
        bytes::complete::take,
        number::complete::{le_f32, le_u32},
    };

    use super::super::mesh::{Face, StlMeshBuilder, Vertex};
    use crate::model::Vec3;

    pub(crate) fn parse(
        mut input: &[u8],
        mut builder: StlMeshBuilder,
    ) -> IResult<&[u8], StlMeshBuilder> {
        let (i, header) = parse_header(input)?;
        input = i;

        builder.set_num_faces(header.num_faces as usize);

        for _ in 0..header.num_faces {
            let (i, face) = parse_face(input)?;
            input = i;

            builder.add_face(face);
        }

        Ok((input, builder))
    }

    struct Header<'a> {
        _text: &'a [u8],
        num_faces: u32,
    }

    fn parse_header(input: &[u8]) -> IResult<&[u8], Header<'_>> {
        let (input, text) = take(80usize)(input)?;
        let (input, num_faces) = le_u32(input)?;

        Ok((
            input,
            Header {
                _text: text,
                num_faces,
            },
        ))
    }

    fn parse_vector(input: &[u8]) -> IResult<&[u8], Vec3> {
        let (input, x) = le_f32(input)?;
        let (input, y) = le_f32(input)?;
        let (input, z) = le_f32(input)?;

        Ok((input, Vec3::new(x, y, z)))
    }

    // face
    fn parse_face(input: &[u8]) -> IResult<&[u8], Face> {
        let (input, normal) = parse_vector(input)?;
        let (input, v0) = parse_vector(input)?;
        let (input, v1) = parse_vector(input)?;
        let (input, v2) = parse_vector(input)?;
        let (input, _bits) = take(2usize)(input)?;

        Ok((
            input,
            Face::new(normal, [Vertex::new(v0), Vertex::new(v1), Vertex::new(v2)]),
        ))
    }
}
