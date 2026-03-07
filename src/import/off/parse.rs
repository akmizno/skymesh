use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{char, multispace1, not_line_ending, space1, u8, usize},
    combinator::{map, opt},
    multi::many0_count,
    number::complete::float,
    sequence::preceded,
};

use super::mesh::{Face, Idx3, Idx4, IdxN, OffMeshBuilder, VertIdx, Vertex};
use crate::model::{Color, Vec3};

pub(super) fn parse(mut input: &str, mut builder: OffMeshBuilder) -> IResult<&str, OffMeshBuilder> {
    let (i, header) = parse_header(input)?;
    input = i;

    builder.set_num_vertices(header.num_vertices);
    builder.set_num_faces(header.num_faces);

    for _ in 0..header.num_vertices {
        let (i, vertex) = preceded(skip_ignored, parse_vertex).parse(input)?;
        input = i;

        builder.add_vertex(vertex);
    }

    for _ in 0..header.num_faces {
        let (i, face) = preceded(skip_ignored, parse_face).parse(input)?;
        input = i;

        builder.add_face(face);
    }

    Ok((input, builder))
}

// # comment
fn parse_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = char('#')(input)?;
    not_line_ending(input)
}

// consume spaces, tabs, newlines, and comments.
fn skip_ignored(input: &str) -> IResult<&str, ()> {
    map(many0_count(alt((multispace1, parse_comment))), |_| ()).parse(input)
}

struct Header {
    num_vertices: usize,
    num_faces: usize,
    _num_edges: Option<usize>,
}

// OFF
// numVertices numFaces numEdges
fn parse_header(input: &str) -> IResult<&str, Header> {
    let (input, _) = skip_ignored(input)?;
    let (input, _) = alt((tag_no_case("OFF"), tag_no_case("COFF"))).parse(input)?;

    let (input, num_vertices) = preceded(skip_ignored, usize).parse(input)?;
    let (input, num_faces) = preceded(space1, usize).parse(input)?;
    let (input, num_edges) = opt(preceded(space1, usize)).parse(input)?;

    Ok((
        input,
        Header {
            num_vertices,
            num_faces,
            _num_edges: num_edges,
        },
    ))
}

fn parse_color_u8(input: &str) -> IResult<&str, Color> {
    let (input, r) = u8(input)?;
    let (input, g) = preceded(space1, u8).parse(input)?;
    let (input, b) = preceded(space1, u8).parse(input)?;
    let (input, a) = opt(preceded(space1, u8)).parse(input)?;

    Ok((input, Color::from_rgba8(r, g, b, a)))
}

fn parse_color_float(input: &str) -> IResult<&str, Color> {
    let (input, r) = float(input)?;
    let (input, g) = preceded(space1, float).parse(input)?;
    let (input, b) = preceded(space1, float).parse(input)?;
    let (input, a) = opt(preceded(space1, float)).parse(input)?;

    Ok((input, Color::from_rgba(r, g, b, a)))
}

// r g b [a]
fn parse_color(input: &str) -> IResult<&str, Color> {
    alt((parse_color_u8, parse_color_float)).parse(input)
}

// x y z
fn parse_position(input: &str) -> IResult<&str, Vec3> {
    let (input, x) = float.parse(input)?;
    let (input, y) = preceded(space1, float).parse(input)?;
    let (input, z) = preceded(space1, float).parse(input)?;

    Ok((input, Vec3::new(x, y, z)))
}

// vertex
fn parse_vertex(input: &str) -> IResult<&str, Vertex> {
    let (input, position) = parse_position(input)?;
    let (input, color) = opt(preceded(space1, parse_color)).parse(input)?;

    Ok((input, Vertex::new(position, color)))
}

fn parse_idx3(input: &str) -> IResult<&str, Idx3> {
    let (input, _) = char('3')(input)?;

    let (input, idx0) = preceded(space1, usize).parse(input)?;
    let (input, idx1) = preceded(space1, usize).parse(input)?;
    let (input, idx2) = preceded(space1, usize).parse(input)?;

    Ok((input, Idx3::new([idx0, idx1, idx2])))
}

fn parse_idx4(input: &str) -> IResult<&str, Idx4> {
    let (input, _) = char('4')(input)?;

    let (input, idx0) = preceded(space1, usize).parse(input)?;
    let (input, idx1) = preceded(space1, usize).parse(input)?;
    let (input, idx2) = preceded(space1, usize).parse(input)?;
    let (input, idx3) = preceded(space1, usize).parse(input)?;

    Ok((input, Idx4::new([idx0, idx1, idx2, idx3])))
}

fn parse_idxn(mut input: &str) -> IResult<&str, IdxN> {
    let (i, n) = usize(input)?;
    input = i;

    debug_assert!(4 < n);

    let mut v = Vec::with_capacity(n);

    for _ in 0..n {
        let (i, idx) = preceded(space1, usize).parse(input)?;
        input = i;

        v.push(idx);
    }

    Ok((input, IdxN::new(v)))
}

// n idx0 idx1 idx2 ...
fn parse_vidx(input: &str) -> IResult<&str, VertIdx> {
    let idx3_parser = map(parse_idx3, VertIdx::Idx3);
    let idx4_parser = map(parse_idx4, VertIdx::Idx4);
    let idxn_parser = map(parse_idxn, VertIdx::IdxN);

    alt((idx3_parser, idx4_parser, idxn_parser)).parse(input)
}

// face
fn parse_face(input: &str) -> IResult<&str, Face> {
    let (input, vidx) = parse_vidx(input)?;
    let (input, color) = opt(preceded(space1, parse_color)).parse(input)?;

    Ok((input, Face::new(vidx, color)))
}
