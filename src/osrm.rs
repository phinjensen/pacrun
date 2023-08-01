use std::process::Command;
use std::str;
use std::{collections::HashMap, ops::Deref};

use geo::{EuclideanDistance, LineLocatePoint, Point};
use geo_types::{coord, point, Coord, LineString};
use geojson::GeoJson;
use itertools::Itertools;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Annotation {
    pub nodes: Vec<u64>,
}

#[derive(Deserialize)]
pub struct Leg {
    pub annotation: Annotation,
}

#[derive(Deserialize)]
pub struct Tracepoint {
    pub waypoint_index: usize,
    pub matchings_index: usize,
    pub name: String,
    pub location: (f64, f64),
}

#[derive(Deserialize)]
pub struct Match {
    confidence: f64,
    geometry: GeoJson,
    pub legs: Vec<Leg>,
}

#[derive(Deserialize)]
pub struct OsrmResponse {
    pub matchings: Vec<Match>,
    pub tracepoints: Vec<Option<Tracepoint>>,
}

pub struct OsrmApi {
    domain: String,
}

pub enum OsrmError {
    MapMatchingJsonParseError,
    MapMatchingServiceError,
}

impl OsrmApi {
    pub fn new(domain: &str) -> Self {
        Self {
            domain: String::from(domain),
        }
    }

    pub fn get_osrm_match_query(&self, points: Vec<Point>, timestamps: Vec<i64>) -> String {
        let result = format!(
            "{}/match/v1/foot/{}?geometries=geojson&timestamps={}&annotations=true",
            self.domain,
            points
                .iter()
                .map(|p| format!("{},{}", p.x(), p.y()))
                .intersperse(";".to_string())
                .collect::<String>(),
            timestamps
                .iter()
                .map(|ts| ts.to_string())
                .intersperse(";".to_string())
                .collect::<String>()
        );

        //eprintln!("{}", result);
        result
    }

    pub async fn get_matches(
        &self,
        points: Vec<Point>,
        timestamps: Vec<i64>,
    ) -> Result<OsrmResponse, OsrmError> {
        // reqwest doesn't like long urls
        let match_json = reqwest::get(self.get_osrm_match_query(points, timestamps))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        //let command_result = command.output().unwrap();.map_err(|_| OsrmError::MapMatchingServiceError)?;
        //let match_json = str::from_utf8(&command_result.stdout)
        //    .map_err(|_| OsrmError::MapMatchingJsonParseError)?;
        //println!("{}", match_json);
        return Ok(serde_json::from_str::<OsrmResponse>(&match_json).unwrap());
    }
}

pub type Segment = (u64, u64);
pub struct SegmentMatches(HashMap<Segment, Vec<Coord>>);

impl Deref for SegmentMatches {
    type Target = HashMap<Segment, Vec<Coord>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SegmentMatches {
    pub fn sort(&mut self, map_nodes: &HashMap<u64, Coord>) {
        for ((s, e), nodes) in self.0.iter_mut() {
            let line = LineString::new(vec![
                map_nodes.get(&s).unwrap().clone(),
                map_nodes.get(&e).unwrap().clone(),
            ]);
            nodes.sort_by(|a, b| {
                let a: Point = (*a).into();
                let b: Point = (*b).into();
                line.line_locate_point(&a)
                    .unwrap()
                    .partial_cmp(&line.line_locate_point(&b).unwrap())
                    .unwrap()
            });
        }
    }

    pub fn get_complete_segments(&mut self, map_nodes: &HashMap<u64, Coord>) -> Vec<Segment> {
        let mut result: Vec<Segment> = vec![];
        self.sort(map_nodes);
        for (segment, coords) in self.iter() {
            let mut valid = true;
            for (p1, p2) in coords
                .iter()
                .map(|c| Into::<Point>::into(*c))
                .tuple_windows()
            {
                if p1.euclidean_distance(&p2) > 10.0 {
                    valid = false;
                    break;
                }
            }
            if valid {
                result.push(*segment);
            }
        }
        result
    }
}

impl OsrmResponse {
    pub fn get_segment_matches(&self) -> SegmentMatches {
        let mut result: HashMap<(u64, u64), Vec<_>> = HashMap::new();
        for t in self.tracepoints.iter().filter_map(|x| x.as_ref()) {
            let Tracepoint {
                matchings_index,
                waypoint_index,
                location,
                ..
            } = *t;

            if let Some(leg) = self.matchings[matchings_index].legs.get(waypoint_index) {
                leg.annotation
                    .nodes
                    .as_slice()
                    .windows(2)
                    .for_each(|window| {
                        let k = (window[0], window[1]);
                        let point = coord! { x: location.0, y: location.1 };
                        if result.contains_key(&k) {
                            result.get_mut(&k).unwrap().push(point);
                        } else {
                            let locations = vec![point];
                            result.insert(k, locations);
                        }
                    });
            }
        }
        SegmentMatches(result)
    }
}
