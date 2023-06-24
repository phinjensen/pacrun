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

//struct WayMatch {
//    id: u64,
//    start_point: (f64, f64),
//    end_point: (f64, f64),
//}
//
//fn get_way_matches(data: OsrmResponse) -> Vec<WayMatch> {
//    let mut way_matches: Vec<WayMatch> = Vec::new();
//    for tracepoint in data.tracepoints {
//        if tracepoint.is_none() {
//            continue;
//        }
//        let tracepoint = tracepoint.unwrap();
//        let id: u64 = tracepoint.name.parse().unwrap();
//        let last_match = way_matches.last();
//        if last_match.is_none() || last_match.unwrap().id != id {
//            way_matches.push(WayMatch {
//                id,
//                start_point: tracepoint.location,
//                end_point: tracepoint.location,
//            });
//        } else {
//            let mut last = way_matches.last_mut().unwrap();
//            last.end_point = tracepoint.location;
//        }
//    }
//    way_matches
//}
