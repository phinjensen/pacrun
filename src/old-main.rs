use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader},
};

use geo::{GeodesicDistance, GeodesicLength, Line, LineInterpolatePoint};
use geo_types::{coord, Geometry, LineString, Point, Polygon};
use geojson::{Feature, FeatureCollection, Value};
use osm_xml::{Node, Reference, Way, OSM};
use rstar::RTree;
use time::OffsetDateTime;

pub fn node_to_geo(node: Node) -> Result<Geometry, String> {
    Ok(Point::new(node.lon, node.lat).into())
}

pub fn way_to_geo(osm: &OSM, way: &Way) -> Result<Geometry, String> {
    // A way should also be longer than one node; otherwise it's not a string or
    // polygon
    if way.nodes.len() <= 1 {
        return Err("Invalid data, way too short".to_string());
    }

    // All items in a Way should be
    let points: Vec<_> = way
        .nodes
        .iter()
        .map(|ref_| {
            if let Reference::Node(node) = osm.resolve_reference(ref_) {
                Ok(coord! {
                    x: node.lon,
                    y: node.lat,
                })
            } else {
                Err("Grr".to_string())
            }
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Default to a LineString, but wrap it in a Polygon if it fits the requirements
    let line = LineString::new(points);
    if way.is_polygon() {
        Ok(Polygon::new(line, vec![]).into())
    } else {
        Ok(line.into())
    }
}

fn main() -> io::Result<()> {
    let street_file = File::open("provo.xml")?;
    let reader = BufReader::new(street_file);
    let osm = OSM::parse(reader).unwrap();
    let streets: Vec<_> = osm
        .ways
        .iter()
        .filter_map(|(_, way)| {
            if let Ok(geo_types::Geometry::LineString(street)) = way_to_geo(&osm, way) {
                Some((way.id, street))
            } else {
                None
            }
        })
        .collect();

    let run_file = File::open("run.gpx")?;
    let reader = BufReader::new(run_file);
    let gpx = gpx::read(reader).unwrap();
    //let gpx_tree = RTree::bulk_load(
    //    gpx.tracks
    //        .iter()
    //        .flat_map(|track| track.segments.iter().flat_map(|segment| &segment.points))
    //        .map(|p| p.point())
    //        .collect::<Vec<_>>(),
    //);
    for track in &gpx.tracks {
        for segment in &track.segments {
            for point in &segment.points {
                eprint!(
                    "{};",
                    OffsetDateTime::parse(
                        &point.time.unwrap().format().unwrap(),
                        &time::format_description::well_known::Iso8601::DEFAULT
                    )
                    .unwrap()
                    .unix_timestamp()
                );
            }
        }
    }
    for track in gpx.tracks {
        for segment in track.segments {
            for point in segment.points {
                print!("{},{};", point.point().x(), point.point().y());
            }
        }
    }

    //let mut matches = HashMap::new();
    //for (id, street) in &streets {
    //    matches.insert(id, Vec::new());
    //    for line in street.lines() {
    //        let length = line.geodesic_length();
    //        let mut start = None;
    //        let mut end = None;
    //        for meters in (0..length as u64).step_by(3) {
    //            let point = line.line_interpolate_point(meters as f64 / length).unwrap();
    //            let nearest_neighbor = gpx_tree.nearest_neighbor(&point).unwrap();
    //            if point.geodesic_distance(nearest_neighbor) < 15.0 {
    //                if start.is_none() {
    //                    start = Some(point);
    //                }
    //                end = Some(point);
    //            } else if start.is_some() {
    //                let v = matches.get_mut(id).unwrap();
    //                v.push(start.unwrap());
    //                v.push(end.unwrap());
    //                start = None;
    //            }
    //        }
    //        if start.is_some() {
    //            let v = matches.get_mut(id).unwrap();
    //            v.push(start.unwrap());
    //            v.push(end.unwrap());
    //        }
    //    }
    //}

    //let collection = FeatureCollection {
    //    bbox: None,
    //    features: matches
    //        .iter()
    //        .filter(|(_, lines)| lines.len() > 0)
    //        .map(|(_, lines)| Feature {
    //            geometry: Some(geojson::Geometry {
    //                bbox: None,
    //                value: Value::from(&geo_types::LineString::from(lines.clone())),
    //                foreign_members: None,
    //            }),
    //            bbox: None,
    //            id: None,
    //            properties: None,
    //            foreign_members: None,
    //        })
    //        .collect::<Vec<_>>(),
    //    foreign_members: None,
    //};
    //println!("{}", collection.to_string());

    // This algorithm is
    //  O(g log g + s*l*log g) where
    //      g: num of points in GPX track
    //      s: num of streets in OSM area
    //      l: length of longest street in OSM area
    // Put all points in gpx into rtree O(g log g)
    // for every street (in OSM) O(s)
    //   iterate over 3 m points for each line segment
    //   start = first point
    //   end = first point
    //   for (0..len(line)).step_by(3) O(len(longest s))
    //      tree.nearest_neighbor(point) O(log n)
    //      if distance to nearest neighbor < 3m:
    //          end = point
    //      else:
    //          add start, end to matches
    //          start = point
    //          end = point

    Ok(())
}
