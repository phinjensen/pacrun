use std::collections::HashMap;
use std::process::Command;
use std::str;

use geo::Point;
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
        format!(
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
        )
    }

    pub async fn get_matches(
        &self,
        points: Vec<Point>,
        timestamps: Vec<i64>,
    ) -> Result<OsrmResponse, OsrmError> {
        // reqwest doesn't like long urls
        let mut command = Command::new("curl");
        command.arg(self.get_osrm_match_query(points, timestamps));
        let command_result = command
            .output()
            .map_err(|_| OsrmError::MapMatchingServiceError)?;
        let match_json = str::from_utf8(&command_result.stdout)
            .map_err(|_| OsrmError::MapMatchingJsonParseError)?;
        //println!("{}", match_json);
        return Ok(serde_json::from_str::<OsrmResponse>(&match_json).unwrap());
    }
}

impl OsrmResponse {
    pub fn get_segment_matches(&self) -> HashMap<(u64, u64), Vec<(f64, f64)>> {
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
                        if result.contains_key(&k) {
                            result.get_mut(&k).unwrap().push(location);
                        } else {
                            let locations = Vec::new();
                            result.insert(k, locations);
                        }
                    });
            }
        }
        result
    }
}
