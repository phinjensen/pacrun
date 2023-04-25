use std::{
    fs::File,
    io::{self, BufReader},
};

use geo_types::{coord, Coord, Geometry, LineString, Point};
use osm_xml::{Node, Reference, Way, OSM};

pub fn node_to_geo(osm: OSM, node: Node) -> Result<Geometry, String> {
    Ok(Point::new(node.lon, node.lat).into())
}

pub fn way_to_geo(osm: &OSM, way: &Way) -> Result<Geometry, String> {
    // A way should also be longer than one node; otherwise it's not a string or
    // polygon
    if way.nodes.len() <= 1 {
        return Err("Invalid data, way too short".to_string());
    }

    // All items in a Way should be
    let points: Result<Vec<_>, String> = way
        .nodes
        .iter()
        .map(|ref_| {
            println!("{:?}", osm.resolve_reference(ref_));
            if let Reference::Node(node) = osm.resolve_reference(ref_) {
                Ok(coord! {
                    x: node.lon,
                    y: node.lat,
                })
            } else {
                Err("Grr".to_string())
            }
        })
        .collect();
    println!("{:?}", points);

    // Default to a LineString, but wrap it in a Polygon if it fits the requirements
    let line = LineString::new(points);
    if let Some(tags) = &meta.tags {
        if nodes.first().unwrap().0 == nodes.last().unwrap().0 && is_polygon_feature(tags) {
            return Ok(Polygon::new(line, vec![]).into());
        }
    }
    Ok(line.into())
}

//pub fn to_geo(osm: OSM) -> Result<Geometry, Error> {
//        Element::Way { nodes, meta } => {
//        }
//        Element::Relation { members, meta } => {
//            if members.is_empty() {
//                Err(Error::InvalidData)
//            } else if let Some(tags) = &meta.tags {
//                let type_tag = match tags.get("type") {
//                    Some(t) => t.as_str(),
//                    _ => "",
//                };
//                // Convert a relation into a MultiPolygon if it has the tag type=multipolygon or type=boundary
//                if type_tag == "multipolygon" || type_tag == "boundary" {
//                    // A multipolygon relation should have at least one outer member
//                    let outer_members = members
//                        .iter()
//                        .filter(|member| member.role == "outer")
//                        .collect::<Vec<_>>();
//                    if outer_members.is_empty() {
//                        return Err(Error::InvalidData);
//                    }
//
//                    // Each outer member should be a way
//                    let outer_ways = outer_members
//                        .iter()
//                        .filter_map(|member| element_map.get(&member._ref.0))
//                        .map(|e| e.as_ref())
//                        .collect::<Vec<_>>();
//
//                    // Each outer way should be a closed ring
//                    let mut polygons = join_ways(outer_ways, element_map)?
//                        .into_iter()
//                        .map(|ring| Polygon::new(ring, vec![]))
//                        .collect::<Vec<_>>();
//
//                    let inner_ways = members
//                        .iter()
//                        .filter(|member| member.role == "inner")
//                        .filter_map(|member| element_map.get(&member._ref.0))
//                        .map(|e| e.as_ref())
//                        .collect::<Vec<_>>();
//
//                    // Each outer way should be a closed ring
//                    let inner_rings = join_ways(inner_ways, element_map)?;
//                    if inner_rings.iter().any(|ring| ring.is_closed()) {
//                        return Err(Error::InvalidData);
//                    }
//
//                    for polygon in &mut polygons {
//                        for ring in &inner_rings {
//                            if polygons_intersect_ls(polygon.exterior(), ring) {
//                                polygon.interiors_push(ring.clone())
//                            }
//                        }
//                    }
//                    Ok(MultiPolygon::new(polygons).into())
//                } else {
//                    Err(Error::InvalidData)
//                }
//            } else {
//                Err(Error::InvalidData)
//            }
//        }
//    }
//}

fn main() -> io::Result<()> {
    let f = File::open("provo2.xml")?;
    let reader = BufReader::new(f);
    let d = OSM::parse(reader).unwrap();

    println!("{:?}", way_to_geo(&d, d.ways.get(&10187190).unwrap()));

    //if let Some(tags) = &first_element.tags {
    //    for (key, value) in tags.iter() {
    //        println!("{}: {}", key, value);
    //    }
    //}
    //println!("GEO: {:?}", first_element.to_geo(&d.element_map()).unwrap());
    Ok(())
}
